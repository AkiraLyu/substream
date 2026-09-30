#pragma once

#include <QJsonObject>
#include <QObject>
#include <QTimer>

class CaptionModel final : public QObject {
    Q_OBJECT

public:
    explicit CaptionModel(int holdMs, QObject* parent = nullptr);

    QString text() const { return m_text; }
    bool synthetic() const { return m_synthetic; }
    bool visible() const { return !m_text.isEmpty(); }
    int remainingMs() const { return qMax(0, m_expiry.remainingTime()); }

    // Accepts /v1/display snapshots or the stream command's JSON events.
    bool apply(const QJsonObject& event, QString* error);
    void clear();
    void reset();

signals:
    void changed();

private:
    bool applyCaption(const QJsonObject& caption, qint64 ageMs, QString* error);

    int m_holdMs;
    QTimer m_expiry;
    QString m_text;
    bool m_synthetic = false;
    qint64 m_session = -1;
    qint64 m_segment = -1;
    qint64 m_revision = -1;
    bool m_final = false;
};
