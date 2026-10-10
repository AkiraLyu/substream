#pragma once

#include "json_process.h"

struct VideoOptions {
    QString program;
    QString config;
    QString url;
    enum class Cookies { Configuration, None, Browser, File };
    Cookies cookies = Cookies::Configuration;
    QString cookieSource;
};

// Runs the same NDJSON video command used by CLI clients. No downloader logic lives here.
class VideoController final : public QObject {
    Q_OBJECT
public:
    explicit VideoController(QObject* parent = nullptr);
    void start(const VideoOptions& options);
    void stop() { m_process.stop(); }
    bool active() const { return m_process.active(); }
    bool stopping() const { return m_process.stopping(); }
    QString status() const;
    QString error() const { return m_error; }
    QJsonObject result() const { return m_result; }

signals:
    void changed();
    void diagnostic(const QString& text);

private:
    void consumeEvent(const QJsonObject& event);
    void fail(const QString& message);
    JsonProcess m_process;
    QString m_stage;
    QString m_error;
    QJsonObject m_result;
};
