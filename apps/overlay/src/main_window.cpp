#include "main_window.h"
#include "caption_model.h"
#include "kde_window.h"
#include "token_file.h"

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QCloseEvent>
#include <QComboBox>
#include <QDir>
#include <QFileDialog>
#include <QFileInfo>
#include <QFormLayout>
#include <QGroupBox>
#include <QJsonArray>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QPushButton>
#include <QScreen>
#include <QSettings>
#include <QSpinBox>
#include <QStandardPaths>
#include <QTabWidget>
#include <QVBoxLayout>
#include <algorithm>

namespace {
QLabel* label(const QString& text = { })
{
    auto* result = new QLabel(text);
    result->setTextFormat(Qt::PlainText);
    result->setWordWrap(true);
    result->setTextInteractionFlags(Qt::TextSelectableByMouse);
    return result;
}

QSpinBox* number(int minimum, int maximum, int value, const QString& suffix = { })
{
    auto* result = new QSpinBox;
    result->setRange(minimum, maximum);
    result->setValue(value);
    result->setSuffix(suffix);
    return result;
}

QWidget* filePicker(QLineEdit* edit, const QString& filter, bool newFile = false)
{
    auto* row = new QWidget;
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    auto* browse = new QPushButton(QStringLiteral("选择…"));
    layout->addWidget(edit, 1);
    layout->addWidget(browse);
    QObject::connect(browse, &QPushButton::clicked, row, [edit, filter, newFile, row] {
        const auto path = newFile
            ? QFileDialog::getSaveFileName(row, QStringLiteral("选择令牌位置"), edit->text(),
                  filter, nullptr, QFileDialog::DontConfirmOverwrite)
            : QFileDialog::getOpenFileName(row, QStringLiteral("选择文件"), edit->text(), filter);
        if (!path.isEmpty())
            edit->setText(path);
    });
    return row;
}

QString defaultProgram()
{
    for (const auto& path : { QCoreApplication::applicationDirPath() + "/substream",
             QCoreApplication::applicationDirPath() + "/../../target/release/substream" }) {
        if (QFileInfo(path).isExecutable())
            return QFileInfo(path).absoluteFilePath();
    }
    const auto path = QStandardPaths::findExecutable(QStringLiteral("substream"));
    return path.isEmpty() ? QStringLiteral("substream") : path;
}
}

