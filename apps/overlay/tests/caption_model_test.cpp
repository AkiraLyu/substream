#include "caption_model.h"

#include <QTest>

namespace {
QJsonObject caption(int revision, const QString& text, bool final = false)
{
    return { { "segment_id", 0 }, { "revision", revision }, { "stable_text", "" },
        { "unstable_text", text }, { "is_final", final } };
}

QJsonObject snapshot(int session, const QJsonObject& caption, int ageMs = 0)
{
    return { { "type", "display" }, { "version", 1 }, { "session_id", session },
        { "status", "listening" }, { "backend", QJsonObject { { "synthetic", true } } },
        { "caption", caption }, { "caption_age_ms", ageMs } };
}
}

class CaptionModelTest : public QObject {
    Q_OBJECT

private slots:
    void correctionsReplaceTextAndNewSessionsStartFresh()
    {
        CaptionModel model(5000);
        QString error;
        QVERIFY(model.apply(snapshot(1, caption(1, "今天用 Ruby")), &error));
        QVERIFY(model.visible());
        QVERIFY(model.synthetic());
        QVERIFY(model.apply(
            snapshot(1, caption(3, "今天用 Rust 👩‍💻 <字幕>", true)), &error));
        QCOMPARE(model.text(), QString("今天用 Rust 👩‍💻 <字幕>"));
        QVERIFY(model.apply(snapshot(1, caption(2, "迟到的旧字幕")), &error));
        QCOMPARE(model.text(), QString("今天用 Rust 👩‍💻 <字幕>"));
        QVERIFY(model.apply(snapshot(2, caption(1, "新会话")), &error));
        QCOMPARE(model.text(), QString("新会话"));
    }

    void captionsExpireAndOldSnapshotsDoNotReappear()
    {
        CaptionModel model(60);
        QString error;
        QVERIFY(model.apply({ { "type", "ready" }, { "version", 1 },
                                { "backend", QJsonObject { { "synthetic", false } } } },
            &error));
        QVERIFY(model.apply(
            { { "type", "caption" }, { "caption", caption(1, "最后一句", true) } }, &error));
        QVERIFY(model.apply({ { "type", "finished" } }, &error));
        QCOMPARE(model.text(), QString("最后一句"));
        QTRY_VERIFY_WITH_TIMEOUT(!model.visible(), 1000);
        model.reset();
        QVERIFY(model.apply(snapshot(1, caption(1, "已经过期"), 1000), &error));
        QVERIFY(!model.visible());
        QVERIFY(model.apply(snapshot(2, caption(1, "正在识别")), &error));
        auto idle = snapshot(2, { });
        idle["status"] = "idle";
        QVERIFY(model.apply(idle, &error));
        QVERIFY(!model.visible());
    }
};

QTEST_GUILESS_MAIN(CaptionModelTest)
#include "caption_model_test.moc"
