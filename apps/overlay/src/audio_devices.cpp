#include "audio_devices.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

#include <algorithm>

AudioDevices::AudioDevices(QObject* parent)
    : QObject(parent)
{
    m_deadline.setSingleShot(true);
    m_deadline.setInterval(5000);
    connect(&m_deadline, &QTimer::timeout, &m_process, &QProcess::kill);
    connect(&m_process, &QProcess::readyReadStandardOutput, this, [this] {
        m_output += m_process.readAllStandardOutput();
        if (m_output.size() > 16 * 1024 * 1024)
            m_process.kill();
    });
    connect(&m_process, &QProcess::errorOccurred, this, [this](QProcess::ProcessError error) {
        if (error == QProcess::FailedToStart) {
            m_deadline.stop();
            emit failed(
                QStringLiteral("无法读取音频设备，请确认已安装 PipeWire 工具（pw-dump）。"));
        }
    });
    connect(&m_process, &QProcess::finished, this, [this](int code, QProcess::ExitStatus status) {
        m_deadline.stop();
        const auto document = QJsonDocument::fromJson(m_output);
        if (code != 0 || status != QProcess::NormalExit || !document.isArray()) {
            emit failed(QStringLiteral("无法读取 PipeWire 设备，请检查音频服务是否运行。"));
            return;
        }
        m_devices.clear();
        for (const auto& value : document.array()) {
            const auto object = value.toObject();
            if (object.value("type") != "PipeWire:Interface:Node")
                continue;
            const auto props = object.value("info").toObject().value("props").toObject();
            const auto kind = props.value("media.class").toString();
            if (kind != "Audio/Sink" && kind != "Audio/Source")
                continue;
            AudioDevice device;
            device.serial = props.value("object.serial").toVariant().toString();
            device.name = props.value("node.name").toString();
            device.sink = kind == "Audio/Sink";
            const auto description = props.value("node.description").toString(device.name);
            device.label
                = (device.sink ? QStringLiteral("系统输出 · ") : QStringLiteral("音频输入 · "))
                + description;
            if (!device.serial.isEmpty() && !device.name.isEmpty())
                m_devices.append(device);
        }
        std::sort(m_devices.begin(), m_devices.end(), [](const auto& a, const auto& b) {
            if (a.sink != b.sink)
                return a.sink;
            return a.label < b.label;
        });
        emit changed();
    });
}

AudioDevices::~AudioDevices()
{
    disconnect(&m_process, nullptr, this, nullptr);
    m_process.kill();
    m_process.waitForFinished(1000);
}

void AudioDevices::refresh()
{
    if (m_process.state() != QProcess::NotRunning)
        return;
    m_output.clear();
    m_process.start(QStringLiteral("pw-dump"), { });
    m_deadline.start();
}
