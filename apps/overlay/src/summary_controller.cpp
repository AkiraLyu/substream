#include "summary_controller.h"

#include <QFileInfo>

SummaryController::SummaryController(QObject* parent)
    : QObject(parent)
{
    connect(&m_process, &JsonProcess::changed, this, &SummaryController::changed);
    connect(&m_process, &JsonProcess::eventReceived, this, [this](const QJsonObject& event) {
        if (!m_pending.isEmpty() || event.value("schema_version").toInt() != 1
            || event.value("markdown").toString().trimmed().isEmpty()) {
            m_process.fail(tr("Substream returned an invalid summary."));
            return;
        }
        m_pending = event;
    });
    connect(&m_process, &JsonProcess::finished, this, [this](bool success, bool cancelled) {
        m_cancelled = cancelled;
        m_error = m_process.error();
        if (success && !m_pending.isEmpty()) {
            m_result = m_pending;
            m_cancelled = false;
        } else if (m_error.isEmpty() && !cancelled) {
            m_error = tr("Substream did not return a summary.");
        }
        m_pending = { };
        emit changed();
    });
}

void SummaryController::start(
    const QString& program, const QString& configuration, const QString& document)
{
    if (active())
        return;
    m_error.clear();
    m_result = { };
    m_pending = { };
    m_cancelled = false;
    if (program.isEmpty() || !QFileInfo(document).isFile() || !QFileInfo(configuration).isFile()) {
        m_error = tr("Choose Substream, a subtitle document, and a summary configuration.");
        emit changed();
        return;
    }
    m_process.start(program, { "summarize", document, "--config", configuration });
}

QString SummaryController::status() const
{
    if (stopping())
        return tr("Cancelling…");
    if (active())
        return tr("Generating summary…");
    if (!m_error.isEmpty())
        return tr("Summary failed");
    if (m_cancelled)
        return tr("Cancelled");
    if (!m_result.isEmpty())
        return tr("Completed");
    return tr("Not started");
}
