#include "video_page.h"
#include "widgets.h"

#include <QComboBox>
#include <QDesktopServices>
#include <QFile>
#include <QFormLayout>
#include <QGroupBox>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QSettings>
#include <QUrl>
#include <QVBoxLayout>

using Widgets::filePicker;
using Widgets::label;

VideoPage::VideoPage(QWidget* parent)
    : QWidget(parent)
{
    QSettings saved;
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(24, 20, 24, 20);
    layout->setSpacing(16);
    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);
    layout->addLayout(columns, 1);
    m_fields = new QWidget;
    auto* fields = new QVBoxLayout(m_fields);
    fields->setContentsMargins(0, 0, 0, 0);
    fields->setSpacing(14);
    columns->addWidget(m_fields, 5);
    m_url = new QLineEdit;
    m_url->setPlaceholderText(QStringLiteral("https://…"));
    fields->addWidget(label(tr("Video link")));
    fields->addWidget(m_url);
    m_config = new QLineEdit(saved.value("videoConfig").toString());
    fields->addWidget(label(tr("Video configuration")));
    fields->addWidget(filePicker(m_config, tr("Video configuration (*.toml)")));
    m_cookies = new QComboBox;
    m_cookies->addItem(
        tr("Use configuration"), static_cast<int>(VideoOptions::Cookies::Configuration));
    m_cookies->addItem(tr("None"), static_cast<int>(VideoOptions::Cookies::None));
    m_cookies->addItem(tr("Browser"), static_cast<int>(VideoOptions::Cookies::Browser));
    m_cookies->addItem(tr("Cookies file"), static_cast<int>(VideoOptions::Cookies::File));
    m_cookies->setCurrentIndex(qMax(0, m_cookies->findData(saved.value("video/cookies", 0))));
    fields->addWidget(label(tr("Cookies")));
    fields->addWidget(m_cookies);
    auto* browserFields = new QWidget;
    auto* browserLayout = new QFormLayout(browserFields);
    browserLayout->setContentsMargins(0, 0, 0, 0);
    m_browser = new QComboBox;
    for (const auto& browser :
        { "Firefox", "Chromium", "Chrome", "Brave", "Edge", "Vivaldi", "Opera", "Whale" })
        m_browser->addItem(QString::fromLatin1(browser), QString::fromLatin1(browser).toLower());
    m_browser->setCurrentIndex(
        qMax(0, m_browser->findData(saved.value("video/browser", "firefox"))));
    m_profile = new QLineEdit(saved.value("video/profile").toString());
    m_profile->setPlaceholderText(tr("Default profile"));
    browserLayout->addRow(tr("Browser"), m_browser);
    browserLayout->addRow(tr("Profile"), m_profile);
    fields->addWidget(browserFields);
    m_cookieFile = new QLineEdit(saved.value("video/cookieFile").toString());
    auto* cookieFile = filePicker(m_cookieFile, tr("Cookies files (*.txt);;All files (*)"));
    fields->addWidget(cookieFile);
    const auto cookiesChanged = [this, browserFields, cookieFile] {
        const auto source = static_cast<VideoOptions::Cookies>(m_cookies->currentData().toInt());
        browserFields->setVisible(source == VideoOptions::Cookies::Browser);
        cookieFile->setVisible(source == VideoOptions::Cookies::File);
    };
    connect(m_cookies, &QComboBox::currentIndexChanged, this, cookiesChanged);
    cookiesChanged();
    fields->addStretch();

    auto* current = new QGroupBox(tr("Video task"));
    columns->addWidget(current, 5);
    auto* currentLayout = new QVBoxLayout(current);
    currentLayout->setContentsMargins(16, 20, 16, 16);
    currentLayout->setSpacing(12);
    m_status = label();
    auto font = m_status->font();
    font.setBold(true);
    m_status->setFont(font);
    currentLayout->addWidget(m_status);
    m_progress = new QProgressBar;
    m_progress->setRange(0, 0);
    currentLayout->addWidget(m_progress);
    m_title = label();
    currentLayout->addWidget(m_title);
    m_preview = new QPlainTextEdit;
    m_preview->setReadOnly(true);
    m_preview->setPlaceholderText(tr("Subtitle preview"));
    currentLayout->addWidget(m_preview, 1);
    m_output = label();
    currentLayout->addWidget(m_output);
    m_error = label();
    m_error->setStyleSheet(QStringLiteral("color: #c34235;"));
    currentLayout->addWidget(m_error);
    m_log = new QPlainTextEdit;
    m_log->setReadOnly(true);
    m_log->setMaximumBlockCount(200);
    m_log->setMaximumHeight(110);
    m_log->hide();
    auto* logs = new QPushButton(tr("Logs"));
    logs->setCheckable(true);
    logs->setFlat(true);
    connect(logs, &QPushButton::toggled, m_log, &QWidget::setVisible);
    currentLayout->addWidget(logs, 0, Qt::AlignRight);
    currentLayout->addWidget(m_log);

    auto* actions = new QHBoxLayout;
    actions->addStretch();
    m_open = new QPushButton(tr("Open output folder"));
    m_summarize = new QPushButton(tr("AI summary"));
    m_start = new QPushButton(tr("Get video subtitles"));
    m_cancel = new QPushButton(tr("Cancel"));
    for (auto* button : { m_open, m_summarize, m_start, m_cancel }) {
        button->setMinimumHeight(36);
        actions->addWidget(button);
    }
    layout->addLayout(actions);
    connect(m_open, &QPushButton::clicked, this, [this] {
        QDesktopServices::openUrl(
            QUrl::fromLocalFile(m_controller.result().value("directory").toString()));
    });
    connect(m_start, &QPushButton::clicked, this, &VideoPage::start);
    connect(m_summarize, &QPushButton::clicked, this,
        [this] { emit summarizeRequested(m_controller.result().value("document").toString()); });
    connect(m_cancel, &QPushButton::clicked, &m_controller, &VideoController::stop);
    connect(&m_controller, &VideoController::changed, this, &VideoPage::refresh);
    connect(&m_controller, &VideoController::diagnostic, this, [this](const QString& text) {
        if (!text.isEmpty())
            m_log->appendPlainText(text);
    });
    refresh();
}

