#pragma once

#include <QString>
#include <optional>

std::optional<QString> readToken(const QString& path, QString* error);
