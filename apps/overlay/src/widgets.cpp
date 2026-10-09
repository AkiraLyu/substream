#include "widgets.h"

#include <QCoreApplication>
#include <QFileDialog>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>

namespace Widgets {
QLabel* label(const QString& text)
{
    auto* result = new QLabel(text);
    result->setTextFormat(Qt::PlainText);
    result->setWordWrap(true);
    result->setTextInteractionFlags(Qt::TextSelectableByMouse);
    return result;
}

QWidget* filePicker(QLineEdit* edit, const QString& filter, bool newFile)
{
    auto* row = new QWidget;
    auto* layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 0);
    auto* browse = new QPushButton(QCoreApplication::translate("FilePicker", "Browse…"));
    layout->addWidget(edit, 1);
    layout->addWidget(browse);
    QObject::connect(browse, &QPushButton::clicked, row, [edit, filter, newFile, row] {
        const auto path = newFile
            ? QFileDialog::getSaveFileName(row,
                  QCoreApplication::translate("FilePicker", "Choose a token file location"),
                  edit->text(), filter, nullptr, QFileDialog::DontConfirmOverwrite)
            : QFileDialog::getOpenFileName(row,
                  QCoreApplication::translate("FilePicker", "Choose a file"), edit->text(), filter);
        if (!path.isEmpty())
            edit->setText(path);
    });
    return row;
}

}
