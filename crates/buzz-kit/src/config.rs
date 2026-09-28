use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Default)]
pub struct Flags {
    pub assistant: Option<String>,
    pub relay: Option<String>,
    pub config: Option<PathBuf>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Personal {
    pub assistants: BTreeMap<String, String>,
    pub autopost: Autopost,
    pub keystore: Backend,
    pub default_relay: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Project {
    pub relay: Option<String>,
    pub channel: Option<Channel>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub name: String,
    pub id: String,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Autopost {
    #[default]
    Ask,
    Status,
    Off,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Backend {
    #[default]
    Auto,
    Keychain,
    SecretService,
    File,
}
pub struct Config {
    pub personal: Personal,
    pub project: Project,
    pub relay: Option<String>,
    pub backend: Backend,
    pub autopost: Autopost,
    pub assistant: Option<String>,
}
pub type Env = BTreeMap<String, String>;
/// --config (or BUZZ_KIT_CONFIG) selects project configuration, not credentials.
pub fn load(home: &Path, cwd: &Path, flags: &Flags, env: &Env) -> Result<Config> {
    let personal = read_json(&home.join(".config/buzz-kit/config.json"), false)?;
    let explicit = flags
        .config
        .clone()
        .or_else(|| env.get("BUZZ_KIT_CONFIG").map(PathBuf::from));
    let project_path = match explicit {
        Some(path) => Some((
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            },
            true,
        )),
        None => discover_project(cwd).map(|p| (p, false)),
    };
    let project = match project_path {
        Some((path, required)) => read_json(&path, required)?,
        None => Project::default(),
    };
    resolve(personal, project, flags, env)
}

fn read_json<T: serde::de::DeserializeOwned + Default>(path: &Path, required: bool) -> Result<T> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| {
            anyhow::anyhow!(
                "invalid configuration JSON or unsupported fields; check the configuration schema"
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => Ok(T::default()),
        Err(_) => anyhow::bail!("cannot read configuration file"),
    }
}

fn discover_project(cwd: &Path) -> Option<PathBuf> {
    for directory in cwd.ancestors() {
        let path = directory.join(".buzz/config.json");
        if path.exists() {
            return Some(path);
        }
        if directory.join(".git").exists() {
            break;
        }
    }
    None
}

pub fn resolve(personal: Personal, project: Project, flags: &Flags, env: &Env) -> Result<Config> {
    for name in personal.assistants.values() {
        validate_name(name)?;
    }
    let assistant = flags
        .assistant
        .clone()
        .or_else(|| env.get("BUZZ_KIT_AS").cloned());
    if let Some(name) = &assistant {
        validate_name(name)?;
    }
    let relay = flags
        .relay
        .as_ref()
        .or_else(|| env.get("BUZZ_KIT_RELAY"))
        .or(project.relay.as_ref())
        .or(personal.default_relay.as_ref())
        .map(|s| normalize_relay(s))
        .transpose()?;
    let backend = match env.get("BUZZ_KIT_KEYSTORE").map(String::as_str) {
        None => personal.keystore,
        Some("auto") => Backend::Auto,
        Some("keychain") => Backend::Keychain,
        Some("secret-service") => Backend::SecretService,
        Some("file") => Backend::File,
        Some(_) => {
            anyhow::bail!("BUZZ_KIT_KEYSTORE must be auto, keychain, secret-service or file")
        }
    };
    let autopost = match env.get("BUZZ_KIT_AUTOPOST").map(String::as_str) {
        None => personal.autopost,
        Some("ask") => Autopost::Ask,
        Some("status") => Autopost::Status,
        Some("off") => Autopost::Off,
        Some(_) => anyhow::bail!("BUZZ_KIT_AUTOPOST must be ask, status or off"),
    };
    Ok(Config {
        personal,
        project,
        relay,
        backend,
        autopost,
        assistant,
    })
}

pub fn validate_name(name: &str) -> Result<()> {
    anyhow::ensure!(
        !name.is_empty()
            && name.len() <= 64
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "assistant names must contain 1–64 ASCII letters, numbers, hyphens or underscores"
    );
    Ok(())
}

pub fn active_assistant(config: &Config, env: &Env) -> Result<String> {
    if let Some(name) = &config.assistant {
        return Ok(name.clone());
    }
    let claude = env.get("CLAUDECODE").is_some_and(|v| v == "1");
    let codex = env.get("CODEX_THREAD_ID").is_some_and(|v| !v.is_empty());
    anyhow::ensure!(
        !(claude && codex),
        "both agent runtime markers are present; choose --as explicitly"
    );
    let runtime = if claude {
        Some("claude")
    } else if codex {
        Some("codex")
    } else {
        None
    };
    if let Some(name) = runtime.and_then(|r| config.personal.assistants.get(r)) {
        return Ok(name.clone());
    }
    let choices: std::collections::BTreeSet<_> =
        config.personal.assistants.values().cloned().collect();
    if choices.len() == 1 {
        return Ok(choices.into_iter().next().unwrap());
    }
    if choices.is_empty() {
        anyhow::bail!(
            "no assistant configured; create an assistant and set the runtime map, or pass --as"
        );
    }
    anyhow::bail!(
        "choose --as from configured assistants: {}",
        choices.into_iter().collect::<Vec<_>>().join(", ")
    )
}

/// The Buzz CLI uses HTTP endpoints; configuration may use Nostr ws/wss spelling.
/// Deliberately supports a small URL subset: no userinfo, query, fragment or escaping.
pub fn normalize_relay(value: &str) -> Result<String> {
    let invalid = || {
        anyhow::anyhow!(
            "relay must be an http(s) or ws(s) URL without credentials, query or fragment"
        )
    };
    if !value.is_ascii()
        || value.bytes().any(|b| {
            b.is_ascii_control()
                || b.is_ascii_whitespace()
                || matches!(b, b'@' | b'?' | b'#' | b'\\' | b'%')
        })
    {
        return Err(invalid());
    }
    let (scheme, rest) = value.split_once("://").ok_or_else(invalid)?;
    let scheme = match scheme {
        "https" | "wss" => "https",
        "http" | "ws" => "http",
        _ => return Err(invalid()),
    };
    let authority = rest.split('/').next().unwrap_or_default();
    let (host, port) = if authority.starts_with('[') {
        let end = authority.find(']').ok_or_else(invalid)?;
        authority[1..end]
            .parse::<std::net::Ipv6Addr>()
            .map_err(|_| invalid())?;
        let tail = &authority[end + 1..];
        (
            &authority[..=end],
            if tail.is_empty() {
                None
            } else {
                Some(tail.strip_prefix(':').ok_or_else(invalid)?)
            },
        )
    } else {
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        if host.is_empty()
            || host.len() > 253
            || !host.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return Err(invalid());
        }
        (host, port)
    };
    if host.is_empty() || port.is_some_and(|p| p.parse::<u16>().map_or(true, |n| n == 0)) {
        return Err(invalid());
    }
    Ok(format!("{scheme}://{}", rest.trim_end_matches('/')))
}
