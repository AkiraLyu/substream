#include "session_controller.h"
#include "event_source.h"
#include "token_file.h"

#include <QDir>
#include <QFileInfo>
#include <QRegularExpression>
#include <QtEndian>

#include <csignal>

SessionController::SessionController(QObject* parent)
    : QObject(parent)
{
    m_deadline.setSingleShot(true);
    m_captureDeadline.setSingleShot(true);
    m_captureDeadline.setInterval(1500);
    m_progress.setInterval(100);
    connect(&m_deadline, &QTimer::timeout, this, [this] {
        abort(m_stopping ? tr("Stopping timed out. The speech process was terminated.")
                         : tr("Startup timed out. Check the model and audio device."));
    });
    connect(&m_captureDeadline, &QTimer::timeout, &m_capture, &QProcess::kill);
    connect(&m_progress, &QTimer::timeout, this, [this] {
        if (m_options.input == LaunchOptions::Input::Device && m_ready && !m_stopping
            && m_lastAudio.isValid() && m_lastAudio.elapsed() > 5000) {
            abort(tr("The audio device is not sending data. Check its connection and try again."));
            return;
        }
        emit changed();
        m_level = 0;
    });
    connect(&m_capture, &QProcess::readyReadStandardOutput, this, &SessionController::forwardAudio);
    const auto readErrors = [this](QProcess& process) {
        const auto text = QString::fromUtf8(process.readAllStandardError());
        m_stderr = (m_stderr + text).right(8192);
        emit diagnostic(text.trimmed());
    };
    connect(&m_engine, &QProcess::readyReadStandardError, this,
        [this, readErrors] { readErrors(m_engine); });
    connect(&m_capture, &QProcess::readyReadStandardError, this,
        [this, readErrors] { readErrors(m_capture); });
    connect(&m_engine, &QProcess::readyReadStandardOutput, this, [this] {
        const auto data = m_engine.readAllStandardOutput();
        if (m_options.input == LaunchOptions::Input::Device && m_events)
            m_events->feed(data);
    });
    connect(&m_engine, &QProcess::started, this, [this] {
        if (m_stopping) {
            m_engine.terminate();
            return;
        }
        if (m_options.input != LaunchOptions::Input::Browser || m_creatingToken)
            return;
        QString error;
        const auto token = readToken(m_options.tokenFile, &error);
        if (!token) {
            abort(error);
            return;
        }
        m_events->startSocket(QUrl(QStringLiteral("ws://127.0.0.1:9743/v1/display")), *token);
    });
    connect(&m_capture, &QProcess::started, this, [this] {
        if (m_stopping) {
            m_capture.terminate();
            return;
        }
        m_status = tr("Waiting for audio…");
        m_lastAudio.start();
        emit changed();
    });
    connect(&m_engine, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart)
            abort(tr("Cannot start the speech process: %1").arg(m_engine.errorString()));
        else if (!m_stopping && error != QProcess::Crashed)
            abort(tr("Cannot communicate with the speech process: %1").arg(m_engine.errorString()));
    });
    connect(&m_capture, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart)
            abort(tr("Cannot start audio capture. Check that pw-cat is installed."));
    });
    connect(&m_capture, &QProcess::finished, this, [this] {
        m_captureDeadline.stop();
        forwardAudio();
        m_engine.closeWriteChannel();
        if (!m_stopping)
            abort(tr("Audio capture stopped unexpectedly. Check the device.\n%1")
                    .arg(m_stderr.trimmed()));
        completeIfStopped();
    });
    connect(&m_engine, &QProcess::finished, this, [this](int code, QProcess::ExitStatus exit) {
        if (m_creatingToken && !m_stopping && code == 0 && exit == QProcess::NormalExit) {
            m_creatingToken = false;
            launchEngine();
            return;
        }
        if (m_options.input == LaunchOptions::Input::Device && m_events) {
            m_events->feed(m_engine.readAllStandardOutput());
            m_events->finishInput();
        }
        if (!m_stopping
            || (m_error.isEmpty() && m_options.input == LaunchOptions::Input::Device && m_ready
                && (!m_finished || code != 0 || exit != QProcess::NormalExit))) {
            abort(tr("The speech process did not finish normally.\n%1").arg(m_stderr.trimmed()));
        }
        if (m_events)
            m_events->stop();
        completeIfStopped();
    });
}

SessionController::~SessionController()
{
    m_active = false;
    // Window close normally waits asynchronously. This also covers application shutdown.
    disconnect(&m_engine, nullptr, this, nullptr);
    disconnect(&m_capture, nullptr, this, nullptr);
    if (m_events)
        m_events->stop();
    m_capture.kill();
    m_engine.kill();
    m_capture.waitForFinished(1000);
    m_engine.waitForFinished(1000);
}

