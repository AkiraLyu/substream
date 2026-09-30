#include "caption_model.h"
#include "event_source.h"
#include "kde_window.h"

#include <QCommandLineParser>
#include <QFile>
#include <QFileInfo>
#include <QGuiApplication>
#include <QHostAddress>
#include <QRegularExpression>
#include <QScreen>
#include <QTextStream>

#include <optional>

namespace {
std::optional<QString> readToken(const QString& path)
{
    const QFileInfo info(path);
    constexpr auto sharedPermissions = QFile::ReadGroup | QFile::WriteGroup | QFile::ExeGroup
        | QFile::ReadOther | QFile::WriteOther | QFile::ExeOther;
    if (!info.isFile() || info.isSymLink() || (info.permissions() & sharedPermissions)
        || info.size() > 128) {
        qCritical("Token must be a private regular file (mode 0600)");
        return std::nullopt;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        qCritical("Cannot read token file");
        return std::nullopt;
    }
    const auto token = QString::fromUtf8(file.read(129)).trimmed();
    static const QRegularExpression pattern(QStringLiteral("^[a-fA-F0-9]{64}$"));
    if (!pattern.match(token).hasMatch()) {
        qCritical("Token must contain 64 hexadecimal characters");
        return std::nullopt;
    }
    return token;
}
}

int main(int argc, char** argv)
{
    qputenv("QT_FORCE_STDERR_LOGGING", "1");
    QGuiApplication app(argc, argv);
    QCoreApplication::setApplicationName(QStringLiteral("substream-overlay"));
    QCoreApplication::setApplicationVersion(QStringLiteral("0.1.0"));
    QGuiApplication::setDesktopFileName(QStringLiteral("substream-overlay"));

    QCommandLineParser parser;
    parser.setApplicationDescription(QStringLiteral("KDE Wayland caption overlay"));
    parser.addHelpOption();
    parser.addVersionOption();
    parser.addOptions({
        { QStringLiteral("stdin"), "Read newline-delimited caption events from standard input." },
        { QStringLiteral("token-file"),
            "Subscribe to the local subtitle service using a private token file.", "path" },
        { QStringLiteral("server"), "Display WebSocket URL (loopback only).", "url",
            "ws://127.0.0.1:9743/v1/display" },
        { QStringLiteral("preview"),
            "Show a synthetic subtitle to preview placement and text size." },
        { QStringLiteral("list-screens"), "List available display names and exit." },
        { QStringLiteral("screen"), "Display name; defaults to the primary display.", "name" },
        { QStringLiteral("width"), "Maximum width in logical pixels.", "pixels", "900" },
        { QStringLiteral("font-size"), "Text size in logical pixels.", "pixels", "30" },
        { QStringLiteral("bottom-margin"), "Distance above the bottom edge in logical pixels.",
            "pixels", "64" },
        { QStringLiteral("hold-ms"), "Hide captions after this interval without an update.",
            "milliseconds", "5000" },
    });
    parser.process(app);
    if (parser.isSet("list-screens")) {
        QTextStream output(stdout);
        for (auto* screen : QGuiApplication::screens()) {
            output << screen->name() << "  " << screen->geometry().width() << 'x'
                   << screen->geometry().height() << '\n';
        }
        return 0;
    }
    if (!QGuiApplication::platformName().startsWith("wayland")
        || !qEnvironmentVariable("XDG_CURRENT_DESKTOP")
            .split(':')
            .contains("KDE", Qt::CaseInsensitive)) {
        qCritical("This renderer requires a KDE Plasma Wayland session");
        return 1;
    }
    const auto sources = int(parser.isSet("stdin")) + int(parser.isSet("token-file"))
        + int(parser.isSet("preview"));
    if (sources != 1) {
        qCritical("Choose exactly one source: --stdin, --token-file, or --preview");
        return 1;
    }
    const auto number
        = [&parser](const QString& name, int minimum, int maximum) -> std::optional<int> {
        bool valid;
        const int value = parser.value(name).toInt(&valid);
        if (!valid || value < minimum || value > maximum) {
            qCritical().noquote() << name << "must be between" << minimum << "and" << maximum;
            return std::nullopt;
        }
        return value;
    };
    const auto width = number("width", 240, 3840);
    const auto fontSize = number("font-size", 14, 72);
    const auto margin = number("bottom-margin", 0, 1000);
    const auto hold = number("hold-ms", 200, 60000);
    if (!width || !fontSize || !margin || !hold)
        return 1;

    std::optional<QString> token;
    QUrl server(parser.value("server"));
    if (parser.isSet("token-file")) {
        if (server.scheme() != "ws" || !QHostAddress(server.host()).isLoopback()
            || server.path() != "/v1/display" || !server.userInfo().isEmpty() || server.hasQuery()
            || server.hasFragment() || server.port(9743) < 1) {
            qCritical("Server must be a loopback ws:// address with path /v1/display");
            return 1;
        }
        token = readToken(parser.value("token-file"));
        if (!token)
            return 1;
    }
    CaptionModel captions(*hold);
    EventSource source;
    KdeWindow window({ parser.value("screen"), *width, *fontSize, *margin }, captions);
    QString error;
    if (!window.initialize(&error)) {
        qCritical().noquote() << error;
        return 1;
    }
    QObject::connect(&source, &EventSource::eventReceived, &app, [&](const QJsonObject& event) {
        QString error;
        if (event.value("type") == "display" && event.value("status") == "error") {
            qWarning().noquote() << event.value("message").toString();
        }
        if (!captions.apply(event, &error)) {
            captions.clear();
            qCritical().noquote() << error;
            app.exit(1);
        }
    });
    QObject::connect(&source, &EventSource::disconnected, &captions, &CaptionModel::reset);
    QObject::connect(&source, &EventSource::ended, &app,
        [&] { QTimer::singleShot(captions.remainingMs(), &app, &QCoreApplication::quit); });
    QObject::connect(&source, &EventSource::failed, &app, [&](const QString& message) {
        captions.clear();
        qCritical().noquote() << message;
        app.exit(1);
    });
    if (parser.isSet("stdin")) {
        if (!source.startStdin(&error)) {
            qCritical().noquote() << error;
            return 1;
        }
    } else if (token) {
        source.startSocket(server, *token);
    } else {
        QTimer::singleShot(0, &app, [&] {
            QString error;
            captions.apply({ { "type", "ready" }, { "version", 1 },
                               { "backend", QJsonObject { { "synthetic", true } } } },
                &error);
            captions.apply(
                { { "type", "caption" },
                    { "caption",
                        QJsonObject { { "segment_id", 0 }, { "revision", 1 }, { "is_final", true },
                            { "stable_text", "实时字幕 · 日本語の字幕 · Live captions" },
                            { "unstable_text", "" } } } },
                &error);
            QTimer::singleShot(*hold, &app, &QCoreApplication::quit);
        });
    }
    return app.exec();
}
