use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
const REPO: &str = "brklyn8900/buzz-agent-kit";
pub fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
pub fn project_root(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|p| p.join(".git").exists() || p.join(".buzz/config.json").exists())
        .unwrap_or(cwd)
        .to_owned()
}
pub fn resolve_channel(
    client: &crate::buzz::Buzz,
    selection: &str,
    secret: &crate::keystore::Secret,
) -> Result<crate::config::Channel> {
    let channel = if is_uuid(selection) {
        client.channel(selection, secret)?
    } else {
        let result = client.read_json(
            &["channels", "search", "--query", selection, "--exact"],
            secret,
        )?;
        let channels: Vec<crate::buzz::Channel> = serde_json::from_value(result)
            .map_err(|_| anyhow::anyhow!("Buzz channel search shape changed"))?;
        let matching: Vec<_> = channels
            .into_iter()
            .filter(|c| c.name.eq_ignore_ascii_case(selection))
            .collect();
        ensure!(
            matching.len() == 1,
            "channel is not uniquely visible; ask its owner to add the assistant by display name or hex, or choose a UUID"
        );
        let channel = matching.into_iter().next().unwrap();
        client.channel(&channel.channel_id, secret)?
    };
    Ok(crate::config::Channel {
        name: channel.name,
        id: channel.channel_id,
    })
}
pub struct Settings {
    pub value: Value,
    pub marketplace: String,
    pub notices: Vec<String>,
}
fn is_repo(entry: &Value) -> bool {
    let source = &entry["source"];
    matches!(source["source"].as_str(), Some("github" | "git"))
        && source["repo"]
            .as_str()
            .or_else(|| source["url"].as_str())
            .is_some_and(crate::host::repository)
}
pub fn merge_settings(mut existing: Value, version: &str, update: bool) -> Result<Settings> {
    ensure!(
        regex::Regex::new(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z]+(?:[.-][0-9A-Za-z]+)*)?$")?
            .is_match(version),
        "invalid pinned version"
    );
    let root = existing
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("Claude settings must be a JSON object"))?;
    let markets = root
        .entry("extraKnownMarketplaces")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("extraKnownMarketplaces must be an object"))?;
    if let Some(entry) = markets.get("buzz-agent-kit") {
        ensure!(
            is_repo(entry),
            "marketplace buzz-agent-kit points to a different or unsupported repository; refusing impersonation risk"
        );
    }
    let aliases: Vec<_> = markets
        .iter()
        .filter(|(_, v)| is_repo(v))
        .map(|(k, _)| k.clone())
        .collect();
    ensure!(
        aliases.len() <= 1,
        "multiple marketplace names point to the kit; resolve the ambiguity first"
    );
    let marketplace = aliases
        .first()
        .cloned()
        .unwrap_or_else(|| "buzz-agent-kit".into());
    crate::config::validate_name(&marketplace)?;
    let reference = format!("v{version}");
    let mut notices = Vec::new();
    if let Some(entry) = markets.get_mut(&marketplace) {
        let source = entry
            .get_mut("source")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| anyhow::anyhow!("invalid marketplace source"))?;
        match source.get("ref").and_then(Value::as_str) {
            Some(current) if current == reference => {}
            current => {
                // Refs are untrusted configuration, so only display safe tag names.
                let old = current
                    .filter(|s| {
                        s.len() <= 128
                            && s.bytes().all(|b| {
                                b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_')
                            })
                    })
                    .unwrap_or("<missing or unrecognized ref>");
                if update {
                    source.insert("ref".into(), reference.clone().into());
                } else {
                    notices.push(format!("kept marketplace ref {old}; requested {reference}; use --update-claude-settings to change it"));
                }
            }
        }
    } else {
        markets.insert(
            marketplace.clone(),
            json!({"source":{"source":"github","repo":REPO,"ref":reference}}),
        );
    }
    let enabled = root
        .entry("enabledPlugins")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("enabledPlugins must be an object"))?;
    let plugin = format!("buzz-kit@{marketplace}");
    match enabled.get(&plugin) {
        None => {
            enabled.insert(plugin, true.into());
        }
        Some(Value::Bool(false)) => {
            notices.push("kit deliberately disabled in team settings; left disabled".into())
        }
        Some(Value::Bool(true)) => {}
        _ => anyhow::bail!("kit enabledPlugins value must be boolean"),
    }
    Ok(Settings {
        value: existing,
        marketplace,
        notices,
    })
}

