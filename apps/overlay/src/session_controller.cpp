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
        abort(m_stopping ? QStringLiteral("停止超时，识别进程已结束。")
                         : QStringLiteral("启动超时，请检查模型和音频设备。"));
    });
    connect(&m_captureDeadline, &QTimer::timeout, &m_capture, &QProcess::kill);
    connect(&m_progress, &QTimer::timeout, this, [this] {
        if (m_options.input == LaunchOptions::Input::Device && m_ready && !m_stopping
            && m_lastAudio.isValid() && m_lastAudio.elapsed() > 5000) {
            abort(QStringLiteral("音频设备未提供数据，请检查连接后重试。"));
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
        m_status = QStringLiteral("等待音频数据…");
        m_lastAudio.start();
        emit changed();
    });
    connect(&m_engine, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart)
            abort(QStringLiteral("无法启动识别程序：") + m_engine.errorString());
        else if (!m_stopping && error != QProcess::Crashed)
            abort(QStringLiteral("识别进程通信失败：") + m_engine.errorString());
    });
    connect(&m_capture, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart)
            abort(QStringLiteral("无法启动音频采集，请确认已安装 pw-cat。"));
    });
    connect(&m_capture, &QProcess::finished, this, [this] {
        m_captureDeadline.stop();
        forwardAudio();
        m_engine.closeWriteChannel();
        if (!m_stopping)
            abort(QStringLiteral("音频采集意外结束，请检查设备。\n") + m_stderr.trimmed());
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
            abort(QStringLiteral("识别进程未正常完成。\n") + m_stderr.trimmed());
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
        m_status = QStringLiteral("无法启动");
        emit changed();
    };
    const bool videoOnly = options.input == LaunchOptions::Input::Browser
        && options.config.isEmpty() && !options.videoConfig.isEmpty();
    if (options.program.isEmpty() || (!videoOnly && !QFileInfo(options.config).isFile())) {
        reject(QStringLiteral("请选择识别程序和有效的模型配置文件。"));
        return;
    }
    if (options.input == LaunchOptions::Input::Device && options.device.serial.isEmpty()) {
        reject(QStringLiteral("请选择要捕获的音频设备。"));
        return;
    }
    if (options.input == LaunchOptions::Input::Browser) {
        if (!options.videoConfig.isEmpty() && !QFileInfo(options.videoConfig).isFile()) {
            reject(QStringLiteral("请选择有效的视频任务配置文件。"));
            return;
        }
        static const QRegularExpression origin(QStringLiteral("^chrome-extension://[a-p]{32}$"));
        if (!origin.match(options.browserOrigin).hasMatch()) {
            reject(QStringLiteral("请输入 Chromium 扩展的 32 位 ID。"));
            return;
        }
        if (!QDir().mkpath(QFileInfo(options.tokenFile).absolutePath())) {
            reject(QStringLiteral("无法创建令牌所在目录。"));
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
        m_status = QStringLiteral("正在连接本地服务…");
        emit eventReceived({ { "type", "ready" }, { "version", 1 } });
        if (!m_deadline.isActive())
            m_deadline.start(10000);
        emit changed();
    });
    m_active = true;
    m_progress.start();
    m_deadline.start(30000);
    m_status = options.input == LaunchOptions::Input::Device
        ? QStringLiteral("正在加载模型…")
        : QStringLiteral("正在启动浏览器字幕服务…");
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
            abort(QStringLiteral("识别程序返回了无效的就绪消息。"));
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
                m_status = QStringLiteral("浏览器已连接，正在加载模型…");
            else if (state == "listening")
                m_status = m_audioBytes > 0 ? QStringLiteral("正在捕获标签页音频")
                                            : QStringLiteral("模型已就绪，等待标签页音频…");
            else if (state == "finished")
                m_status = QStringLiteral("标签页捕获已结束，等待下一次开始");
            else if (state == "error") {
                m_status = QStringLiteral("浏览器识别失败，可在扩展中重试");
                emit diagnostic(event.value("message").toString());
            } else
                m_status = QStringLiteral("服务已启动，等待浏览器连接");
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
        abort(QStringLiteral("识别速度跟不上音频，已停止采集。请减少负载或选择较小的模型。"));
        return;
    }
    if (m_engine.write(bytes) != bytes.size()) {
        abort(QStringLiteral("无法向识别程序发送音频。"));
        return;
    }
    m_audioBytes += bytes.size();
    for (qsizetype i = 0; i + 1 < bytes.size(); i += 2) {
        const auto sample = qFromLittleEndian<qint16>(bytes.constData() + i);
        m_level = qMax(m_level, qAbs(static_cast<int>(sample)) * 100 / 32768);
    }
    m_lastAudio.restart();
    if (!m_stopping)
        m_status = QStringLiteral("正在捕获音频");
}

void SessionController::stop()
{
    if (!m_active || m_stopping)
        return;
    m_stopping = true;
    m_status = QStringLiteral("正在停止，保存末尾字幕…");
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
        abort(QStringLiteral("捕获设备已断开，请重新选择音频源。"));
}

void SessionController::abort(const QString& message)
{
    if (m_error.isEmpty())
        m_error = message;
    m_stopping = true;
    m_status = QStringLiteral("正在结束进程…");
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
    m_status = m_error.isEmpty() ? QStringLiteral("已停止") : QStringLiteral("启动或运行失败");
    emit changed();
}