MainWindow::MainWindow(bool kdeWayland)
{
    setWindowTitle(QStringLiteral("Substream · 实时字幕"));
    resize(1060, 720);
    QSettings saved;
    auto* body = new QWidget;
    setCentralWidget(body);
    auto* layout = new QVBoxLayout(body);
    layout->setContentsMargins(24, 20, 24, 20);
    layout->setSpacing(16);
    auto* heading = label(QStringLiteral("实时字幕"));
    auto headingFont = heading->font();
    headingFont.setPointSize(22);
    headingFont.setBold(true);
    heading->setFont(headingFont);
    layout->addWidget(heading);
    auto* columns = new QHBoxLayout;
    columns->setSpacing(20);
    layout->addLayout(columns, 1);
    m_settings = new QTabWidget;
    columns->addWidget(m_settings, 5);

    auto* capture = new QWidget;
    auto* captureLayout = new QVBoxLayout(capture);
    captureLayout->setContentsMargins(16, 20, 16, 16);
    captureLayout->setSpacing(14);
    m_input = new QComboBox;
    m_input->addItems({ QStringLiteral("系统音频"), QStringLiteral("浏览器标签页") });
    m_input->setCurrentIndex(saved.value("input", 0).toInt() == 1 ? 1 : 0);
    captureLayout->addWidget(label(QStringLiteral("捕获方式")));
    captureLayout->addWidget(m_input);
    auto* deviceFields = new QWidget;
    auto* deviceLayout = new QVBoxLayout(deviceFields);
    deviceLayout->setContentsMargins(0, 0, 0, 0);
    deviceLayout->addWidget(label(QStringLiteral("音频设备")));
    m_device = new QComboBox;
    m_device->setMinimumContentsLength(20);
    m_device->setSizeAdjustPolicy(QComboBox::AdjustToMinimumContentsLengthWithIcon);
    auto* deviceRow = new QHBoxLayout;
    deviceRow->addWidget(m_device, 1);
    auto* refresh = new QPushButton(QStringLiteral("刷新"));
    deviceRow->addWidget(refresh);
    deviceLayout->addLayout(deviceRow);
    captureLayout->addWidget(deviceFields);
    auto* browserFields = new QWidget;
    auto* browserLayout = new QVBoxLayout(browserFields);
    browserLayout->setContentsMargins(0, 0, 0, 0);
    m_extension = new QLineEdit(saved.value("extension").toString());
    m_extension->setPlaceholderText(QStringLiteral("扩展管理页中的 32 位 ID"));
    browserLayout->addWidget(label(QStringLiteral("Chromium 扩展 ID")));
    browserLayout->addWidget(m_extension);
    const auto tokenPath
        = QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/browser.token";
    m_token = new QLineEdit(saved.value("tokenFile", tokenPath).toString());
    browserLayout->addWidget(label(QStringLiteral("配对令牌文件")));
    browserLayout->addWidget(filePicker(m_token, QStringLiteral("所有文件 (*)"), true));
    auto* copy = new QPushButton(QStringLiteral("复制配对令牌"));
    captureLayout->addWidget(browserFields);
    captureLayout->addStretch();
    const auto inputChanged = [this, deviceFields, browserFields, copy] {
        const bool browser = m_input->currentIndex() == 1;
        deviceFields->setVisible(!browser);
        browserFields->setVisible(browser);
        copy->setVisible(browser);
    };
    connect(m_input, &QComboBox::currentIndexChanged, this, inputChanged);
    inputChanged();
    connect(refresh, &QPushButton::clicked, &m_devices, &AudioDevices::refresh);
    connect(copy, &QPushButton::clicked, this, [this] {
        QString error;
        const auto token = readToken(m_token->text(), &error);
        if (token)
            QApplication::clipboard()->setText(*token);
        else
            m_error->setText(error + QStringLiteral("\n首次使用时，请先启动服务。"));
    });
    m_settings->addTab(capture, QStringLiteral("音频源"));

    auto* model = new QWidget;
    auto* modelLayout = new QVBoxLayout(model);
    modelLayout->setContentsMargins(16, 20, 16, 16);
    modelLayout->setSpacing(14);
    m_config = new QLineEdit(saved.value("config").toString());
    m_config->setPlaceholderText(QStringLiteral("模型配置（.toml）"));
    m_program = new QLineEdit(saved.value("program", defaultProgram()).toString());
    m_threads = number(0, 64, saved.value("threads", 0).toInt());
    m_threads->setSpecialValueText(QStringLiteral("使用模型配置"));
    modelLayout->addWidget(label(QStringLiteral("模型配置文件")));
    modelLayout->addWidget(filePicker(m_config, QStringLiteral("模型配置 (*.toml);;所有文件 (*)")));
    modelLayout->addWidget(label(QStringLiteral("CPU 推理线程数")));
    modelLayout->addWidget(m_threads);
    modelLayout->addWidget(label(QStringLiteral("识别程序")));
    modelLayout->addWidget(filePicker(m_program, QStringLiteral("所有文件 (*)")));
    modelLayout->addStretch();
    m_settings->addTab(model, QStringLiteral("模型"));

    auto* display = new QWidget;
    auto* displayLayout = new QVBoxLayout(display);
    displayLayout->setContentsMargins(16, 20, 16, 16);
    displayLayout->setSpacing(14);
    m_showOverlay = new QCheckBox(QStringLiteral("显示桌面悬浮字幕"));
    m_showOverlay->setChecked(kdeWayland && saved.value("overlay", true).toBool());
    m_showOverlay->setEnabled(kdeWayland);
    displayLayout->addWidget(m_showOverlay);
    if (!kdeWayland)
        displayLayout->addWidget(label(QStringLiteral("悬浮字幕仅支持 KDE Wayland。")));
    auto* form = new QFormLayout;
    m_screen = new QComboBox;
    refreshScreens();
    m_screen->setCurrentIndex(qMax(0, m_screen->findData(saved.value("screen", ""))));
    m_font = number(14, 72, saved.value("font", 30).toInt(), QStringLiteral(" 像素"));
    m_width = number(240, 3840, saved.value("width", 900).toInt(), QStringLiteral(" 像素"));
    m_margin = number(0, 1000, saved.value("margin", 64).toInt(), QStringLiteral(" 像素"));
    m_hold = number(1, 60, saved.value("holdSeconds", 5).toInt(), QStringLiteral(" 秒"));
    form->addRow(QStringLiteral("显示屏幕"), m_screen);
    form->addRow(QStringLiteral("字幕字号"), m_font);
    form->addRow(QStringLiteral("最大宽度"), m_width);
    form->addRow(QStringLiteral("底部距离"), m_margin);
    form->addRow(QStringLiteral("无更新后隐藏"), m_hold);
    displayLayout->addLayout(form);
    displayLayout->addStretch();
    m_settings->addTab(display, QStringLiteral("字幕显示"));

    auto* current = new QGroupBox(QStringLiteral("当前会话"));
    columns->addWidget(current, 5);
    auto* currentLayout = new QVBoxLayout(current);
    currentLayout->setContentsMargins(16, 20, 16, 16);
    currentLayout->setSpacing(12);
    m_status = label();
    auto statusFont = m_status->font();
    statusFont.setBold(true);
    statusFont.setPointSize(statusFont.pointSize() + 2);
    m_status->setFont(statusFont);
    currentLayout->addWidget(m_status);
    auto* details = new QFormLayout;
    details->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
    m_source = label(QStringLiteral("未捕获"));
    m_model = label(QStringLiteral("未加载"));
    m_source->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Minimum);
    m_model->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Minimum);
    m_duration = label(QStringLiteral("0:00 · 16 kHz / 单声道"));
    details->addRow(QStringLiteral("音频源"), m_source);
    details->addRow(QStringLiteral("识别模型"), m_model);
    details->addRow(QStringLiteral("已接收音频"), m_duration);
    currentLayout->addLayout(details);
    m_level = new QProgressBar;
    m_level->setRange(0, 100);
    m_level->setValue(0);
    m_level->setFormat(QStringLiteral("输入电平 %p%"));
    currentLayout->addWidget(m_level);
    m_preview = new QPlainTextEdit;
    m_preview->setReadOnly(true);
    m_preview->setPlaceholderText(QStringLiteral("等待字幕"));
    m_preview->setMinimumHeight(100);
    currentLayout->addWidget(m_preview, 1);
    m_error = label();
    m_error->setStyleSheet(QStringLiteral("color: #c34235;"));
    currentLayout->addWidget(m_error);
    m_log = new QPlainTextEdit;
    m_log->setReadOnly(true);
    m_log->setMaximumBlockCount(200);
    m_log->setPlaceholderText(QStringLiteral("运行消息"));
    m_log->setMaximumHeight(110);
    m_log->hide();
    auto* logToggle = new QPushButton(QStringLiteral("运行日志"));
    logToggle->setCheckable(true);
    logToggle->setFlat(true);
    logToggle->setSizePolicy(QSizePolicy::Fixed, QSizePolicy::Fixed);
    connect(logToggle, &QPushButton::toggled, m_log, &QWidget::setVisible);
    currentLayout->addWidget(logToggle, 0, Qt::AlignRight);
    currentLayout->addWidget(m_log);

    auto* actions = new QHBoxLayout;
    actions->addStretch();
    m_start = new QPushButton(QStringLiteral("开始字幕"));
    m_start->setDefault(true);
    m_start->setMinimumHeight(36);
    m_stop = new QPushButton(QStringLiteral("停止"));
    m_stop->setMinimumHeight(36);
    actions->addWidget(copy);
    actions->addWidget(m_start);
    actions->addWidget(m_stop);
    layout->addLayout(actions);
    connect(m_start, &QPushButton::clicked, this, &MainWindow::start);
    connect(m_stop, &QPushButton::clicked, &m_session, &SessionController::stop);
    connect(&m_session, &SessionController::changed, this, &MainWindow::refreshState);
    connect(&m_session, &SessionController::diagnostic, this, [this](const QString& message) {
        if (!message.isEmpty())
            m_log->appendPlainText(message);
    });
    connect(&m_session, &SessionController::eventReceived, this, [this](const QJsonObject& event) {
        QString error;
        if (m_captions && !m_captions->apply(event, &error)) {
            m_log->appendPlainText(error);
            m_session.stop();
        }
    });
    connect(&m_devices, &AudioDevices::changed, this, &MainWindow::refreshDevices);
    connect(&m_devices, &AudioDevices::failed, this,
        [this](const QString& error) { m_log->appendPlainText(error); });
    connect(qApp, &QGuiApplication::screenAdded, this, &MainWindow::refreshScreens);
    connect(qApp, &QGuiApplication::screenRemoved, this, &MainWindow::refreshScreens);
    m_deviceRefresh.setInterval(3000);
    connect(&m_deviceRefresh, &QTimer::timeout, &m_devices, &AudioDevices::refresh);
    m_savedDevice = saved.value("device").toString();
    m_devices.refresh();
    connect(m_input, &QComboBox::currentIndexChanged, this, &MainWindow::refreshState);
    refreshState();
}

