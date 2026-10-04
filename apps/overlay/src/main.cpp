#include "caption_model.h"
#include "event_source.h"
#include "kde_window.h"
#include "main_window.h"
#include "token_file.h"
#include "translations.h"
#include <QApplication>

#include <QCommandLineParser>
#include <QGuiApplication>
#include <QHostAddress>
#include <QScreen>
#include <QTextStream>

#include <optional>

int main(int argc, char** argv)
{
    qputenv("QT_FORCE_STDERR_LOGGING", "1");
    QApplication app(argc, argv);
    QCoreApplication::setOrganizationName(QStringLiteral("Substream"));
    QCoreApplication::setApplicationName(QStringLiteral("substream-overlay"));
    QCoreApplication::setApplicationVersion(QStringLiteral("0.1.0"));
    QGuiApplication::setDesktopFileName(QStringLiteral("substream-overlay"));
    Translations translations;

    QCommandLineParser parser;
    parser.setApplicationDescription(
        QCoreApplication::translate("main", "Desktop controls and KDE Wayland captions"));
    parser.addHelpOption();
    parser.addVersionOption();
    parser.addOptions({
        { QStringLiteral("stdin"),
            QCoreApplication::translate(
                "main", "Read newline-delimited caption events from standard input.") },
        { QStringLiteral("token-file"),
            QCoreApplication::translate(
                "main", "Subscribe to the local subtitle service using a private token file."),
            "path" },
        { QStringLiteral("server"),
            QCoreApplication::translate("main", "Display WebSocket URL (loopback only)."), "url",
            "ws://127.0.0.1:9743/v1/display" },
        { QStringLiteral("list-screens"),
            QCoreApplication::translate("main", "List available display names and exit.") },
        { QStringLiteral("screen"),
            QCoreApplication::translate("main", "Display name; defaults to the primary display."),
            "name" },
        { QStringLiteral("width"),
            QCoreApplication::translate("main", "Maximum width in logical pixels."), "pixels",
            "900" },
        { QStringLiteral("font-size"),
            QCoreApplication::translate("main", "Text size in logical pixels."), "pixels", "30" },
        { QStringLiteral("bottom-margin"),
            QCoreApplication::translate(
                "main", "Distance above the bottom edge in logical pixels."),
            "pixels", "64" },
        { QStringLiteral("hold-ms"),
            QCoreApplication::translate(
                "main", "Hide captions after this interval without an update."),
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
    const bool kdeWayland = QGuiApplication::platformName().startsWith("wayland")
        && qEnvironmentVariable("XDG_CURRENT_DESKTOP")
               .split(':')
               .contains("KDE", Qt::CaseInsensitive);
    if (!parser.isSet("stdin") && !parser.isSet("token-file")) {
        MainWindow window(kdeWayland);
        window.show();
        return app.exec();
    }
    if (!kdeWayland) {
        qCritical().noquote() << QCoreApplication::translate(
            "main", "This renderer requires a KDE Plasma Wayland session");
        return 1;
    }
    if (parser.isSet("stdin") && parser.isSet("token-file")) {
        qCritical().noquote() << QCoreApplication::translate(
            "main", "Choose either --stdin or --token-file");
        return 1;
    }
    const auto number
        = [&parser](const QString& name, int minimum, int maximum) -> std::optional<int> {
        bool valid;
        const int value = parser.value(name).toInt(&valid);
        if (!valid || value < minimum || value > maximum) {
            qCritical().noquote() << QCoreApplication::translate(
                "main", "%1 must be between %2 and %3")
                                         .arg(name)
                                         .arg(minimum)
                                         .arg(maximum);
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
            qCritical().noquote() << QCoreApplication::translate(
                "main", "Server must be a loopback ws:// address with path /v1/display");
            return 1;
        }
        QString error;
        token = readToken(parser.value("token-file"), &error);
        if (!token) {
            qCritical().noquote() << error;
            return 1;
        }
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
    }
    return app.exec();
}
