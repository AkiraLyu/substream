#pragma once

#include "json_process.h"

class SummaryController final : public QObject {
    Q_OBJECT
public:
    explicit SummaryController(QObject* parent = nullptr);
    void start(const QString& program, const QString& configuration, const QString& document);
    void stop() { m_process.stop(); }
    bool active() const { return m_process.active(); }
    bool stopping() const { return m_process.stopping(); }
    QString status() const;
    QString error() const { return m_error; }
    QString markdown() const { return m_result.value("markdown").toString(); }

signals:
    void changed();

private:
    JsonProcess m_process;
    QJsonObject m_pending;
    QJsonObject m_result;
    QString m_error;
    bool m_cancelled = false;
};