MainWindow::~MainWindow()
{
    disconnect(&m_session, nullptr, this, nullptr);
    disconnect(&m_devices, nullptr, this, nullptr);
}

void MainWindow::refreshDevices()
{
    const auto selected = m_device->currentData().toString();
    m_device->clear();
    for (const auto& device : m_devices.devices())
        m_device->addItem(device.label, device.name);
    const auto preferred = selected.isEmpty() ? m_savedDevice : selected;
    const int index = m_device->findData(preferred);
    // A disappeared selection must not silently change to another device.
    if (!preferred.isEmpty())
        m_device->setCurrentIndex(index);
    const auto serial = m_session.deviceSerial();
    if (!serial.isEmpty()) {
        const auto& devices = m_devices.devices();
        if (std::none_of(
                devices.begin(), devices.end(), [&](const auto& d) { return d.serial == serial; }))
            m_session.inputUnavailable();
    }
}

void MainWindow::refreshScreens()
{
    if (!m_screen)
        return;
    const auto selected = m_screen->currentData();
    m_screen->clear();
    m_screen->addItem(QStringLiteral("主屏幕"), QString());
    for (auto* screen : QGuiApplication::screens())
        m_screen->addItem(screen->name(), screen->name());
    m_screen->setCurrentIndex(qMax(0, m_screen->findData(selected)));
}