QString SessionController::deviceSerial() const
{
    return m_active && m_options.input == LaunchOptions::Input::Device ? m_options.device.serial
                                                                       : QString();
}

QStringList SessionController::recognizerArguments() const
{
    QStringList args;
    if (!m_options.config.isEmpty())
        args << QStringLiteral("--config") << m_options.config;
    if (!m_options.config.isEmpty() && m_options.threads > 0)
        args << QStringLiteral("--threads") << QString::number(m_options.threads);
    return args;
}

void SessionController::start(const LaunchOptions& options)
{
    if (m_active)
        return;
    m_options = options;
    m_error.clear();
    m_stderr.clear();
    m_backend = { };
    m_audioBytes = 0;
    m_level = 0;
    m_ready = false;
    m_finished = false;
    m_stopping = false;
    m_creatingToken = false;
    m_source.clear();
    m_lastAudio.invalidate();
    const auto reject = [this](const QString& message) {
        m_error = message;
        m_status = tr("Cannot start");
        emit changed();
    };
    const bool videoOnly = options.input == LaunchOptions::Input::Browser
        && options.config.isEmpty() && !options.videoConfig.isEmpty();
    if (options.program.isEmpty() || (!videoOnly && !QFileInfo(options.config).isFile())) {
        reject(tr("Choose the Substream executable and a valid model configuration."));
        return;
    }
    if (options.input == LaunchOptions::Input::Device && options.device.serial.isEmpty()) {
        reject(tr("Choose an audio device to capture."));
        return;
    }
    if (options.input == LaunchOptions::Input::Browser) {
        if (!options.videoConfig.isEmpty() && !QFileInfo(options.videoConfig).isFile()) {
            reject(tr("Choose a valid video configuration."));
            return;
        }
        static const QRegularExpression origin(QStringLiteral("^chrome-extension://[a-p]{32}$"));
        if (!origin.match(options.browserOrigin).hasMatch()) {
            reject(tr("Enter the 32-character Chromium extension ID."));
            return;
        }
        if (!QDir().mkpath(QFileInfo(options.tokenFile).absolutePath())) {
            reject(tr("Cannot create the token directory."));
            return;
        }
        if (QFileInfo::exists(options.tokenFile)) {
            QString error;
            if (!readToken(options.tokenFile, &error)) {
                reject(error);
                return;
            }
        }
    }
    m_events = std::make_unique<EventSource>();
    connect(m_events.get(), &EventSource::eventReceived, this, &SessionController::consumeEvent);
    connect(m_events.get(), &EventSource::failed, this, &SessionController::abort);
    connect(m_events.get(), &EventSource::disconnected, this, [this] {
        if (!m_active || m_stopping)
            return;
        m_backend = { };
        m_source.clear();
        m_status = tr("Connecting to the local service…");
        emit eventReceived({ { "type", "ready" }, { "version", 1 } });
        if (!m_deadline.isActive())
            m_deadline.start(10000);
        emit changed();
    });
    m_active = true;
    m_progress.start();
    m_deadline.start(30000);
    m_status = options.input == LaunchOptions::Input::Device
        ? tr("Loading the model…")
        : tr("Starting the browser caption service…");
    emit changed();
    if (options.input == LaunchOptions::Input::Browser && !QFileInfo::exists(options.tokenFile)) {
        m_creatingToken = true;
        m_engine.start(options.program, { "token", "--output", options.tokenFile });
    } else {
        launchEngine();
    }
}

void SessionController::launchEngine()
{
    auto args = recognizerArguments();
    if (m_options.input == LaunchOptions::Input::Device) {
        args.prepend(QStringLiteral("stream"));
    } else {
        args.prepend(QStringLiteral("serve"));
        args << "--token-file" << m_options.tokenFile << "--allow-origin"
             << m_options.browserOrigin;
        if (!m_options.videoConfig.isEmpty())
            args << "--video-config" << m_options.videoConfig;
        if (!m_options.summaryConfig.isEmpty())
            args << "--summary-config" << m_options.summaryConfig;
    }
    m_engine.start(m_options.program, args);
}

