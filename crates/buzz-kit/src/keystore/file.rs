use super::{KeyStore, Secret};
use crate::config::validate_name;
use anyhow::{Result, ensure};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

// getuid/geteuid are OS interfaces, not a new Rust dependency.
unsafe extern "C" {
    fn geteuid() -> u32;
}
fn owned(meta: &fs::Metadata) -> bool {
    meta.uid() == unsafe { geteuid() }
}

pub(super) fn directory(path: &Path, create: bool) -> Result<bool> {
    // Refuse symlinks in user-controlled ancestors too. macOS system aliases
    // such as /var -> /private/var are root-owned and outside kit control.
    for parent in path.ancestors().skip(1) {
        if let Ok(meta) = fs::symlink_metadata(parent) {
            ensure!(
                !(owned(&meta) && meta.file_type().is_symlink()),
                "symlinked keystore parent is forbidden"
            );
        }
    }

    match fs::symlink_metadata(path) {
        Ok(meta) => {
            ensure!(
                meta.is_dir()
                    && !meta.file_type().is_symlink()
                    && owned(&meta)
                    && meta.mode() & 0o777 == 0o700,
                "keystore directory must be owned by you, mode 0700, and not a symlink"
            );
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if !create {
                return Ok(false);
            }
            let parent = path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("invalid keystore directory"))?;
            match fs::symlink_metadata(parent) {
                Ok(m) => ensure!(
                    m.is_dir() && !m.file_type().is_symlink() && owned(&m) && m.mode() & 0o022 == 0,
                    "unsafe keystore parent directory"
                ),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    directory(parent, true)?;
                }
                Err(_) => anyhow::bail!("cannot inspect keystore directory"),
            }
            let result = fs::DirBuilder::new().mode(0o700).create(path);
            if let Err(e) = result {
                ensure!(
                    e.kind() == std::io::ErrorKind::AlreadyExists,
                    "cannot create keystore directory"
                );
            }
            directory(path, false)
        }
        Err(_) => anyhow::bail!("cannot inspect keystore directory"),
    }
}

pub(super) fn read(path: &Path) -> Result<Option<Zeroizing<Vec<u8>>>> {
    let before = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => anyhow::bail!("cannot inspect keystore file"),
    };
    ensure!(
        before.is_file()
            && !before.file_type().is_symlink()
            && owned(&before)
            && before.mode() & 0o777 == 0o600
            && before.nlink() == 1,
        "keystore file must be owned by you, mode 0600, and have no links"
    );
    let file = File::open(path).map_err(|_| anyhow::anyhow!("cannot open keystore file"))?;
    let after = file.metadata()?;
    ensure!(
        before.dev() == after.dev()
            && before.ino() == after.ino()
            && before.mode() == after.mode()
            && owned(&after)
            && after.nlink() == 1,
        "keystore file changed while opening"
    );
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(66)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow::anyhow!("cannot read keystore file"))?;
    ensure!(bytes.len() <= 64, "invalid keystore file length");
    Ok(Some(bytes))
}

/// Link a fully written, synced temporary file into place without overwriting.
pub(super) fn create(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random)?;
    let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let staging = path.with_extension(format!("tmp-{suffix}"));
    struct Remove(PathBuf);
    impl Drop for Remove {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staging)
        .map_err(|_| anyhow::anyhow!("cannot create keystore staging file"))?;
    let _cleanup = Remove(staging.clone());
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| anyhow::anyhow!("cannot write keystore staging file"))?;
    fs::hard_link(&staging, path)
        .map_err(|_| anyhow::anyhow!("keystore entry already exists or cannot be created"))?;
    // Remove the extra link before readers can accept the entry.
    fs::remove_file(&staging)?;
    Ok(())
}

pub struct FileStore {
    pub directory: PathBuf,
}
impl FileStore {
    fn path(&self, name: &str) -> Result<PathBuf> {
        validate_name(name)?;
        Ok(self.directory.join(format!("{name}.key")))
    }
}
impl KeyStore for FileStore {
    fn put(&self, name: &str, secret: &Secret) -> Result<()> {
        let path = self.path(name)?;
        directory(&self.directory, true)?;
        create(&path, secret.expose().as_bytes())
    }
    fn get(&self, name: &str) -> Result<Option<Secret>> {
        let path = self.path(name)?;
        if !directory(&self.directory, false)? {
            return Ok(None);
        }
        read(&path)?
            .map(|bytes| Secret::from_bytes(&bytes))
            .transpose()
    }
    fn delete(&self, name: &str) -> Result<()> {
        let path = self.path(name)?;
        if !directory(&self.directory, false)? {
            return Ok(());
        }
        if read(&path)?.is_some() {
            fs::remove_file(path).map_err(|_| anyhow::anyhow!("cannot delete keystore entry"))?;
        }
        Ok(())
    }
    fn list(&self) -> Result<Vec<String>> {
        if !directory(&self.directory, false)? {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            if let Some(name) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.strip_suffix(".key"))
            {
                validate_name(name)?;
                self.get(name)?;
                names.push(name.to_owned());
            }
        }
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn owner_check_rejects_root_owned_file_for_normal_user() {
        if unsafe { super::geteuid() } == 0 {
            return;
        }
        let m = std::fs::metadata("/etc/passwd").unwrap();
        assert!(!super::owned(&m));
    }
}