void MainWindow::start()
{
    if (m_session.active())
        return;
    saveSettings();
    m_error->clear();
    m_log->clear();
    m_overlay.reset();
    m_captions = std::make_unique<CaptionModel>(m_hold->value() * 1000);
    m_preview->clear();
    connect(m_captions.get(), &CaptionModel::changed, this,
        [this] { m_preview->setPlainText(m_captions->text()); });
    if (m_showOverlay->isChecked()) {
        m_overlay = std::make_unique<KdeWindow>(
            OverlayOptions { m_screen->currentData().toString(), m_width->value(), m_font->value(),
                m_margin->value() },
            *m_captions);
        QString error;
        if (!m_overlay->initialize(&error)) {
            m_overlay.reset();
            m_error->setText(error);
            return;
        }
    }
    LaunchOptions options;
    options.input = m_input->currentIndex() == 0 ? LaunchOptions::Input::Device
                                                 : LaunchOptions::Input::Browser;
    options.program = m_program->text().trimmed();
    options.config = m_config->text().trimmed();
    options.threads = m_threads->value();
    options.tokenFile = m_token->text().trimmed();
    options.browserOrigin = "chrome-extension://" + m_extension->text().trimmed();
    for (const auto& device : m_devices.devices()) {
        if (device.name == m_device->currentData().toString())
            options.device = device;
    }
    m_session.start(options);
}

void MainWindow::refreshState()
{
    const bool active = m_session.active();
    for (int i = 0; i < m_settings->count(); ++i)
        m_settings->widget(i)->setEnabled(!active);
    m_start->setEnabled(!active);
    m_stop->setEnabled(active && !m_session.stopping());
    m_stop->setText(
        m_input->currentIndex() == 1 ? QStringLiteral("停止服务") : QStringLiteral("停止"));
    m_status->setText(m_session.status());
    m_error->setText(m_session.error());
    m_source->setText(active && !m_session.source().isEmpty()
            ? m_session.source().section('\n', 0, 0)
            : QStringLiteral("未捕获"));
    m_source->setToolTip(active ? m_session.source() : QString());
    const auto backend = m_session.backend();
    if (active && !backend.isEmpty()) {
        QStringList languages;
        for (const auto& value : backend.value("languages").toArray())
            languages << value.toString();
        const QFileInfo modelFile(backend.value("model").toString());
        const auto modelName = modelFile.dir().dirName();
        m_model->setText(modelName
            + QStringLiteral("\n%1 · %2 个线程")
                .arg(languages.join(" / "))
                .arg(backend.value("threads").toInt()));
        m_model->setToolTip(backend.value("name").toString() + "\n" + modelFile.filePath());
    } else {
        m_model->setText(QStringLiteral("未加载"));
        m_model->setToolTip({ });
    }
    const auto seconds = m_session.samples() / 16000;
    m_duration->setText(QStringLiteral("%1:%2 · 16 kHz / 单声道")
            .arg(seconds / 60)
            .arg(seconds % 60, 2, 10, QChar('0')));
    m_level->setVisible(m_input->currentIndex() == 0);
    m_level->setValue(m_session.level());
    if (active && !m_deviceRefresh.isActive())
        m_deviceRefresh.start();
    if (!active) {
        m_deviceRefresh.stop();
        if (!m_session.error().isEmpty() && m_captions)
            m_captions->reset();
        if (m_closing)
            QTimer::singleShot(0, this, &QWidget::close);
    }
}

void MainWindow::saveSettings()
{
    QSettings saved;
    saved.setValue("input", m_input->currentIndex());
    saved.setValue("device", m_device->currentData());
    saved.setValue("program", m_program->text());
    saved.setValue("config", m_config->text());
    saved.setValue("threads", m_threads->value());
    saved.setValue("extension", m_extension->text());
    saved.setValue("tokenFile", m_token->text());
    saved.setValue("overlay", m_showOverlay->isChecked());
    saved.setValue("screen", m_screen->currentData());
    saved.setValue("font", m_font->value());
    saved.setValue("width", m_width->value());
    saved.setValue("margin", m_margin->value());
    saved.setValue("holdSeconds", m_hold->value());
}

void MainWindow::closeEvent(QCloseEvent* event)
{
    saveSettings();
    if (m_session.active()) {
        m_closing = true;
        m_session.stop();
        event->ignore();
        return;
    }
    m_overlay.reset();
    event->accept();
}
