#include "json_process.h"

#include <QJsonDocument>

JsonProcess::JsonProcess(QObject* parent)
    : QObject(parent)
{
    m_stopDeadline.setSingleShot(true);
    m_stopDeadline.setInterval(5000);
    connect(&m_stopDeadline, &QTimer::timeout, this, [this] {
        m_error = tr("Cancellation timed out. Substream was terminated.");
        m_process.kill();
    });
    connect(&m_process, &QProcess::readyReadStandardOutput, this, &JsonProcess::readOutput);
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
            m_stopping = false;
            m_error = tr("Cannot start Substream: %1").arg(m_process.errorString());
            emit finished(false, false);
            emit changed();
        }
    });
    connect(&m_process, &QProcess::finished, this, [this](int code, QProcess::ExitStatus exit) {
        readOutput();
        if (m_error.isEmpty() && !m_stopping && !m_buffer.trimmed().isEmpty())
            m_error = tr("Substream returned an incomplete JSON message.");
        const bool cancelled = m_stopping;
        const bool success = code == 0 && exit == QProcess::NormalExit && m_error.isEmpty();
        if (!success && m_error.isEmpty() && !cancelled)
            m_error = tr("Substream did not finish normally.\n%1").arg(m_stderr.trimmed());
        m_stopDeadline.stop();
        m_active = false;
        m_stopping = false;
        emit finished(success, cancelled);
        emit changed();
    });
}

JsonProcess::~JsonProcess()
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

void JsonProcess::start(const QString& program, const QStringList& arguments)
{
    if (m_active)
        return;
    m_buffer.clear();
    m_error.clear();
    m_stderr.clear();
    m_stopping = false;
    m_active = true;
    emit changed();
    m_process.start(program, arguments);
}

void JsonProcess::stop()
{
    if (!m_active || m_stopping)
        return;
    m_stopping = true;
    m_stopDeadline.start();
    if (m_process.state() == QProcess::Running)
        m_process.terminate();
    emit changed();
}

void JsonProcess::fail(const QString& message)
{
    if (m_error.isEmpty())
        m_error = message;
    stop();
}

void JsonProcess::readOutput()
{
    const auto bytes = m_process.readAllStandardOutput();
    if (!m_error.isEmpty())
        return;
    m_buffer += bytes;
    constexpr qsizetype MaxMessage = 4 * 1024 * 1024;
    while (m_error.isEmpty()) {
        const auto end = m_buffer.indexOf('\n');
        if (end < 0)
            break;
        if (end > MaxMessage) {
            fail(tr("Substream returned a JSON message larger than 4 MiB."));
            return;
        }
        QJsonParseError error;
        const auto document = QJsonDocument::fromJson(m_buffer.left(end), &error);
        m_buffer.remove(0, end + 1);
        if (error.error != QJsonParseError::NoError || !document.isObject()) {
            fail(tr("Substream returned an invalid JSON message."));
            return;
        }
        emit eventReceived(document.object());
    }
    if (m_buffer.size() > MaxMessage)
        fail(tr("Substream returned a JSON message larger than 4 MiB."));
}
