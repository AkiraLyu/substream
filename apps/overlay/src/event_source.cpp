#include "event_source.h"

#include <QJsonDocument>
#include <QJsonParseError>
#include <QNetworkProxy>

#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <unistd.h>
#include <utility>

namespace {
constexpr qsizetype MaxEventBytes = 64 * 1024;
}

EventSource::EventSource(QObject* parent)
    : QObject(parent)
{
    m_socket.setProxy(QNetworkProxy::NoProxy);
    m_socket.setMaxAllowedIncomingMessageSize(MaxEventBytes);
    m_reconnect.setSingleShot(true);
    m_reconnect.setInterval(1500);
    m_handshake.setSingleShot(true);
    m_handshake.setInterval(8000);
    connect(&m_handshake, &QTimer::timeout, &m_socket, &QWebSocket::abort);
}

EventSource::~EventSource()
{
    if (m_stdinFlags >= 0) {
        fcntl(STDIN_FILENO, F_SETFL, m_stdinFlags);
    }
}

bool EventSource::decode(const QByteArray& data)
{
    QJsonParseError error;
    const auto document = QJsonDocument::fromJson(data, &error);
    if (error.error != QJsonParseError::NoError || !document.isObject()) {
        m_fatal = true;
        emit failed(QStringLiteral("Invalid JSON caption message"));
        return false;
    }
    emit eventReceived(document.object());
    return true;
}

bool EventSource::startStdin(QString* error)
{
    if (isatty(STDIN_FILENO)) {
        *error = QStringLiteral("Pipe caption events into --stdin or redirect an event file");
        return false;
    }
    m_stdinFlags = fcntl(STDIN_FILENO, F_GETFL);
    if (m_stdinFlags < 0 || fcntl(STDIN_FILENO, F_SETFL, m_stdinFlags | O_NONBLOCK) < 0) {
        *error = QString::fromLocal8Bit(std::strerror(errno));
        return false;
    }
    m_stdin = std::make_unique<QSocketNotifier>(STDIN_FILENO, QSocketNotifier::Read, this);
    connect(m_stdin.get(), &QSocketNotifier::activated, this, &EventSource::readStdin);
    return true;
}

void EventSource::readStdin()
{
    char bytes[4096];
    // Yield between batches so a fast producer cannot starve rendering.
    for (int batch = 0; batch < 16; ++batch) {
        const auto count = ::read(STDIN_FILENO, bytes, sizeof(bytes));
        if (count < 0) {
            if (errno == EINTR)
                continue;
            if (errno == EAGAIN || errno == EWOULDBLOCK)
                return;
            m_stdin->setEnabled(false);
            emit failed(QString::fromLocal8Bit(std::strerror(errno)));
            return;
        }
        if (count == 0) {
            m_stdin->setEnabled(false);
            if (finishInput())
                emit ended();
            return;
        }
        if (!feed(QByteArray(bytes, count))) {
            m_stdin->setEnabled(false);
            return;
        }
    }
}

void EventSource::startSocket(const QUrl& url, const QString& token)
{
    connect(&m_socket, &QWebSocket::connected, this, [this, token] {
        const QJsonObject authentication { { "type", "authenticate" }, { "version", 1 },
            { "token", token } };
        m_socket.sendTextMessage(
            QString::fromUtf8(QJsonDocument(authentication).toJson(QJsonDocument::Compact)));
    });
    connect(&m_socket, &QWebSocket::textMessageReceived, this, [this](const QString& text) {
        m_handshake.stop();
        if (!decode(text.toUtf8()))
            m_socket.close();
    });
    connect(&m_socket, &QWebSocket::disconnected, this, [this] {
        m_handshake.stop();
        emit disconnected();
        if (!m_fatal)
            m_reconnect.start();
    });
    connect(&m_socket, &QWebSocket::errorOccurred, this, [this](QAbstractSocket::SocketError) {
        emit disconnected();
        if (!m_fatal)
            m_reconnect.start();
    });
    connect(&m_reconnect, &QTimer::timeout, this, [this, url] {
        m_socket.open(url);
        m_handshake.start();
    });
    m_socket.open(url);
    m_handshake.start();
}

void EventSource::stop()
{
    m_fatal = true;
    m_reconnect.stop();
    m_handshake.stop();
    if (m_stdin)
        m_stdin->setEnabled(false);
    m_socket.abort();
}

bool EventSource::feed(const QByteArray& data)
{
    if (m_fatal)
        return false;
    m_buffer += data;
    qsizetype end;
    while ((end = m_buffer.indexOf('\n')) >= 0) {
        const auto line = m_buffer.first(end);
        m_buffer.remove(0, end + 1);
        if (end > MaxEventBytes) {
            m_fatal = true;
            emit failed(QStringLiteral("Caption message is too large"));
            return false;
        }
        if (!line.trimmed().isEmpty() && !decode(line))
            return false;
    }
    if (m_buffer.size() > MaxEventBytes) {
        m_fatal = true;
        emit failed(QStringLiteral("Caption message is too large"));
        return false;
    }
    return true;
}

bool EventSource::finishInput()
{
    const auto tail = std::exchange(m_buffer, { });
    return !m_fatal && (tail.trimmed().isEmpty() || decode(tail));
}
