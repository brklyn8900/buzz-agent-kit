use crate::{
    buzz::{self, Buzz},
    config::{self, Backend, Env, Flags},
    host, identity, keystore,
};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Serialize)]
pub struct Check {
    pub name: String,
    pub status: String,
    pub message: String,
}
#[derive(Serialize, Default)]
pub struct Report {
    pub ok: bool,
    pub checks: Vec<Check>,
}
impl Report {
    fn add(&mut self, name: &str, status: &str, message: impl Into<String>) {
        self.checks.push(Check {
            name: name.into(),
            status: status.into(),
            message: message.into(),
        });
    }
}
pub fn run(home: &Path, cwd: &Path, flags: &Flags, env: &Env) -> Report {
    let mut report = Report::default();
    report.add(
        "platform",
        if cfg!(any(target_os = "macos", target_os = "linux")) {
            "pass"
        } else {
            "fail"
        },
        std::env::consts::OS,
    );
    let executable = match buzz::discover(env, home) {
        Ok(p) => {
            report.add("buzz-cli", "pass", "Buzz CLI found");
            Some(p)
        }
        Err(e) => {
            report.add("buzz-cli", "fail", e.to_string());
            None
        }
    };
    let personal = config::load_personal(home);
    let exists = home.join(".config/buzz-kit/config.json").is_file();
    report.add(
        "personal-config",
        if personal.is_ok() && exists {
            "pass"
        } else {
            "fail"
        },
        if personal.is_ok() && exists {
            "personal config valid"
        } else {
            "create or repair ~/.config/buzz-kit/config.json"
        },
    );
    let project = config::load_project(cwd, flags, env);
    report.add(
        "project-config",
        if project.is_ok() { "pass" } else { "fail" },
        if project.is_ok() {
            "project config valid or optional config absent"
        } else {
            "repair project configuration JSON"
        },
    );
    let resolved =
        personal.and_then(|p| project.and_then(|project| config::resolve(p, project, flags, env)));
    let config = match resolved {
        Ok(c) => Some(c),
        Err(e) => {
            report.add("server", "fail", e.to_string());
            None
        }
    };
    let reachable = if let Some(c) = &config {
        match c
            .relay
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("configure a relay URL"))
            .and_then(buzz::reachable)
        {
            Ok(()) => {
                report.add("server", "pass", "NIP-11 HTTP request succeeded");
                true
            }
            Err(e) => {
                report.add("server", "fail", e.to_string());
                false
            }
        }
    } else {
        false
    };
    let mut key = None;
    let mut client = None;
    if let Some(c) = &config {
        let result = (|| -> anyhow::Result<keystore::Secret> {
            let name = config::active_assistant(c, env)?;
            let secret = keystore::open(c.backend, home)?
                .get(&name)?
                .ok_or_else(|| {
                    anyhow::anyhow!("assistant key absent; run assistant new or import")
                })?;
            identity::public(&secret)?;
            Ok(secret)
        })();
        match result {
            Ok(k) => {
                report.add("assistant-key", "pass", "assistant key readable and valid");
                key = Some(k)
            }
            Err(e) => report.add("assistant-key", "fail", e.to_string()),
        }
        if let (Some(executable), Some(relay)) = (executable, c.relay.clone()) {
            client = Some(Buzz { executable, relay });
        }
    } else {
        report.add("assistant-key", "skipped", "repair configuration first");
    }
    let membership = if let (Some(client), Some(secret)) = (&client, &key) {
        if reachable {
            match client.channels(secret) {
                Ok(_) => {
                    report.add("membership", "pass", "authenticated channel read succeeded");
                    true
                }
                Err(e) => {
                    let message = if e
                        .downcast_ref::<buzz::Failure>()
                        .is_some_and(|e| e.code == Some(3))
                    {
                        "authentication failed (exit 3); ask the operator to add this assistant's public key".into()
                    } else {
                        e.to_string()
                    };
                    report.add("membership", "fail", message);
                    false
                }
            }
        } else {
            report.add("membership", "skipped", "repair server connectivity first");
            false
        }
    } else {
        report.add(
            "membership",
            "skipped",
            "repair Buzz CLI and assistant key first",
        );
        false
    };
    if let Some(channel) = config.as_ref().and_then(|c| c.project.channel.as_ref()) {
        if membership {
            match client.as_ref().unwrap().channel(&channel.id,key.as_ref().unwrap()) {
                Ok(_)=>report.add("channel","pass","project channel is visible"),
                Err(_)=>report.add("channel","fail","project channel is not visible; ask its owner to add the assistant by display name or hex public key"),
            }
        } else {
            report.add("channel", "skipped", "verify membership first");
        }
    } else {
        report.add(
            "channel",
            "skipped",
            "no project channel configured; run init when ready",
        );
    }
    match config.as_ref().map(|c| c.backend) {
        Some(Backend::File) => report.add(
            "keystore",
            "warning",
            "file keystore is explicitly enabled; protect backups and local access",
        ),
        Some(_) => report.add("keystore", "pass", "platform keystore selected"),
        None => report.add("keystore", "skipped", "repair configuration first"),
    }
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local/share"))
        .join("buzz-kit");
    report.add(
        "current",
        if data.join("current").exists() {
            "pass"
        } else {
            "warning"
        },
        match std::fs::read_link(data.join("current")) {
            Ok(p) => format!("current: {}", p.display()),
            Err(_) => "shared binary cache is absent; run launcher bootstrap".into(),
        },
    );
    report.add(
        "pin",
        "pass",
        if data.join("pin").is_file() {
            "a binary pin is active; update requires --unpin"
        } else {
            "no binary pin"
        },
    );
    host_status(&mut report, "claude", home);
    host_status(&mut report, "codex", home);
    let bin = home.join(".local/bin");
    let on_path =
        std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|p| p == bin));
    report.add(
        "path",
        if on_path { "pass" } else { "warning" },
        if on_path {
            "~/.local/bin is on PATH"
        } else {
            "add ~/.local/bin to PATH and start a fresh shell and agent session"
        },
    );
    report.ok = !report.checks.iter().any(|c| c.status == "fail");
    report
}
fn host_status(report: &mut Report, name: &str, home: &Path) {
    let Some(executable) = keystore::find_program(name) else {
        report.add(name, "warning", "host CLI is unavailable");
        return;
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
        report.add(
            name,
            "warning",
            format!("{}: cannot query installed plugins", version.trim()),
        );
        return;
    };
    let result = if name == "claude" {
        host::claude(&plugins, &markets)
    } else {
        let root = std::env::var_os("CODEX_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".codex"));
        host::codex(&plugins, &markets, &root)
    };
    let status = if result.candidates.is_empty() {
        "warning"
    } else {
        "pass"
    };
    report.add(
        name,
        status,
        format!(
            "{}: {} verified candidate(s); {}",
            version.trim(),
            result.candidates.len(),
            result.notices.join("; ")
        ),
    );
}
