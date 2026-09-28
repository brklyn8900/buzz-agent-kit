use crate::host::Candidate;
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Default)]
pub struct Options {
    pub to: Option<String>,
    pub unpin: bool,
    pub no_prune: bool,
}
#[derive(Eq, PartialEq, Debug)]
struct Version {
    core: [u64; 3],
    pre: Vec<String>,
}
impl Version {
    fn parse(s: &str) -> Result<Self> {
        ensure!(s.len() <= 128, "invalid semantic version");
        let (core, pre) = s.split_once('-').map_or((s, None), |(a, b)| (a, Some(b)));
        let numbers: Vec<_> = core.split('.').collect();
        ensure!(numbers.len() == 3, "invalid semantic version");
        let mut parsed = [0; 3];
        for (i, n) in numbers.into_iter().enumerate() {
            ensure!(
                !n.is_empty()
                    && n.bytes().all(|b| b.is_ascii_digit())
                    && (n == "0" || !n.starts_with('0')),
                "invalid semantic version"
            );
            parsed[i] = n
                .parse()
                .map_err(|_| anyhow::anyhow!("version component too large"))?;
        }
        let pre = match pre {
            None => vec![],
            Some(pre) => {
                let parts: Vec<_> = pre.split('.').map(String::from).collect();
                ensure!(
                    parts.iter().all(|p| !p.is_empty()
                        && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                        && !(p.len() > 1
                            && p.starts_with('0')
                            && p.bytes().all(|b| b.is_ascii_digit()))),
                    "invalid prerelease version"
                );
                parts
            }
        };
        Ok(Self { core: parsed, pre })
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let core = self.core.cmp(&other.core);
        if core != Ordering::Equal {
            return core;
        }
        if self.pre.is_empty() || other.pre.is_empty() {
            return self.pre.is_empty().cmp(&other.pre.is_empty());
        }
        for (a, b) in self.pre.iter().zip(&other.pre) {
            let numeric = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
            let order = match (numeric(a), numeric(b)) {
                (true, true) => a.len().cmp(&b.len()).then_with(|| a.cmp(b)),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => natural_prerelease(a, b),
            };
            if order != Ordering::Equal {
                return order;
            }
        }
        self.pre.len().cmp(&other.pre.len())
    }
}
// SemVer compares nonnumeric identifiers lexically: rc2 > rc10. Use rc.2/rc.10
// for numeric prerelease ordering. Do not invent a different version scheme.
fn natural_prerelease(a: &str, b: &str) -> Ordering {
    a.cmp(b)
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
fn tag(value: &str) -> Result<Version> {
    Version::parse(
        value
            .strip_prefix("bin-v")
            .ok_or_else(|| anyhow::anyhow!("binary tag must start with bin-v"))?,
    )
}
const TARGETS: [&str; 4] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
];
pub struct Release {
    pub tag: String,
    pub hashes: BTreeMap<String, String>,
}
impl Release {
    pub fn read(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .map_err(|_| anyhow::anyhow!("release checksum file unavailable"))?;
        let mut bin_tag = None;
        let mut hashes = BTreeMap::new();
        for line in text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            let fields: Vec<_> = line.split_whitespace().collect();
            ensure!(fields.len() == 2, "malformed checksum line");
            if fields[0] == "bin_tag:" {
                ensure!(bin_tag.is_none(), "duplicate binary tag");
                tag(fields[1])?;
                bin_tag = Some(fields[1].to_owned());
            } else {
                ensure!(hex_hash(fields[0]), "malformed archive checksum");
                let target = fields[1]
                    .strip_prefix("buzz-kit-")
                    .and_then(|s| s.strip_suffix(".tar.gz"))
                    .ok_or_else(|| anyhow::anyhow!("unexpected release asset"))?;
                ensure!(
                    TARGETS.contains(&target)
                        && hashes.insert(target.into(), fields[0].into()).is_none(),
                    "unexpected or duplicate release target"
                );
            }
        }
        ensure!(
            hashes.len() == 4,
            "release checksum file must contain four targets"
        );
        Ok(Self {
            tag: bin_tag
                .ok_or_else(|| anyhow::anyhow!("no released binary tag in this checkout"))?,
            hashes,
        })
    }
}
fn hex_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn data_dir(home: &Path) -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("buzz-kit")
}
pub fn highest(candidates: Vec<Candidate>) -> Result<Candidate> {
    let mut versions = Vec::new();
    for candidate in candidates {
        versions.push((Version::parse(&candidate.version)?, candidate));
    }
    versions.into_iter().max_by(|(a,_),(b,_)|a.cmp(b)).map(|(_,c)|c).ok_or_else(||anyhow::anyhow!("no enabled installed kit plugin with a verified location; reinstall a released plugin tag"))
}
fn regular(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(m) => {
            ensure!(
                m.is_file() && !m.file_type().is_symlink(),
                "unsafe cache metadata file"
            );
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => anyhow::bail!("cannot inspect cache metadata"),
    }
}
fn text(path: &Path) -> Result<String> {
    ensure!(regular(path)?, "cache metadata missing");
    fs::read_to_string(path)
        .map(|s| s.trim().to_owned())
        .map_err(|_| anyhow::anyhow!("cannot read cache metadata"))
}
fn pin(data: &Path) -> Result<Option<String>> {
    if !regular(&data.join("pin"))? {
        return Ok(None);
    }
    let value = text(&data.join("pin"))?;
    tag(&value)?;
    Ok(Some(value))
}
pub fn current(data: &Path) -> Result<Option<String>> {
    match fs::read_link(data.join("current")) {
        Ok(link) => {
            let s = link
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid current target"))?;
            let name = s
                .strip_prefix("bin/")
                .ok_or_else(|| anyhow::anyhow!("current is outside binary cache"))?;
            ensure!(
                !name.contains('/') && !name.contains(".."),
                "unsafe current target"
            );
            if !name.starts_with("dev-") {
                tag(name)?;
            }
            Ok(Some(name.into()))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => anyhow::bail!("current is not a readable managed symlink"),
    }
}
fn verify(data: &Path, name: &str, release: Option<&Release>) -> Result<()> {
    tag(name)?;
    let dir = data.join("bin").join(name);
    let metadata = fs::symlink_metadata(&dir)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "cached version is not a regular directory"
    );
    let hash = text(&dir.join("binary.sha256"))?;
    ensure!(
        hex_hash(&hash) && regular(&dir.join("buzz-kit"))?,
        "invalid cached executable metadata"
    );
    ensure!(
        fs::metadata(dir.join("buzz-kit"))?.permissions().mode() & 0o111 != 0,
        "cached binary is not executable"
    );
    let bytes = fs::read(dir.join("buzz-kit"))?;
    ensure!(
        Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            == hash,
        "cached binary integrity check failed"
    );
    let archive = text(&dir.join("archive.sha256"))?;
    let target = text(&dir.join("target"))?;
    ensure!(
        hex_hash(&archive) && TARGETS.contains(&target.as_str()),
        "invalid cached archive metadata"
    );
    if let Some(release) = release {
        ensure!(
            release.hashes.get(&target) == Some(&archive),
            "cached archive does not match installed plugin checksum"
        );
    }
    Ok(())
}
struct Remove(PathBuf);
impl Drop for Remove {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn temporary(data: &Path) -> Result<PathBuf> {
    let mut bytes = [0u8; 12];
    getrandom::fill(&mut bytes)?;
    Ok(data.join(format!(
        ".update-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )))
}
fn switch(data: &Path, target: &str) -> Result<()> {
    let tmp = Remove(temporary(data)?);
    symlink(format!("bin/{target}"), &tmp.0)?;
    fs::rename(&tmp.0, data.join("current"))?;
    Ok(())
}
fn stage_pin(data: &Path, target: &str) -> Result<Remove> {
    let tmp = Remove(temporary(data)?);
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp.0)?;
    writeln!(f, "{target}")?;
    f.sync_all()?;
    Ok(tmp)
}
fn restore(data: &Path, previous: Option<&str>) -> Result<()> {
    if let Some(previous) = previous {
        switch(data, previous)
    } else {
        if fs::symlink_metadata(data.join("current")).is_ok() {
            fs::remove_file(data.join("current"))?;
        }
        Ok(())
    }
}
fn prune(data: &Path) -> Result<()> {
    let current = current(data)?;
    let pinned = pin(data)?;
    let mut others = Vec::new();
    for entry in fs::read_dir(data.join("bin"))? {
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if tag(&name).is_err()
            || !entry.file_type()?.is_dir()
            || Some(&name) == current.as_ref()
            || Some(&name) == pinned.as_ref()
        {
            continue;
        }
        let installed = text(&entry.path().join("installed-at"))
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        others.push((installed, name, entry.path()));
    }
    others.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    for (_, _, path) in others.into_iter().skip(2) {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}
pub fn run(
    home: &Path,
    data: &Path,
    options: &Options,
    discover: impl FnOnce() -> Result<Vec<Candidate>>,
) -> Result<String> {
    // This must precede discovery, lock creation, downloads and every mutation.
    let pinned = pin(data)?;
    ensure!(
        pinned.is_none() || options.unpin,
        "binary is pinned to {}; pass --unpin before updating",
        pinned.as_deref().unwrap_or("")
    );
    let previous = current(data)?;
    let message = if let Some(to) = &options.to {
        verify(data, to, None)?;
        let staged = stage_pin(data, to)?;
        switch(data, to)?;
        if fs::rename(&staged.0, data.join("pin")).is_err() {
            restore(data, previous.as_deref())?;
            anyhow::bail!("could not persist rollback pin; previous current restored");
        }
        format!("Using and pinning {to}")
    } else {
        let candidate = highest(discover()?)?;
        let release = Release::read(&candidate.kit_dir.join("release/checksums.txt"))?;
        if previous.as_deref() == Some(&release.tag) {
            verify(data, &release.tag, Some(&release))?;
            if options.unpin && pinned.is_some() {
                fs::remove_file(data.join("pin"))?;
            }
            return Ok(format!("Up to date: {}", release.tag));
        }
        let status = Command::new(&candidate.launcher)
            .arg("bootstrap")
            .env("HOME", home)
            .env(
                "XDG_DATA_HOME",
                data.parent()
                    .ok_or_else(|| anyhow::anyhow!("invalid shared data directory"))?,
            )
            .env_remove("BUZZ_KIT_BIN")
            .stdin(Stdio::null())
            .output()?;
        let valid = status.status.success()
            && matches!(current(data), Ok(Some(name)) if name == release.tag)
            && verify(data, &release.tag, Some(&release)).is_ok();
        if !valid {
            restore(data, previous.as_deref())?;
            anyhow::bail!(
                "installed launcher failed verification; previous current and pin preserved"
            );
        }
        if options.unpin && pinned.is_some() {
            fs::remove_file(data.join("pin"))?;
        }
        format!(
            "Updated from {} to {} using {} plugin {}",
            previous.as_deref().unwrap_or("no current"),
            release.tag,
            candidate.host,
            candidate.version
        )
    };
    if !options.no_prune {
        if let Err(_) = prune(data) {
            return Ok(format!(
                "{message}; older-cache pruning failed, inspect cache permissions"
            ));
        }
    }
    Ok(message)
}
pub fn newer_offer(data: &Path, candidates: Vec<Candidate>) -> Result<Option<String>> {
    if pin(data)?.is_some() {
        return Ok(None);
    }
    let candidate = highest(candidates)?;
    let release = Release::read(&candidate.kit_dir.join("release/checksums.txt"))?;
    let current = current(data)?;
    if current.as_deref() == Some(&release.tag) {
        return Ok(None);
    }
    if let Some(current) = current.as_deref().filter(|s| s.starts_with("bin-v")) {
        if tag(&release.tag)? <= tag(current)? {
            return Ok(None);
        }
    }
    Ok(Some(release.tag))
}
