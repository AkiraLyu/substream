#pragma once

#include "audio_devices.h"

#include <QElapsedTimer>
#include <QJsonObject>
#include <QObject>
#include <QProcess>
#include <QTimer>
#include <memory>

class EventSource;

struct LaunchOptions {
    enum class Input { Device, Browser };
    Input input = Input::Device;
    QString program;
    QString config;
    int threads = 0; // Zero uses the model configuration.
    AudioDevice device;
    QString tokenFile;
    QString browserOrigin;
    QString videoConfig;
};

// Owns capture and recognition processes; the window only edits options and renders state.
class SessionController final : public QObject {
    Q_OBJECT
public:
    explicit SessionController(QObject* parent = nullptr);
    ~SessionController() override;
    void start(const LaunchOptions& options);
    void stop();
    void inputUnavailable();
    bool active() const { return m_active; }
    bool stopping() const { return m_stopping; }
    QString status() const { return m_status; }
    QString error() const { return m_error; }
    QString source() const { return m_source; }
    QJsonObject backend() const { return m_backend; }
    quint64 samples() const { return m_audioBytes / 2; }
    int level() const { return m_level; }
    QString deviceSerial() const;

signals:
    void changed();
    void eventReceived(const QJsonObject& event);
    void diagnostic(const QString& text);

private:
    void launchEngine();
    void consumeEvent(const QJsonObject& event);
    void forwardAudio();
    void abort(const QString& message);
    void completeIfStopped();
    QStringList recognizerArguments() const;

    LaunchOptions m_options;
    QProcess m_engine;
    QProcess m_capture;
    std::unique_ptr<EventSource> m_events;
    QTimer m_deadline;
    QTimer m_captureDeadline;
    QTimer m_progress;
    QElapsedTimer m_lastAudio;
    QString m_status = QStringLiteral("尚未启动");
    QString m_error;
    QString m_source;
    QString m_stderr;
    QJsonObject m_backend;
    quint64 m_audioBytes = 0;
    int m_level = 0;
    bool m_active = false;
    bool m_stopping = false;
    bool m_creatingToken = false;
    bool m_ready = false;
    bool m_finished = false;
};
