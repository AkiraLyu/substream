#pragma once

#include <QObject>
#include <QProcess>
#include <QTimer>

struct AudioDevice {
    QString serial;
    QString name;
    QString label;
    bool sink = false;
};

class AudioDevices final : public QObject {
    Q_OBJECT
public:
    explicit AudioDevices(QObject* parent = nullptr);
    ~AudioDevices() override;
    void refresh();
    const QList<AudioDevice>& devices() const { return m_devices; }

signals:
    void changed();
    void failed(const QString& message);

private:
    QProcess m_process;
    QTimer m_deadline;
    QByteArray m_output;
    QList<AudioDevice> m_devices;
};
