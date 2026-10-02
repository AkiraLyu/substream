#include "caption_model.h"

CaptionModel::CaptionModel(int holdMs, QObject* parent)
    : QObject(parent)
    , m_holdMs(holdMs)
{
    m_expiry.setSingleShot(true);
    connect(&m_expiry, &QTimer::timeout, this, &CaptionModel::clear);
}

void CaptionModel::clear()
{
    m_expiry.stop();
    m_text.clear();
    emit changed();
}

void CaptionModel::reset()
{
    clear();
    m_session = -1;
    m_segment = -1;
    m_revision = -1;
    m_final = false;
}

bool CaptionModel::applyCaption(const QJsonObject& caption, qint64 ageMs, QString* error)
{
    const auto segment = caption.value("segment_id").toInteger(-1);
    const auto revision = caption.value("revision").toInteger(-1);
    if (segment < 0 || revision < 1 || ageMs < 0 || !caption.value("stable_text").isString()
        || !caption.value("unstable_text").isString() || !caption.value("is_final").isBool()) {
        *error = QStringLiteral("Invalid caption event");
        return false;
    }
    if (segment < m_segment || (segment == m_segment && (revision <= m_revision || m_final))) {
        return true;
    }
    m_segment = segment;
    m_revision = revision;
    m_final = caption.value("is_final").toBool();
    if (ageMs >= m_holdMs) {
        clear();
        return true;
    }
    m_text = caption.value("stable_text").toString() + caption.value("unstable_text").toString();
    m_expiry.start(m_holdMs - static_cast<int>(ageMs));
    emit changed();
    return true;
}

bool CaptionModel::apply(const QJsonObject& event, QString* error)
{
    const auto type = event.value("type").toString();
    if (type == "display") {
        if (event.value("version").toInt() != 1 || event.value("session_id").toInteger(-1) < 0) {
            *error = QStringLiteral("Unsupported display protocol");
            return false;
        }
        const auto session = event.value("session_id").toInteger();
        if (m_session != session) {
            reset();
            m_session = session;
        }
        const auto status = event.value("status").toString();
        if (status == "idle" || status == "loading" || status == "error") {
            reset();
            m_session = session;
            return true;
        }
        if (status != "listening" && status != "finished") {
            *error = QStringLiteral("Unknown display status");
            return false;
        }
        if (event.value("caption").isNull()) {
            clear();
            return true;
        }
        return applyCaption(
            event.value("caption").toObject(), event.value("caption_age_ms").toInteger(-1), error);
    }
    if (type == "ready") {
        if (event.value("version").toInt() != 1) {
            *error = QStringLiteral("Unsupported stream protocol");
            return false;
        }
        reset();
        return true;
    }
    if (type == "caption") {
        return applyCaption(event.value("caption").toObject(), 0, error);
    }
    if (type == "finished") {
        return true; // Let the last caption expire normally.
    }
    if (type == "error") {
        reset();
        *error = event.value("message").toString(QStringLiteral("Caption source failed"));
        return false;
    }
    *error = QStringLiteral("Unknown caption event");
    return false;
}
