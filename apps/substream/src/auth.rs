use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

use anyhow::{Context, Result, ensure};

/// Stored only in an explicitly selected private file; never in a URL or log.
#[derive(Clone)]
pub struct Token(String);

impl Token {
    pub fn read(path: &Path) -> Result<Self> {
        let metadata = std::fs::symlink_metadata(path).context("read token metadata")?;
        ensure!(
            metadata.file_type().is_file(),
            "token must be a regular file, not a symlink"
        );
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "token file permissions must be 0600"
        );
        ensure!(metadata.len() <= 128, "invalid token file");
        let token = std::fs::read_to_string(path)?.trim().to_owned();
        ensure!(
            token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()),
            "token must contain 64 hex characters"
        );
        Ok(Self(token))
    }

    pub fn matches(&self, other: &str) -> bool {
        if other.len() != self.0.len() {
            return false;
        }
        self.0
            .bytes()
            .zip(other.bytes())
            .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
            == 0
    }
}

pub fn create_token(path: &Path) -> Result<()> {
    let mut bytes = [0_u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    let value: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .context("create private token file (will not overwrite an existing file)")?;
    writeln!(file, "{value}")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_private_tokens_and_does_not_overwrite_them() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("token");
        create_token(&path).unwrap();
        let original = std::fs::read_to_string(&path).unwrap();
        assert!(create_token(&path).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o077,
            0
        );
        assert!(Token::read(&path).is_ok());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Token::read(&path).is_err());
    }
}
