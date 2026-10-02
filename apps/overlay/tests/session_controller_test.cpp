#include "session_controller.h"

#include <QFile>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>

class SessionControllerTest : public QObject {
    Q_OBJECT
private:
    QTemporaryDir m_directory;
    QByteArray m_path;
    LaunchOptions options() const
    {
        LaunchOptions options;
        options.program = QCoreApplication::applicationDirPath() + "/session-peer";
        options.config = m_directory.filePath("model.toml");
        options.device = { "test-device", "test-device", "测试音频源", true };
        return options;
    }

private slots:
    void initTestCase()
    {
        QVERIFY(m_directory.isValid());
        QFile config(m_directory.filePath("model.toml"));
        QVERIFY(config.open(QIODevice::WriteOnly));
        config.close();
        QVERIFY(config.copy(m_directory.filePath("slow.toml")));
        QVERIFY(QFile::link(QCoreApplication::applicationDirPath() + "/session-peer",
            m_directory.filePath("pw-cat")));
        m_path = qgetenv("PATH");
        qputenv("PATH", m_directory.path().toUtf8() + ":" + m_path);
    }

    void cleanupTestCase() { qputenv("PATH", m_path); }

    void stopFlushesAudioAndAllowsRestart()
    {
        SessionController session;
        for (int run = 0; run < 2; ++run) {
            QSignalSpy events(&session, &SessionController::eventReceived);
            session.start(options());
            QTRY_VERIFY_WITH_TIMEOUT(session.samples() >= 640, 5000);
            QVERIFY(!session.backend().value("model").toString().isEmpty());
            session.stop();
            QTRY_VERIFY_WITH_TIMEOUT(!session.active(), 5000);
            QVERIFY2(session.error().isEmpty(), qPrintable(session.error()));
            bool finalCaption = false;
            bool finished = false;
            for (const auto& args : events) {
                const auto event = args.first().toJsonObject();
                if (event.value("type") == "caption") {
                    const auto caption = event.value("caption").toObject();
                    finalCaption |= caption.value("is_final").toBool()
                        && !caption.value("stable_text").toString().isEmpty();
                }
                if (event.value("type") == "finished") {
                    finished = true;
                    QCOMPARE(event.value("samples_processed").toInteger(), session.samples());
                }
            }
            QVERIFY(finalCaption);
            QVERIFY(finished);
        }
    }

    void startupFailureAndDeviceLossLeaveTheSessionReusable()
    {
        SessionController session;
        auto invalid = options();
        invalid.program = m_directory.filePath("missing-program");
        session.start(invalid);
        QTRY_VERIFY_WITH_TIMEOUT(!session.active(), 5000);
        QVERIFY(!session.error().isEmpty());
        session.start(options());
        QTRY_VERIFY_WITH_TIMEOUT(session.samples() > 0, 5000);
        session.inputUnavailable();
        QTRY_VERIFY_WITH_TIMEOUT(!session.active(), 5000);
        QVERIFY(!session.error().isEmpty());
        auto loading = options();
        loading.config = m_directory.filePath("slow.toml");
        session.start(loading);
        session.stop();
        QTRY_VERIFY_WITH_TIMEOUT(!session.active(), 5000);
        QCOMPARE(session.samples(), 0);
        session.start(options());
        QTRY_VERIFY_WITH_TIMEOUT(session.samples() > 0, 5000);
        session.stop();
        QTRY_VERIFY_WITH_TIMEOUT(!session.active(), 5000);
        QVERIFY2(session.error().isEmpty(), qPrintable(session.error()));
    }
};

QTEST_GUILESS_MAIN(SessionControllerTest)
#include "session_controller_test.moc"
