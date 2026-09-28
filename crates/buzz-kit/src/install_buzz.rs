use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub const VERSION: &str = "0.5.25";
pub const URL: &str =
    "https://github.com/block/buzz/releases/download/desktop-v0.5.25/Buzz_0.5.25_amd64.deb";
pub const ARCHIVE_HASH: &str = "0990e351453d7eb31e50a498df8efced5ede57931bb414fc50cc9ca56d672293";
pub const BINARY_HASH: &str = "f30dc8744e49faa3b4e1703fbc26f6af9285b36dcca498b41eb3680b5b9a25e6";
pub struct Artifact<'a> {
    pub version: &'a str,
    pub archive_hash: &'a str,
    pub binary_hash: &'a str,
}
fn version(value: &str) -> bool {
    let p: Vec<_> = value.split('.').collect();
    p.len() == 3
        && p.iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}
pub fn supported(os: &str, arch: &str, libc: &str) -> Result<()> {
    ensure!(
        os == "linux" && arch == "x86_64",
        "automatic Buzz installation supports Linux x86_64 only; supply a compatible CLI with BUZZ_KIT_BUZZ_CLI"
    );
    let mut parts = libc.split_whitespace();
    ensure!(
        parts.next() == Some("glibc"),
        "Buzz CLI requires glibc >= 2.38"
    );
    let values: Vec<_> = parts.next().unwrap_or("").split('.').collect();
    ensure!(
        values.len() == 2 && parts.next().is_none(),
        "cannot identify glibc version"
    );
    let major = values[0].parse::<u32>()?;
    let minor = values[1].parse::<u32>()?;
    ensure!(
        (major, minor) >= (2, 38),
        "Buzz CLI requires glibc >= 2.38; supply a compatible CLI with BUZZ_KIT_BUZZ_CLI"
    );
    Ok(())
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn regular(path: &Path) -> Result<()> {
    let m = fs::symlink_metadata(path)?;
    ensure!(
        m.is_file() && !m.file_type().is_symlink(),
        "expected a regular file"
    );
    Ok(())
}
fn base(home: &Path) -> Result<PathBuf> {
    let mut p = home.to_owned();
    for part in ["", ".local", "share", "buzz-kit", "buzz-cli"] {
        if !part.is_empty() {
            p.push(part);
        }
        match fs::symlink_metadata(&p) {
            Ok(m) => ensure!(
                m.is_dir() && !m.file_type().is_symlink(),
                "unsafe Buzz installation directory"
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::create_dir(&p)?,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(p)
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn stage(base: &Path, name: &str) -> Result<Stage> {
    let p = base.join(name);
    fs::create_dir(&p).map_err(|_|anyhow::anyhow!("installation already running or stale staging directory; inspect the cache before retrying"))?;
    Ok(Stage(p))
}
fn current_safe(base: &Path) -> Result<()> {
    match fs::symlink_metadata(base.join("current")) {
        Ok(m) => {
            ensure!(
                m.file_type().is_symlink(),
                "refusing to replace foreign current file"
            );
            let link = fs::read_link(base.join("current"))?;
            let s = link.to_str().unwrap_or("");
            ensure!(
                s.strip_suffix("/buzz").is_some_and(version),
                "refusing foreign current symlink"
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
fn helper(mut command: Command) -> Result<Vec<u8>> {
    let out = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|_| anyhow::anyhow!("required system archive tool unavailable"))?;
    ensure!(out.status.success(), "archive tool failed");
    Ok(out.stdout)
}
fn verify_binary(path: &Path, expected: &str) -> Result<()> {
    regular(path)?;
    ensure!(
        hash(&fs::read(path)?) == expected,
        "Buzz binary checksum mismatch; existing file left untouched"
    );
    Ok(())
}
pub fn install_deb(home: &Path, deb: &Path, artifact: &Artifact<'_>) -> Result<PathBuf> {
    ensure!(version(artifact.version), "invalid Buzz version");
    regular(deb)?;
    ensure!(
        hash(&fs::read(deb)?) == artifact.archive_hash,
        "Buzz package checksum mismatch"
    );
    let base = base(home)?;
    let staging = stage(&base, ".install-lock")?;
    current_safe(&base)?;
    let destination = base.join(artifact.version);
    let binary = destination.join("buzz");
    match fs::symlink_metadata(&destination) {
        Ok(m) => {
            ensure!(
                m.is_dir() && !m.file_type().is_symlink(),
                "unsafe cached Buzz version"
            );
            verify_binary(&binary, artifact.binary_hash)?;
            ensure!(
                fs::metadata(&binary)?.permissions().mode() & 0o111 != 0,
                "cached Buzz CLI is not executable"
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut command = Command::new("ar");
            command.arg("t").arg(deb);
            let members = String::from_utf8(helper(command)?)?;
            ensure!(
                members.lines().filter(|s| *s == "data.tar.gz").count() == 1,
                "expected one data.tar.gz in Buzz package"
            );
            let data = staging.0.join("data.tar.gz");
            let out = fs::File::create(&data)?;
            let status = Command::new("ar")
                .arg("p")
                .arg(deb)
                .arg("data.tar.gz")
                .stdin(Stdio::null())
                .stdout(out)
                .stderr(Stdio::null())
                .status()?;
            ensure!(status.success(), "cannot extract Buzz data archive");
            let mut command = Command::new("tar");
            command.arg("-tf").arg(&data);
            let listing = String::from_utf8(helper(command)?)?;
            let matches: Vec<_> = listing
                .lines()
                .filter(|s| *s == "usr/bin/buzz" || *s == "./usr/bin/buzz")
                .collect();
            ensure!(matches.len() == 1, "expected exactly one Buzz executable");
            let member = matches[0];
            let mut command = Command::new("tar");
            command.arg("-tvf").arg(&data).arg(member);
            let details = String::from_utf8(helper(command)?)?;
            ensure!(
                details.lines().count() == 1 && details.starts_with('-'),
                "Buzz archive member must be a regular file"
            );
            let mut command = Command::new("tar");
            command.arg("-xOf").arg(&data).arg(member);
            let bytes = helper(command)?;
            ensure!(
                hash(&bytes) == artifact.binary_hash,
                "extracted Buzz binary checksum mismatch"
            );
            let install = staging.0.join("install");
            fs::create_dir(&install)?;
            fs::write(install.join("buzz"), bytes)?;
            fs::set_permissions(install.join("buzz"), fs::Permissions::from_mode(0o755))?;
            fs::rename(install, &destination)?;
        }
        Err(e) => return Err(e.into()),
    }
    symlink(
        format!("{}/buzz", artifact.version),
        staging.0.join("current"),
    )?;
    fs::rename(staging.0.join("current"), base.join("current"))?;
    Ok(binary)
}
pub fn run(home: &Path) -> Result<PathBuf> {
    if cfg!(target_os = "macos") {
        let p = PathBuf::from("/Applications/Buzz.app/Contents/MacOS/buzz");
        ensure!(
            p.is_file() && fs::metadata(&p)?.permissions().mode() & 0o111 != 0,
            "install Buzz Desktop in /Applications, then run doctor"
        );
        return Ok(p);
    }
    // Check architecture before even probing helpers or touching the cache.
    ensure!(
        std::env::consts::OS == "linux" && std::env::consts::ARCH == "x86_64",
        "automatic Buzz installation supports Linux x86_64 only; supply BUZZ_KIT_BUZZ_CLI"
    );
    let out = Command::new("getconf")
        .arg("GNU_LIBC_VERSION")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()?;
    ensure!(
        out.status.success(),
        "cannot identify glibc; Buzz requires glibc >= 2.38"
    );
    supported(
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::str::from_utf8(&out.stdout)?,
    )?;
    for name in ["curl", "ar", "tar"] {
        ensure!(
            crate::keystore::find_program(name).is_some(),
            "required system tool {name} is missing"
        );
    }
    let base = base(home)?;
    current_safe(&base)?;
    let download = stage(&base, ".download-lock")?;
    let deb = download.0.join("buzz.deb");
    let status = Command::new("curl")
        .args([
            "--fail",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--silent",
            "--show-error",
            "--output",
        ])
        .arg(&deb)
        .arg(URL)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    ensure!(status.success(), "Buzz download failed");
    install_deb(
        home,
        &deb,
        &Artifact {
            version: VERSION,
            archive_hash: ARCHIVE_HASH,
            binary_hash: BINARY_HASH,
        },
    )
}
