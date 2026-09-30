#pragma once

#include <QJsonObject>
#include <QObject>
#include <QSocketNotifier>
#include <QTimer>
#include <QWebSocket>

#include <memory>

class EventSource final : public QObject {
    Q_OBJECT

public:
    explicit EventSource(QObject* parent = nullptr);
    ~EventSource() override;

    bool startStdin(QString* error);
    void startSocket(const QUrl& url, const QString& token);

signals:
    void eventReceived(const QJsonObject& event);
    void disconnected();
    void ended();
    void failed(const QString& message);

private:
    bool decode(const QByteArray& data);
    void readStdin();

    QWebSocket m_socket;
    QTimer m_reconnect;
    QTimer m_handshake;
    std::unique_ptr<QSocketNotifier> m_stdin;
    QByteArray m_buffer;
    int m_stdinFlags = -1;
    bool m_fatal = false;
};
