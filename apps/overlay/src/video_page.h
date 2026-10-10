#pragma once

#include "video_controller.h"

#include <QWidget>

class QComboBox;
class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QProgressBar;
class QPushButton;

class VideoPage final : public QWidget {
    Q_OBJECT
public:
    explicit VideoPage(QWidget* parent = nullptr);
    QString configuration() const;
    void setProgram(const QString& program) { m_program = program; }
    bool active() const { return m_controller.active(); }
    void stop() { m_controller.stop(); }
    void saveSettings();

signals:
    void activityChanged();
    void summarizeRequested(const QString& document);

private:
    void start();
    void refresh();
    void loadResult();
    VideoController m_controller;
    QString m_program;
    QString m_loadedDocument;
    QWidget* m_fields;
    QLineEdit* m_url;
    QLineEdit* m_config;
    QComboBox* m_cookies;
    QComboBox* m_browser;
    QLineEdit* m_profile;
    QLineEdit* m_cookieFile;
    QLabel* m_status;
    QLabel* m_error;
    QLabel* m_title;
    QLabel* m_output;
    QPlainTextEdit* m_preview;
    QPlainTextEdit* m_log;
    QProgressBar* m_progress;
    QPushButton* m_start;
    QPushButton* m_cancel;
    QPushButton* m_open;
    QPushButton* m_summarize;
};
