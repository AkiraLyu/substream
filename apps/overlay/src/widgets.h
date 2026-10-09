#pragma once

#include <QString>

class QLabel;
class QLineEdit;
class QWidget;

namespace Widgets {
QLabel* label(const QString& text = { });
QWidget* filePicker(QLineEdit* edit, const QString& filter, bool newFile = false);
}
