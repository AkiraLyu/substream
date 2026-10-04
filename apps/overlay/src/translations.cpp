#include "translations.h"

#include <QCoreApplication>
#include <QLibraryInfo>

Translations::Translations(const QLocale& locale)
{
    if (!m_app.load(
            locale, QStringLiteral("substream"), QStringLiteral("_"), QStringLiteral(":/i18n"))) {
        if (!m_app.load(QStringLiteral(":/i18n/substream_en.qm")))
            qFatal("The built-in English translation catalog is missing");
    }
    // Use the selected UI language for Qt's standard dialogs as well.
    if (m_qt.load(QLocale(m_app.language()), QStringLiteral("qtbase"), QStringLiteral("_"),
            QLibraryInfo::path(QLibraryInfo::TranslationsPath))) {
        QCoreApplication::installTranslator(&m_qt);
    }
    QCoreApplication::installTranslator(&m_app);
}
