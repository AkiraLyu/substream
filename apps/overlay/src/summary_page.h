#pragma once

#include "summary_controller.h"
#include <QWidget>

class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QProgressBar;
class QPushButton;
class QSpinBox;
class QTabWidget;
class QTextBrowser;

class SummaryPage final : public QWidget {
    Q_OBJECT
public:
    explicit SummaryPage(QWidget* parent = nullptr);
    void setProgram(const QString& program) { m_program = program; }
    void setDocument(const QString& path);
    QString configuration() const;
    bool active() const { return m_controller.active(); }
    void stop() { m_controller.stop(); }

signals:
    void activityChanged();

private:
    bool saveSettings();
    void start();
    void refresh();
    void exportSummary();
    QJsonObject settings() const;
    SummaryController m_controller;
    QJsonObject m_defaults;
    QString m_program;
    QString m_configurationPath;
    QTabWidget* m_settings;
    QWidget* m_documentPicker;
    QLineEdit* m_document;
    QLineEdit* m_endpoint;
    QLineEdit* m_key;
    QLineEdit* m_keyEnv;
    QLineEdit* m_model;
    QSpinBox* m_timeout;
    QSpinBox* m_inputLimit;
    QPlainTextEdit* m_parameters;
    QPlainTextEdit* m_system;
    QPlainTextEdit* m_prompt;
    QLabel* m_status;
    QLabel* m_error;
    QProgressBar* m_progress;
    QTextBrowser* m_preview;
    QPushButton* m_start;
    QPushButton* m_cancel;
    QPushButton* m_saveSettings;
    QPushButton* m_export;
};
