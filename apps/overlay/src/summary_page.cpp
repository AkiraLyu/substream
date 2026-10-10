#include "summary_page.h"
#include "widgets.h"

#include <QComboBox>
#include <QDesktopServices>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QGroupBox>
#include <QJsonDocument>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QSaveFile>
#include <QSpinBox>
#include <QStandardPaths>
#include <QTabWidget>
#include <QTextBrowser>
#include <QVBoxLayout>

using Widgets::filePicker;
using Widgets::label;

namespace {
class MarkdownView final : public QTextBrowser {
protected:
    // Provider text cannot cause automatic file or network resource loads.
    QVariant loadResource(int, const QUrl&) override { return { }; }
};
}

SummaryPage::SummaryPage(QWidget* parent)
    : QWidget(parent)
{
    QFile defaults(QStringLiteral(":/defaults/summary.example.json"));
    if (!defaults.open(QIODevice::ReadOnly))
        qFatal("Summary defaults are missing");
    m_defaults = QJsonDocument::fromJson(defaults.readAll()).object();
    m_configurationPath
        = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/summary.json";
    auto saved = m_defaults;
    QString loadError;
    QFile file(m_configurationPath);
    if (file.exists()) {
        QJsonParseError error;
        if (file.open(QIODevice::ReadOnly) && file.size() <= 128 * 1024) {
            const auto document = QJsonDocument::fromJson(file.readAll(), &error);
            if (document.isObject() && error.error == QJsonParseError::NoError) {
                const auto values = document.object();
                for (auto it = values.begin(); it != values.end(); ++it)
                    saved.insert(it.key(), it.value());
            } else
                loadError = tr("Cannot read the saved AI settings.");
        } else
            loadError = tr("Cannot read the saved AI settings.");
    }
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(24, 20, 24, 20);
    layout->setSpacing(16);
    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);
    layout->addLayout(columns, 1);
    m_settings = new QTabWidget;
    columns->addWidget(m_settings, 5);
    auto* api = new QWidget;
    auto* apiLayout = new QVBoxLayout(api);
    apiLayout->setContentsMargins(16, 20, 16, 16);
    auto* form = new QFormLayout;
    form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
    form->setRowWrapPolicy(QFormLayout::WrapAllRows);
    m_endpoint = new QLineEdit(saved.value("endpoint").toString());
    m_key = new QLineEdit(saved.value("api_key").toString());
    m_key->setEchoMode(QLineEdit::Password);
    m_keyEnv = new QLineEdit(saved.value("api_key_env").toString());
    m_model = new QLineEdit(saved.value("model").toString());
    m_timeout = new QSpinBox;
    m_timeout->setRange(1, 3600);
    m_timeout->setValue(saved.value("timeout_secs").toInt());
    m_timeout->setSuffix(tr(" s"));
    m_inputLimit = new QSpinBox;
    m_inputLimit->setRange(1, 2000000);
    m_inputLimit->setValue(saved.value("max_input_chars").toInt());
    form->addRow(tr("API endpoint"), m_endpoint);
    form->addRow(tr("API key"), m_key);
    form->addRow(tr("Model"), m_model);
    form->addRow(tr("Timeout"), m_timeout);
    apiLayout->addLayout(form);
    apiLayout->addStretch();
    m_settings->addTab(api, tr("API"));

    auto* prompts = new QWidget;
    auto* promptsLayout = new QVBoxLayout(prompts);
    promptsLayout->setContentsMargins(16, 20, 16, 16);
    promptsLayout->addWidget(label(tr("System prompt")));
    m_system = new QPlainTextEdit(saved.value("system_prompt").toString());
    promptsLayout->addWidget(m_system, 1);
    auto* promptHeading = new QHBoxLayout;
    promptHeading->addWidget(label(tr("User prompt")));
    promptHeading->addStretch();
    auto* variables = new QComboBox;
    variables->addItem(tr("Insert variable"));
    variables->addItem(tr("Subtitles"), "{{transcript}}");
    variables->addItem(tr("Title"), "{{title}}");
    variables->addItem(tr("Source link"), "{{url}}");
    promptHeading->addWidget(variables);
    promptsLayout->addLayout(promptHeading);
    m_prompt = new QPlainTextEdit(saved.value("user_prompt").toString());
    promptsLayout->addWidget(m_prompt, 2);
    connect(variables, &QComboBox::activated, this, [this, variables](int index) {
        if (index > 0)
            m_prompt->insertPlainText(variables->itemData(index).toString());
        variables->setCurrentIndex(0);
    });
    auto* reset = new QPushButton(tr("Reset prompts"));
    connect(reset, &QPushButton::clicked, this, [this] {
        m_system->setPlainText(m_defaults.value("system_prompt").toString());
        m_prompt->setPlainText(m_defaults.value("user_prompt").toString());
    });
    promptsLayout->addWidget(reset, 0, Qt::AlignRight);
    m_settings->addTab(prompts, tr("Prompts"));

    auto* advanced = new QWidget;
    auto* advancedLayout = new QVBoxLayout(advanced);
    advancedLayout->setContentsMargins(16, 20, 16, 16);
    auto* advancedForm = new QFormLayout;
    advancedForm->setRowWrapPolicy(QFormLayout::WrapAllRows);
    advancedForm->addRow(tr("API key environment variable"), m_keyEnv);
    advancedForm->addRow(tr("Input character limit"), m_inputLimit);
    advancedLayout->addLayout(advancedForm);
    advancedLayout->addWidget(label(tr("Request parameters (JSON)")));
    m_parameters = new QPlainTextEdit(
        QString::fromUtf8(QJsonDocument(saved.value("parameters").toObject()).toJson()));
    m_parameters->setPlaceholderText(
        QStringLiteral("{\"temperature\": 0.2, \"max_completion_tokens\": 4096}"));
    advancedLayout->addWidget(m_parameters, 1);
    m_settings->addTab(advanced, tr("Advanced"));

    auto* result = new QGroupBox(tr("Summary"));
    columns->addWidget(result, 5);
    auto* resultLayout = new QVBoxLayout(result);
    resultLayout->setContentsMargins(16, 20, 16, 16);
    resultLayout->setSpacing(12);
    resultLayout->addWidget(label(tr("Subtitle document")));
    m_document = new QLineEdit;
    m_documentPicker = filePicker(m_document, tr("Subtitle documents (*.json)"));
    resultLayout->addWidget(m_documentPicker);
    m_status = label();
    resultLayout->addWidget(m_status);
    m_progress = new QProgressBar;
    m_progress->setRange(0, 0);
    resultLayout->addWidget(m_progress);
    m_preview = new MarkdownView;
    m_preview->setOpenLinks(false);
    connect(m_preview, &QTextBrowser::anchorClicked, this, [](const QUrl& url) {
        if (url.scheme() == "http" || url.scheme() == "https")
            QDesktopServices::openUrl(url);
    });
    resultLayout->addWidget(m_preview, 1);
    m_error = label();
    m_error->setStyleSheet(QStringLiteral("color: #c34235;"));
    resultLayout->addWidget(m_error);

    auto* actions = new QHBoxLayout;
    m_saveSettings = new QPushButton(tr("Save settings"));
    actions->addWidget(m_saveSettings);
    actions->addStretch();
    m_export = new QPushButton(tr("Save Markdown"));
    m_start = new QPushButton(tr("Generate summary"));
    m_cancel = new QPushButton(tr("Cancel"));
    for (auto* button : { m_export, m_start, m_cancel }) {
        button->setMinimumHeight(36);
        actions->addWidget(button);
    }
    layout->addLayout(actions);
    connect(m_saveSettings, &QPushButton::clicked, this, [this] {
        if (saveSettings())
            m_status->setText(tr("Settings saved"));
    });
    connect(m_start, &QPushButton::clicked, this, &SummaryPage::start);
    connect(m_cancel, &QPushButton::clicked, &m_controller, &SummaryController::stop);
    connect(m_export, &QPushButton::clicked, this, &SummaryPage::exportSummary);
    connect(&m_controller, &SummaryController::changed, this, &SummaryPage::refresh);
    refresh();
    m_error->setText(loadError);
}