QString VideoPage::configuration() const { return m_config->text().trimmed(); }

void VideoPage::start()
{
    saveSettings();
    m_loadedDocument.clear();
    m_title->clear();
    m_preview->clear();
    m_log->clear();
    VideoOptions options;
    options.program = m_program;
    options.config = configuration();
    options.url = m_url->text().trimmed();
    options.cookies = static_cast<VideoOptions::Cookies>(m_cookies->currentData().toInt());
    if (options.cookies == VideoOptions::Cookies::Browser) {
        options.cookieSource = m_browser->currentData().toString();
        if (!m_profile->text().trimmed().isEmpty())
            options.cookieSource += ":" + m_profile->text().trimmed();
    } else if (options.cookies == VideoOptions::Cookies::File) {
        options.cookieSource = m_cookieFile->text().trimmed();
    }
    m_controller.start(options);
}

void VideoPage::refresh()
{
    const bool active = m_controller.active();
    m_fields->setEnabled(!active);
    m_start->setEnabled(!active);
    m_cancel->setEnabled(active && !m_controller.stopping());
    m_progress->setVisible(active);
    m_status->setText(m_controller.status());
    m_error->setText(m_controller.error());
    const auto result = m_controller.result();
    m_output->setText(result.value("directory").toString());
    m_open->setEnabled(!active && !m_output->text().isEmpty());
    m_summarize->setEnabled(!active && !result.value("document").toString().isEmpty());
    if (!active && !result.isEmpty())
        loadResult();
    emit activityChanged();
}

void VideoPage::loadResult()
{
    const auto path = m_controller.result().value("document").toString();
    if (path == m_loadedDocument)
        return;
    m_loadedDocument = path;
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly) || file.size() > 32 * 1024 * 1024) {
        m_error->setText(tr("Cannot read the subtitle document."));
        return;
    }
    QJsonParseError error;
    const auto document = QJsonDocument::fromJson(file.readAll(), &error).object();
    if (error.error != QJsonParseError::NoError || document.value("schema_version").toInt() != 1) {
        m_error->setText(tr("Cannot read the subtitle document."));
        return;
    }
    m_title->setText(document.value("source").toObject().value("title").toString());
    QStringList text;
    for (const auto& segment : document.value("transcript").toObject().value("segments").toArray())
        text << segment.toObject().value("text").toString();
    m_preview->setPlainText(text.join('\n'));
}

void VideoPage::saveSettings()
{
    QSettings saved;
    saved.setValue("videoConfig", configuration());
    saved.setValue("video/cookies", m_cookies->currentData());
    saved.setValue("video/browser", m_browser->currentData());
    saved.setValue("video/profile", m_profile->text());
    saved.setValue("video/cookieFile", m_cookieFile->text());
}
