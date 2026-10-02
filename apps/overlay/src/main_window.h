#pragma once

#include "audio_devices.h"
#include "session_controller.h"

#include <QMainWindow>
#include <memory>

class CaptionModel;
class KdeWindow;
class QCheckBox;
class QComboBox;
class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QProgressBar;
class QPushButton;
class QSpinBox;
class QTabWidget;

class MainWindow final : public QMainWindow {
    Q_OBJECT
public:
    explicit MainWindow(bool kdeWayland);
    ~MainWindow() override;

protected:
    void closeEvent(QCloseEvent* event) override;

private:
    void start();
    void refreshState();
    void refreshDevices();
    void refreshScreens();
    void saveSettings();

    AudioDevices m_devices;
    SessionController m_session;
    QTimer m_deviceRefresh;
    std::unique_ptr<CaptionModel> m_captions;
    std::unique_ptr<KdeWindow> m_overlay;
    QTabWidget* m_settings;
    QComboBox* m_input;
    QComboBox* m_device;
    QComboBox* m_screen;
    QLineEdit* m_config;
    QLineEdit* m_program;
    QLineEdit* m_extension;
    QLineEdit* m_token;
    QSpinBox* m_threads;
    QSpinBox* m_font;
    QSpinBox* m_width;
    QSpinBox* m_margin;
    QSpinBox* m_hold;
    QCheckBox* m_showOverlay;
    QLabel* m_status;
    QLabel* m_source;
    QLabel* m_model;
    QLabel* m_duration;
    QLabel* m_error;
    QProgressBar* m_level;
    QPlainTextEdit* m_preview;
    QPlainTextEdit* m_log;
    QPushButton* m_start;
    QPushButton* m_stop;
    QString m_savedDevice;
    bool m_closing = false;
};
