#include "video_controller.h"

#include <QFileInfo>
#include <QUrl>

VideoController::VideoController(QObject* parent)
    : QObject(parent)
{
    connect(&m_process, &JsonProcess::eventReceived, this, &VideoController::consumeEvent);
    connect(&m_process, &JsonProcess::changed, this, &VideoController::changed);
    connect(&m_process, &JsonProcess::diagnostic, this, &VideoController::diagnostic);
    connect(&m_process, &JsonProcess::finished, this, [this](bool success, bool cancelled) {
        if (!m_process.error().isEmpty())
            fail(m_process.error());
        else if (!(m_stage == "completed" && success) && m_stage != "failed"
            && m_stage != "cancelled") {
            if (cancelled)
                m_stage = "cancelled";
            else
                fail(tr("The video process did not return a completed result."));
        }
        emit changed();
    });
}

void VideoController::start(const VideoOptions& options)
{
    if (active())
        return;
    m_error.clear();
    m_result = { };
    m_stage.clear();
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
    m_stage = "queued";
    emit changed();
    m_process.start(options.program, args);
}

void VideoController::consumeEvent(const QJsonObject& event)
{
    if (m_stage == "failed")
        return;
    const auto stage = event.value("stage").toString();
    static const QStringList stages { "queued", "checking_subtitles", "downloading",
        "importing_subtitles", "converting", "transcribing", "cancelling", "completed", "cancelled",
        "failed" };
    if (!stages.contains(stage)) {
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

void VideoController::fail(const QString& message)
{
    m_error = message;
    m_stage = "failed";
    m_result = { };
    if (active())
        m_process.fail(message);
    emit changed();
}

QString VideoController::status() const
{
    if ((stopping() && active()) || m_stage == "cancelling")
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
