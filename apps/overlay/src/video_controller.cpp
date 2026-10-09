#include "video_controller.h"

#include <QFileInfo>
#include <QJsonDocument>
#include <QUrl>

VideoController::VideoController(QObject* parent)
    : QObject(parent)
{
    m_stopDeadline.setSingleShot(true);
    m_stopDeadline.setInterval(5000);
    connect(&m_stopDeadline, &QTimer::timeout, this, [this] {
        fail(tr("Cancellation timed out. The video process was terminated."));
        m_process.kill();
    });
    connect(&m_process, &QProcess::readyReadStandardOutput, this, &VideoController::readOutput);
    connect(&m_process, &QProcess::readyReadStandardError, this, [this] {
        const auto text = QString::fromUtf8(m_process.readAllStandardError());
        m_stderr = (m_stderr + text).right(8192);
        emit diagnostic(text.trimmed());
    });
    connect(&m_process, &QProcess::started, this, [this] {
        if (m_stopping)
            m_process.terminate();
    });
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) {
            m_stopDeadline.stop();
            m_active = false;
            fail(tr("Cannot start Substream: %1").arg(m_process.errorString()));
        }
    });
    connect(&m_process, &QProcess::finished, this, [this](int code, QProcess::ExitStatus exit) {
        m_stopDeadline.stop();
        readOutput();
        m_active = false;
        if (m_stage == "completed" && (code != 0 || exit != QProcess::NormalExit))
            fail(tr("The video process did not finish normally.\n%1").arg(m_stderr.trimmed()));
        else if (m_stage != "completed" && m_stage != "cancelled" && m_stage != "failed") {
            if (m_stopping)
                m_stage = "cancelled";
            else
                fail(tr("The video process did not finish normally.\n%1").arg(m_stderr.trimmed()));
        }
        m_stopping = false;
        emit changed();
    });
}

VideoController::~VideoController()
{
    disconnect(&m_process, nullptr, this, nullptr);
    if (m_process.state() != QProcess::NotRunning) {
        m_process.terminate();
        if (!m_process.waitForFinished(5000)) {
            m_process.kill();
            m_process.waitForFinished(1000);
        }
    }
}

void VideoController::start(const VideoOptions& options)
{
    if (m_active)
        return;
    m_error.clear();
    m_stderr.clear();
    m_buffer.clear();
    m_result = { };
    m_stage.clear();
    m_stopping = false;
    const QUrl url(options.url, QUrl::StrictMode);
    if (!url.isValid() || url.host().isEmpty() || !url.userInfo().isEmpty()
        || (url.scheme() != "http" && url.scheme() != "https")) {
        fail(tr("Enter a valid HTTP or HTTPS video link."));
        return;
    }
    if (options.program.isEmpty() || !QFileInfo(options.config).isFile()) {
        fail(tr("Choose the Substream executable and a video configuration."));
        return;
    }
    QStringList args { "video", options.url, "--config", options.config };
    switch (options.cookies) {
    case VideoOptions::Cookies::Configuration:
        break;
    case VideoOptions::Cookies::None:
        args << "--no-cookies";
        break;
    case VideoOptions::Cookies::Browser:
        if (options.cookieSource.isEmpty()) {
            fail(tr("Choose a browser for cookies."));
            return;
        }
        args << "--cookies-from-browser" << options.cookieSource;
        break;
    case VideoOptions::Cookies::File:
        if (!QFileInfo(options.cookieSource).isFile()) {
            fail(tr("Choose an existing cookies file."));
            return;
        }
        args << "--cookies-file" << options.cookieSource;
        break;
    }
    m_active = true;
    m_stage = "queued";
    emit changed();
    m_process.start(options.program, args);
}

void VideoController::stop()
{
    if (!m_active || m_stopping)
        return;
    m_stopping = true;
    m_stopDeadline.start();
    if (m_process.state() == QProcess::Running)
        m_process.terminate();
    emit changed();
}

void VideoController::readOutput()
{
    const auto data = m_process.readAllStandardOutput();
    if (m_stage == "failed")
        return;
    m_buffer += data;
    constexpr qsizetype MaxMessage = 128 * 1024;
    while (true) {
        const auto end = m_buffer.indexOf('\n');
        if (end < 0)
            break;
        if (end > MaxMessage) {
            fail(tr("The video process returned an invalid status message."));
            return;
        }
        QJsonParseError error;
        const auto document = QJsonDocument::fromJson(m_buffer.left(end), &error);
        m_buffer.remove(0, end + 1);
        const auto event = document.object();
        const auto stage = event.value("stage").toString();
        static const QStringList stages { "queued", "checking_subtitles", "downloading",
            "importing_subtitles", "converting", "transcribing", "cancelling", "completed",
            "cancelled", "failed" };
        if (error.error != QJsonParseError::NoError || !stages.contains(stage)) {
            fail(tr("The video process returned an invalid status message."));
            return;
        }
        m_stage = stage;
        m_result = event.value("result").toObject();
        m_error = event.value("error").toString();
        if (stage == "completed"
            && (m_result.value("directory").toString().isEmpty()
                || m_result.value("document").toString().isEmpty())) {
            fail(tr("The video process returned an invalid status message."));
            return;
        }
        emit changed();
    }
    if (m_buffer.size() > MaxMessage)
        fail(tr("The video process returned an invalid status message."));
}

void VideoController::fail(const QString& message)
{
    m_error = message;
    m_stage = "failed";
    m_result = { };
    if (m_active)
        stop();
    emit changed();
}

QString VideoController::status() const
{
    if ((m_stopping && m_active) || m_stage == "cancelling")
        return tr("Cancelling…");
    if (m_stage == "queued")
        return tr("Starting…");
    if (m_stage == "checking_subtitles")
        return tr("Checking available subtitles…");
    if (m_stage == "downloading")
        return tr("Downloading video…");
    if (m_stage == "importing_subtitles")
        return tr("Using existing subtitles…");
    if (m_stage == "converting")
        return tr("Converting audio…");
    if (m_stage == "transcribing")
        return tr("Recognizing speech…");
    if (m_stage == "cancelled")
        return tr("Cancelled");
    if (m_stage == "failed")
        return tr("Video task failed");
    if (m_stage == "completed") {
        const auto source = m_result.value("subtitle_source").toString();
        if (source == "provided")
            return tr("Completed · Existing subtitles");
        if (source == "automatic")
            return tr("Completed · Platform auto-captions");
        return tr("Completed · Speech recognition");
    }
    return tr("Not started");
}