fn safe_path(path: &Path) -> Result<()> {
    for part in [path.parent().unwrap(), path] {
        match fs::symlink_metadata(part) {
            Ok(m) => ensure!(
                !m.file_type().is_symlink()
                    && (if part == path {
                        m.is_file()
                    } else {
                        m.is_dir()
                    }),
                "configuration path is a symlink or has an unexpected type"
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => anyhow::bail!("cannot inspect configuration path"),
        }
    }
    Ok(())
}
fn existing(path: &Path) -> Result<Option<Vec<u8>>> {
    safe_path(path)?;
    match fs::read(path) {
        Ok(b) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => anyhow::bail!("cannot read existing configuration"),
    }
}
fn parse(bytes: Option<&[u8]>) -> Result<Value> {
    match bytes {
        Some(b) => serde_json::from_slice(b)
            .map_err(|_| anyhow::anyhow!("invalid existing JSON; nothing changed")),
        None => Ok(json!({})),
    }
}
fn bytes(value: &Value) -> Result<Vec<u8>> {
    let mut b = serde_json::to_vec_pretty(value)?;
    b.push(b'\n');
    Ok(b)
}
struct Staged(PathBuf);
impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn stage(path: &Path, contents: &[u8]) -> Result<Staged> {
    let parent = path.parent().unwrap();
    if !parent.exists() {
        fs::create_dir(parent)?;
    }
    safe_path(path)?;
    let mut random = [0u8; 8];
    getrandom::fill(&mut random)?;
    let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let tmp = Staged(parent.join(format!(".buzz-kit-{suffix}.tmp")));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(&tmp.0)?;
    file.write_all(contents)?;
    file.sync_all()?;
    Ok(tmp)
}
/// Preflight both files before mutation; stage complete replacements on the same
/// filesystem. If the second rename fails, restore the original project file.
pub fn write_project(
    root: &Path,
    project: &Value,
    version: &str,
    update: bool,
    no_settings: bool,
) -> Result<Vec<String>> {
    let project_path = root.join(".buzz/config.json");
    let settings_path = root.join(".claude/settings.json");
    let old_project = existing(&project_path)?;
    let original = parse(old_project.as_deref())?;
    ensure!(
        original.is_object() && project.is_object(),
        "project config must be a JSON object"
    );
    let mut notices = Vec::new();
    let mut old_settings = None;
    let mut settings_bytes = None;
    if !no_settings {
        old_settings = existing(&settings_path)?;
        let original = parse(old_settings.as_deref())?;
        let merged = merge_settings(original.clone(), version, update)?;
        notices = merged.notices;
        notices.push(format!("Claude marketplace: {}", merged.marketplace));
        if merged.value != original || old_settings.is_none() {
            settings_bytes = Some(bytes(&merged.value)?);
        }
    }
    let project_bytes = bytes(project)?;
    let new_project = if original != *project || old_project.is_none() {
        Some(stage(&project_path, &project_bytes)?)
    } else {
        None
    };
    let new_settings = settings_bytes
        .as_deref()
        .map(|b| stage(&settings_path, b))
        .transpose()?;
    let backup = if new_project.is_some() {
        old_project
            .as_deref()
            .map(|b| stage(&project_path, b))
            .transpose()?
    } else {
        None
    };
    ensure!(
        existing(&project_path)? == old_project,
        "project config changed during init; retry after reviewing it"
    );
    if !no_settings {
        ensure!(
            existing(&settings_path)? == old_settings,
            "Claude settings changed during init; retry after reviewing them"
        );
    }
    if let Some(staged) = &new_project {
        fs::rename(&staged.0, &project_path)?;
    }
    if let Some(staged) = &new_settings {
        if fs::rename(&staged.0, &settings_path).is_err() {
            if new_project.is_some() {
                let restored = if let Some(backup) = &backup {
                    fs::rename(&backup.0, &project_path)
                } else {
                    fs::remove_file(&project_path)
                };
                ensure!(
                    restored.is_ok(),
                    "settings write failed and project rollback failed; inspect both configs before retrying"
                );
            }
            anyhow::bail!("settings write failed; original project configuration restored");
        }
    }
    notices.push(format!("Codex: codex plugin marketplace add {REPO} --ref v{version} && codex plugin add buzz-kit@buzz-agent-kit"));
    Ok(notices)
}
