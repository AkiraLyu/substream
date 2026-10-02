// Process-boundary fixture: capture produces PCM; recognition holds one utterance until EOF.
#include <QCoreApplication>
#include <QFileInfo>
#include <QThread>

#include <array>
#include <csignal>
#include <cstdio>
#include <unistd.h>

namespace {
volatile std::sig_atomic_t running = 1;
void stop(int) { running = 0; }
}

int main(int argc, char** argv)
{
    QCoreApplication app(argc, argv);
    if (QFileInfo(QString::fromLocal8Bit(argv[0])).fileName() == "pw-cat") {
        std::signal(SIGTERM, stop);
        const std::array<char, 640> pcm { };
        while (running) {
            if (::write(STDOUT_FILENO, pcm.data(), pcm.size()) < 0)
                return 1;
            QThread::msleep(20);
        }
        return 0;
    }
    for (const auto& arg : app.arguments()) {
        if (arg.endsWith("slow.toml"))
            QThread::msleep(1000);
    }
    std::puts(
        R"({"type":"ready","version":1,"backend":{"name":"fixture","model":"fixture.onnx","threads":1,"languages":["zh"]}})");
    std::fflush(stdout);
    std::array<char, 4096> bytes { };
    size_t received = 0;
    while (const auto count = std::fread(bytes.data(), 1, bytes.size(), stdin))
        received += count;
    std::puts(
        R"({"type":"caption","caption":{"segment_id":0,"revision":1,"is_final":true,"stable_text":"末尾字幕","unstable_text":""}})");
    std::printf(
        "{\"type\":\"finished\",\"samples_processed\":%zu,\"processing_ms\":0}\n", received / 2);
    return 0;
}
