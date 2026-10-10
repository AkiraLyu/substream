#pragma once

#include <QJsonObject>
#include <QObject>
#include <QProcess>
#include <QTimer>

// Owns a cancellable Substream command that emits one JSON object per line.
class JsonProcess final : public QObject {
    Q_OBJECT
public:
    explicit JsonProcess(QObject* parent = nullptr);
    ~JsonProcess() override;
    void start(const QString& program, const QStringList& arguments);
    void stop();
    void fail(const QString& message);
    bool active() const { return m_active; }
    bool stopping() const { return m_stopping; }
    QString error() const { return m_error; }

signals:
    void changed();
    void eventReceived(const QJsonObject& event);
    void diagnostic(const QString& text);
    void finished(bool success, bool cancelled);

private:
    void readOutput();
    QProcess m_process;
    QTimer m_stopDeadline;
    QByteArray m_buffer;
    QString m_error;
    QString m_stderr;
    bool m_active = false;
    bool m_stopping = false;
};