void SessionController::consumeEvent(const QJsonObject& event)
{
    if (!m_active || !m_error.isEmpty())
        return;
    const auto type = event.value("type").toString();
    if (type == "error") {
        abort(event.value("message").toString());
        return;
    }
    if (type == "ready" && !m_stopping) {
        if (m_ready || event.value("version").toInt() != 1) {
            abort(tr("The speech process returned an invalid ready message."));
            return;
        }
        m_ready = true;
        m_backend = event.value("backend").toObject();
        m_deadline.stop();
        m_source = m_options.device.label + "\n" + m_options.device.name;
        QStringList args { "--record", "--raw", "--rate", "16000", "--channels", "1", "--format",
            "s16", "--latency", "20ms", "--target", m_options.device.serial, "--properties",
            m_options.device.sink ? "{ stream.capture.sink=true node.dont-reconnect=true }"
                                  : "{ node.dont-reconnect=true }",
            "-" };
        m_capture.start(QStringLiteral("pw-cat"), args);
    } else if (type == "finished") {
        m_finished = true;
    } else if (type == "display") {
        m_deadline.stop();
        const auto state = event.value("status").toString();
        m_backend = event.value("backend").toObject();
        m_source = event.value("source").toString();
        m_audioBytes
            = static_cast<quint64>(qMax<qint64>(0, event.value("samples_received").toInteger()))
            * 2;
        if (!m_stopping) {
            if (state == "loading")
                m_status = tr("Browser connected. Loading the model…");
            else if (state == "listening")
                m_status = m_audioBytes > 0 ? tr("Capturing tab audio")
                                            : tr("Model ready. Waiting for tab audio…");
            else if (state == "finished")
                m_status = tr("Tab capture finished. Ready to start again.");
            else if (state == "error") {
                m_status = tr("Browser recognition failed. Try again in the extension.");
                emit diagnostic(event.value("message").toString());
            } else
                m_status = tr("Service started. Waiting for the browser.");
            if (state == "idle" || state == "finished" || state == "error") {
                m_backend = { };
                m_source.clear();
            }
        }
    }
    emit eventReceived(event);
    emit changed();
}

void SessionController::forwardAudio()
{
    const auto bytes = m_capture.readAllStandardOutput();
    if (bytes.isEmpty() || !m_error.isEmpty())
        return;
    // Stop on overload instead of accumulating delayed audio in the GUI process.
    constexpr qint64 MaxQueuedBytes = 16000 * 2 / 4;
    if (m_engine.bytesToWrite() + bytes.size() > MaxQueuedBytes) {
        abort(tr("Capture stopped because recognition is too slow. Reduce system load or choose a "
                 "smaller model."));
        return;
    }
    if (m_engine.write(bytes) != bytes.size()) {
        abort(tr("Cannot send audio to the speech process."));
        return;
    }
    m_audioBytes += bytes.size();
    for (qsizetype i = 0; i + 1 < bytes.size(); i += 2) {
        const auto sample = qFromLittleEndian<qint16>(bytes.constData() + i);
        m_level = qMax(m_level, qAbs(static_cast<int>(sample)) * 100 / 32768);
    }
    m_lastAudio.restart();
    if (!m_stopping)
        m_status = tr("Capturing audio");
}

void SessionController::stop()
{
    if (!m_active || m_stopping)
        return;
    m_stopping = true;
    m_status = tr("Stopping and saving final captions…");
    if (m_options.input == LaunchOptions::Input::Device && m_ready) {
        if (m_capture.state() != QProcess::NotRunning) {
            m_capture.terminate();
            m_captureDeadline.start();
        } else {
            m_engine.closeWriteChannel();
        }
        m_deadline.start(30000);
    } else {
        if (m_events)
            m_events->stop();
        if (m_engine.state() == QProcess::Running)
            ::kill(static_cast<pid_t>(m_engine.processId()), SIGINT);
        m_deadline.start(2000);
    }
    emit changed();
    completeIfStopped();
}

void SessionController::inputUnavailable()
{
    if (!deviceSerial().isEmpty() && !m_stopping)
        abort(tr("The capture device disconnected. Choose an audio source again."));
}

void SessionController::abort(const QString& message)
{
    if (m_error.isEmpty())
        m_error = message;
    m_stopping = true;
    m_status = tr("Stopping processes…");
    m_deadline.stop();
    if (m_events)
        m_events->stop();
    m_capture.kill();
    m_engine.kill();
    emit changed();
    completeIfStopped();
}

void SessionController::completeIfStopped()
{
    if (!m_active || !m_stopping || m_capture.state() != QProcess::NotRunning
        || m_engine.state() != QProcess::NotRunning)
        return;
    m_active = false;
    m_stopping = false;
    m_level = 0;
    m_progress.stop();
    m_deadline.stop();
    m_captureDeadline.stop();
    m_status = m_error.isEmpty() ? tr("Stopped") : tr("Startup or capture failed");
    emit changed();
}
