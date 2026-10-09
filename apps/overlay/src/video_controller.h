#pragma once

#include <QJsonObject>
#include <QObject>
#include <QProcess>
#include <QTimer>

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
    ~VideoController() override;
    void start(const VideoOptions& options);
    void stop();
    bool active() const { return m_active; }
    bool stopping() const { return m_stopping; }
    QString status() const;
    QString error() const { return m_error; }
    QJsonObject result() const { return m_result; }

signals:
    void changed();
    void diagnostic(const QString& text);

private:
    void readOutput();
    void fail(const QString& message);
    QProcess m_process;
    QTimer m_stopDeadline;
    QByteArray m_buffer;
    QString m_stage;
    QString m_error;
    QString m_stderr;
    QJsonObject m_result;
    bool m_active = false;
    bool m_stopping = false;
};
