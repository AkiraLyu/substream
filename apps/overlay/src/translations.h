#pragma once

#include <QLocale>
#include <QTranslator>

// Keep this object alive for the lifetime of the UI. Install before creating widgets.
class Translations final {
public:
    explicit Translations(const QLocale& locale = QLocale::system());

private:
    QTranslator m_qt;
    QTranslator m_app;
};
