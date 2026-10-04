#include "session_controller.h"
#include "translations.h"

#include <QTest>

class TranslationsTest : public QObject {
    Q_OBJECT
private slots:
    void uiUsesLocaleWithEnglishFallback_data()
    {
        QTest::addColumn<QString>("locale");
        QTest::addColumn<QString>("status");
        QTest::addColumn<QString>("oneThread");
        QTest::addColumn<QString>("twoThreads");
        QTest::newRow("Chinese") << "zh_CN" << "尚未启动" << "1 个线程" << "2 个线程";
        QTest::newRow("English") << "en_US" << "Not started" << "1 thread" << "2 threads";
        QTest::newRow("English region") << "en_GB" << "Not started" << "1 thread" << "2 threads";
        QTest::newRow("Unsupported language")
            << "fr_FR" << "Not started" << "1 thread" << "2 threads";
    }

    void uiUsesLocaleWithEnglishFallback()
    {
        QFETCH(QString, locale);
        QFETCH(QString, status);
        QFETCH(QString, oneThread);
        QFETCH(QString, twoThreads);
        Translations translations { QLocale(locale) };
        SessionController session;
        QCOMPARE(session.status(), status);
        QCOMPARE(QCoreApplication::translate("MainWindow", "%n thread(s)", nullptr, 1), oneThread);
        QCOMPARE(QCoreApplication::translate("MainWindow", "%n thread(s)", nullptr, 2), twoThreads);
    }
};

QTEST_GUILESS_MAIN(TranslationsTest)
#include "translations_test.moc"
