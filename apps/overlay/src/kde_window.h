#pragma once

#include <QQuickView>

class CaptionModel;

struct OverlayOptions {
    QString screen;
    int width = 900;
    int fontSize = 30;
    int margin = 64;
};

// Only this class depends on KDE's window placement API.
class KdeWindow final : public QQuickView {
    Q_OBJECT
public:
    KdeWindow(const OverlayOptions& options, CaptionModel& captions);
    bool initialize(QString* error);

private:
    void fitScreen();
    OverlayOptions m_options;
    CaptionModel& m_captions;
};