void SummaryPage::setDocument(const QString& path)
{
    if (!active())
        m_document->setText(path);
}

QString SummaryPage::configuration() const
{
    return QFileInfo(m_configurationPath).isFile() ? m_configurationPath : QString();
}

QJsonObject SummaryPage::settings() const
{
    return { { "endpoint", m_endpoint->text().trimmed() }, { "api_key", m_key->text().trimmed() },
        { "api_key_env", m_keyEnv->text().trimmed() }, { "model", m_model->text().trimmed() },
        { "timeout_secs", m_timeout->value() }, { "max_input_chars", m_inputLimit->value() },
        { "system_prompt", m_system->toPlainText() }, { "user_prompt", m_prompt->toPlainText() } };
}

bool SummaryPage::saveSettings()
{
    m_error->clear();
    QJsonParseError error;
    const auto parameters = QJsonDocument::fromJson(m_parameters->toPlainText().toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !parameters.isObject()) {
        m_error->setText(tr("Request parameters must be a JSON object."));
        return false;
    }
    auto value = settings();
    value.insert("parameters", parameters.object());
    if (!QDir().mkpath(QFileInfo(m_configurationPath).absolutePath())) {
        m_error->setText(tr("Cannot create the settings directory."));
        return false;
    }
    QSaveFile file(m_configurationPath);
    const auto bytes = QJsonDocument(value).toJson();
    if (!file.open(QIODevice::WriteOnly)
        || !file.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner)
        || file.write(bytes) != bytes.size() || !file.commit()) {
        m_error->setText(tr("Cannot save the AI settings: %1").arg(file.errorString()));
        return false;
    }
    return true;
}

void SummaryPage::start()
{
    if (!saveSettings())
        return;
    m_preview->clear();
    m_controller.start(m_program, m_configurationPath, m_document->text().trimmed());
}

void SummaryPage::refresh()
{
    const bool active = m_controller.active();
    m_settings->setEnabled(!active);
    m_documentPicker->setEnabled(!active);
    m_saveSettings->setEnabled(!active);
    m_start->setEnabled(!active);
    m_cancel->setEnabled(active && !m_controller.stopping());
    m_export->setEnabled(!active && !m_controller.markdown().isEmpty());
    m_status->setText(m_controller.status());
    m_error->setText(m_controller.error());
    m_progress->setVisible(active);
    if (!active && !m_controller.markdown().isEmpty())
        m_preview->setMarkdown(m_controller.markdown());
    emit activityChanged();
}

void SummaryPage::exportSummary()
{
    const auto path = QFileDialog::getSaveFileName(this, tr("Save summary"),
        QFileInfo(m_document->text()).absolutePath() + "/summary.md", tr("Markdown (*.md)"));
    if (path.isEmpty())
        return;
    QSaveFile file(path);
    const auto bytes = m_controller.markdown().toUtf8();
    if (!file.open(QIODevice::WriteOnly) || file.write(bytes) != bytes.size() || !file.commit())
        m_error->setText(tr("Cannot save the summary: %1").arg(file.errorString()));
}
