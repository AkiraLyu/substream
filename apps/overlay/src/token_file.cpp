#include "token_file.h"

#include <QFile>
#include <QFileInfo>
#include <QRegularExpression>

std::optional<QString> readToken(const QString& path, QString* error)
{
    const QFileInfo info(path);
    constexpr auto sharedPermissions = QFile::ReadGroup | QFile::WriteGroup | QFile::ExeGroup
        | QFile::ReadOther | QFile::WriteOther | QFile::ExeOther;
    if (!info.isFile() || info.isSymLink() || (info.permissions() & sharedPermissions)
        || info.size() > 128) {
        *error = QStringLiteral("令牌必须是仅当前用户可读写的普通文件（权限 0600）。");
        return std::nullopt;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        *error = QStringLiteral("无法读取令牌文件。");
        return std::nullopt;
    }
    const auto token = QString::fromUtf8(file.read(129)).trimmed();
    static const QRegularExpression pattern(QStringLiteral("^[a-fA-F0-9]{64}$"));
    if (!pattern.match(token).hasMatch()) {
        *error = QStringLiteral("令牌应包含 64 位十六进制字符。");
        return std::nullopt;
    }
    return token;
}
