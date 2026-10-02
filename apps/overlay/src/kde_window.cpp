#include "kde_window.h"
#include "caption_model.h"

#include <LayerShellQt/Window>
#include <QGuiApplication>
#include <QQuickItem>
#include <QScreen>

KdeWindow::KdeWindow(const OverlayOptions& options, CaptionModel& captions)
    : m_options(options)
    , m_captions(captions)
{
}

bool KdeWindow::initialize(QString* error)
{
    auto* selected = QGuiApplication::primaryScreen();
    if (!m_options.screen.isEmpty()) {
        selected = nullptr;
        for (auto* screen : QGuiApplication::screens()) {
            if (screen->name() == m_options.screen)
                selected = screen;
        }
    }
    if (!selected) {
        *error = QStringLiteral("Display not found; use --list-screens to see available displays");
        return false;
    }
    setTitle(QStringLiteral("Substream"));
    setFlags(Qt::Window | Qt::FramelessWindowHint | Qt::WindowTransparentForInput
        | Qt::WindowDoesNotAcceptFocus);
    setColor(Qt::transparent);
    setScreen(selected);
    auto* layer = LayerShellQt::Window::get(this);
    layer->setScope(QStringLiteral("substream"));
    layer->setLayer(LayerShellQt::Window::LayerOverlay);
    layer->setAnchors(LayerShellQt::Window::AnchorBottom);
    layer->setKeyboardInteractivity(LayerShellQt::Window::KeyboardInteractivityNone);
    layer->setExclusiveZone(0);
    layer->setMargins(QMargins(0, 0, 0, m_options.margin));
    setResizeMode(QQuickView::SizeRootObjectToView);
    setSource(QUrl(QStringLiteral("qrc:/qml/Overlay.qml")));
    if (status() != QQuickView::Ready) {
        *error = QStringLiteral("Cannot load the subtitle view");
        return false;
    }
    rootObject()->setProperty("fontSize", m_options.fontSize);
    connect(&m_captions, &CaptionModel::changed, this,
        [this] { rootObject()->setProperty("captionText", m_captions.text()); });
    connect(selected, &QScreen::geometryChanged, this, [this] { fitScreen(); });
    connect(qGuiApp, &QGuiApplication::screenRemoved, this, [this, selected](QScreen* removed) {
        if (removed == selected) {
            qWarning("Subtitle display disconnected; restart with an available display");
            close();
        }
    });
    fitScreen();
    show();
    return true;
}

void KdeWindow::fitScreen()
{
    const auto available = screen()->geometry().size();
    resize(qMax(1, qMin(m_options.width, available.width() - 32)),
        qMax(1, qMin(m_options.fontSize * 6 + 64, available.height() / 2)));
}
