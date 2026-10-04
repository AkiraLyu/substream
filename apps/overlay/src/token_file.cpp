#include "token_file.h"

#include <QCoreApplication>
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
        *error = QCoreApplication::translate("TokenFile",
            "The token must be a regular file readable and writable only by its owner (permissions "
            "0600).");
        return std::nullopt;
    }
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        *error = QCoreApplication::translate("TokenFile", "Cannot read the token file.");
        return std::nullopt;
    }
    const auto token = QString::fromUtf8(file.read(129)).trimmed();
    static const QRegularExpression pattern(QStringLiteral("^[a-fA-F0-9]{64}$"));
    if (!pattern.match(token).hasMatch()) {
        *error = QCoreApplication::translate(
            "TokenFile", "The token must contain 64 hexadecimal characters.");
        return std::nullopt;
    }
    return token;
}
