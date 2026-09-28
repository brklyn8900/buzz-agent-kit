use crate::config::{Backend, validate_name};
use anyhow::{Result, ensure};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use zeroize::Zeroizing;
mod file;
mod platform;
pub use file::FileStore;
pub use platform::{KeychainStore, SecretServiceStore};

/// A secret intentionally has no Debug, Display or serialization implementation.
pub struct Secret(Zeroizing<String>);
impl Secret {
    pub fn from_hex(mut value: Zeroizing<String>) -> Result<Self> {
        ensure!(
            value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid assistant key encoding"
        );
        value.make_ascii_lowercase();
        Ok(Self(value))
    }
    pub fn from_bytes(value: &[u8]) -> Result<Self> {
        let text = std::str::from_utf8(value)
            .map_err(|_| anyhow::anyhow!("invalid assistant key encoding"))?;
        Self::from_hex(Zeroizing::new(
            text.trim_end_matches(['\r', '\n']).to_owned(),
        ))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
}
/// put never replaces an existing entry. A failed put must not remove an entry
/// created by another writer. Inventory tracks public names only.
pub trait KeyStore {
    fn put(&self, name: &str, secret: &Secret) -> Result<()>;
    fn get(&self, name: &str) -> Result<Option<Secret>>;
    fn delete(&self, name: &str) -> Result<()>;
    fn list(&self) -> Result<Vec<String>>;
}
pub fn validate_service(service: &str) -> Result<()> {
    validate_name(service)?;
    ensure!(
        service != "buzz-desktop" && !service.starts_with("buzz-desktop-"),
        "access to desktop identity services is forbidden"
    );
    Ok(())
}

pub fn open(backend: Backend, home: &Path) -> Result<Box<dyn KeyStore>> {
    let root = home.join(".config/buzz-kit");
    match backend {
        Backend::File => Ok(Box::new(FileStore {
            directory: root.join("keys"),
        })),
        Backend::Keychain if cfg!(target_os = "macos") => Ok(Box::new(KeychainStore::new(
            "buzz-kit",
            root.join("identities/keychain"),
        )?)),
        Backend::SecretService if cfg!(target_os = "linux") => {
            let executable = find_program("secret-tool").ok_or_else(|| anyhow::anyhow!("secret-tool unavailable; install a secret-service client or explicitly choose the file keystore"))?;
            Ok(Box::new(SecretServiceStore::new(
                executable,
                root.join("identities/secret-service"),
            )?))
        }
        Backend::Auto => {
            if cfg!(target_os = "macos") {
                open(Backend::Keychain, home)
            } else if cfg!(target_os = "linux") {
                open(Backend::SecretService, home)
            } else {
                anyhow::bail!("unsupported platform")
            }
        }
        _ => anyhow::bail!(
            "selected keystore is unavailable on this platform; choose a supported backend explicitly"
        ),
    }
}

pub fn find_program(name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|p| p.join(name))
        .find(|p| {
            p.is_file()
                && p.metadata()
                    .is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
        })
}

struct Output {
    code: Option<i32>,
    stderr_empty: bool,
    stdout: Zeroizing<Vec<u8>>,
}
fn capture(command: &mut Command, input: Option<&[u8]>) -> Result<Output> {
    let mut child = command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| anyhow::anyhow!("cannot start keystore helper"))?;
    if let Some(bytes) = input {
        let result = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("keystore stdin unavailable"))?
            .write_all(bytes);
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("keystore stdin write failed");
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|_| anyhow::anyhow!("keystore helper failed"))?;
    let _stderr = Zeroizing::new(output.stderr);
    Ok(Output {
        code: output.status.code(),
        stderr_empty: _stderr.is_empty(),
        stdout: Zeroizing::new(output.stdout),
    })
}

struct Inventory {
    directory: PathBuf,
}
impl Inventory {
    fn register(&self, name: &str) -> Result<()> {
        validate_name(name)?;
        file::directory(&self.directory, true)?;
        let path = self.directory.join(name);
        if file::read(&path)?.is_none() {
            file::create(&path, b"")?;
        }
        Ok(())
    }
    fn list(&self) -> Result<Vec<String>> {
        if !file::directory(&self.directory, false)? {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in std::fs::read_dir(&self.directory)? {
            let name = entry?
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("invalid identity inventory entry"))?;
            // Interrupted atomic writes carry no secret, and aren't names.
            if name.contains(".tmp-") || name.starts_with(".lock-") {
                continue;
            }
            validate_name(&name)?;
            ensure!(
                file::read(&self.directory.join(&name))?.is_some_and(|b| b.is_empty()),
                "invalid identity inventory entry"
            );
            names.push(name);
        }
        names.sort();
        Ok(names)
    }
}
