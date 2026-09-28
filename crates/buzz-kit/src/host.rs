use serde::Serialize;
use serde_json::Value;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize)]
pub struct Candidate {
    pub host: String,
    pub version: String,
    pub launcher: PathBuf,
    pub kit_dir: PathBuf,
}
#[derive(Debug, Default, Serialize)]
pub struct Report {
    pub candidates: Vec<Candidate>,
    pub notices: Vec<String>,
}
fn unavailable(reason: &str) -> Report {
    Report {
        candidates: vec![],
        notices: vec![format!("Unavailable: {reason}")],
    }
}
fn component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'+'))
}
pub(crate) fn repository(value: &str) -> bool {
    let path = value
        .strip_prefix("https://github.com/")
        .or_else(|| value.strip_prefix("ssh://git@github.com/"))
        .or_else(|| value.strip_prefix("git@github.com:"))
        .unwrap_or(value);
    path.strip_suffix(".git").unwrap_or(path) == "brklyn8900/buzz-agent-kit"
}
fn valid_candidate(host: &str, version: &str, path: PathBuf) -> Option<Candidate> {
    if !component(version)
        || !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return None;
    }
    let launcher = path.join("bin/buzz-kit");
    if !launcher.is_file()
        || launcher.metadata().ok()?.permissions().mode() & 0o111 == 0
        || !path.join("release/checksums.txt").is_file()
    {
        return None;
    }
    let mut found = false;
    for manifest in [
        path.join("plugin.json"),
        path.join(".claude-plugin/plugin.json"),
    ] {
        if !manifest.exists() {
            continue;
        }
        let value: Value = serde_json::from_slice(&std::fs::read(manifest).ok()?).ok()?;
        if value["name"].as_str() != Some("buzz-kit") || value["version"].as_str() != Some(version)
        {
            return None;
        }
        found = true;
    }
    found.then(|| Candidate {
        host: host.into(),
        version: version.into(),
        launcher,
        kit_dir: path,
    })
}

pub fn claude(plugins: &str, marketplaces: &str) -> Report {
    parse("Claude", plugins, marketplaces, Path::new(""))
}
pub fn codex(plugins: &str, marketplaces: &str, codex_home: &Path) -> Report {
    parse("Codex", plugins, marketplaces, codex_home)
}
fn parse(host: &str, plugins: &str, marketplaces: &str, home: &Path) -> Report {
    let Ok(p): Result<Value, _> = serde_json::from_str(plugins) else {
        return unavailable("invalid plugin-list JSON");
    };
    let Ok(m): Result<Value, _> = serde_json::from_str(marketplaces) else {
        return unavailable("invalid marketplace-list JSON");
    };
    let native = host == "Codex";
    let Some(plugins) = (if native { &p["installed"] } else { &p }).as_array() else {
        return unavailable("unknown plugin-list shape");
    };
    let Some(markets) = (if native { &m["marketplaces"] } else { &m }).as_array() else {
        return unavailable("unknown marketplace-list shape");
    };
    let mut report = Report::default();
    for plugin in plugins {
        let Some(id) = plugin[if native { "pluginId" } else { "id" }].as_str() else {
            continue;
        };
        let Some(market) = id.strip_prefix("buzz-kit@") else {
            continue;
        };
        if !component(market) {
            report
                .notices
                .push("Unavailable: unsafe marketplace name".into());
            continue;
        }
        let matching: Vec<_> = markets
            .iter()
            .filter(|m| m["name"].as_str() == Some(market))
            .collect();
        if matching.len() != 1 {
            report
                .notices
                .push("Unavailable: missing or ambiguous marketplace source".into());
            continue;
        }
        let source = matching[0];
        let valid_source = if native {
            source["marketplaceSource"]["sourceType"] == "git"
                && source["marketplaceSource"]["source"]
                    .as_str()
                    .is_some_and(repository)
        } else {
            matches!(source["source"].as_str(), Some("github" | "git"))
                && source["repo"].as_str().is_some_and(repository)
        };
        if !valid_source {
            report
                .notices
                .push("Unavailable: marketplace source is not the kit repository".into());
            continue;
        }
        if plugin["enabled"].as_bool() != Some(true) {
            report
                .notices
                .push("buzz-kit is disabled or has no enabled status".into());
            continue;
        }
        let Some(version) = plugin["version"].as_str().filter(|v| component(v)) else {
            report
                .notices
                .push("Unavailable: invalid plugin version".into());
            continue;
        };
        let path = if native {
            if plugin["installed"].as_bool() != Some(true)
                || plugin["name"] != "buzz-kit"
                || plugin["marketplaceName"].as_str() != Some(market)
            {
                report
                    .notices
                    .push("Unavailable: inconsistent installed identity".into());
                continue;
            }
            home.join("plugins/cache")
                .join(market)
                .join("buzz-kit")
                .join(version)
        } else {
            let Some(path) = plugin["installPath"].as_str() else {
                report
                    .notices
                    .push("Unavailable: missing installPath".into());
                continue;
            };
            PathBuf::from(path)
        };
        match valid_candidate(host, version, path) {
            Some(candidate) => report.candidates.push(candidate),
            None => report.notices.push(format!(
                "Unavailable: cannot resolve install location for buzz-kit@{market}@{version}"
            )),
        }
    }
    if report.candidates.is_empty() && report.notices.is_empty() {
        report
            .notices
            .push("Unavailable: no installed kit plugin".into());
    }
    report
}

/// Read only host inventories; no cache scan and no host installation changes.
pub fn probe(name: &str, home: &Path) -> (String, Report) {
    use std::process::{Command, Stdio};
    let Some(executable) = crate::keystore::find_program(name) else {
        return (
            format!("{name} unavailable"),
            unavailable("host CLI is unavailable"),
        );
    };
    let run = |args: &[&str]| -> Option<String> {
        let output = Command::new(&executable)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8(output.stdout).ok()
    };
    let version = run(&["--version"]).unwrap_or_else(|| "unknown version".into());
    let Some((plugins, markets)) =
        run(&["plugin", "list", "--json"]).zip(run(&["plugin", "marketplace", "list", "--json"]))
    else {
        return (version, unavailable("cannot query installed plugins"));
    };
    let result = if name == "claude" {
        claude(&plugins, &markets)
    } else {
        let root = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"));
        codex(&plugins, &markets, &root)
    };
    (version, result)
}
pub fn installed(home: &Path) -> Vec<Candidate> {
    ["claude", "codex"]
        .into_iter()
        .flat_map(|host| probe(host, home).1.candidates)
        .collect()
}
