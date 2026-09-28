use crate::{
    config::{self, Env},
    keystore::{self, Secret},
};
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::{
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use zeroize::Zeroizing;

pub struct Buzz {
    pub executable: PathBuf,
    pub relay: String,
}
#[derive(Debug)]
pub struct Failure {
    pub code: Option<i32>,
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Buzz CLI failed (exit {:?}); helper diagnostics suppressed",
            self.code
        )
    }
}
impl std::error::Error for Failure {}
#[derive(Deserialize)]
pub struct Channel {
    pub channel_id: String,
    pub name: String,
}

pub fn validate_read(args: &[String]) -> Result<()> {
    ensure!(
        args.len() >= 2,
        "as requires an allowlisted read command and subcommand"
    );
    for arg in args {
        let option = arg.split('=').next().unwrap_or(arg);
        ensure!(
            !matches!(
                option,
                "--broadcast"
                    | "--private-key"
                    | "--auth-tag"
                    | "--relay"
                    | "--format"
                    | "--help"
                    | "-h"
                    | "--version"
            ),
            "passthrough cannot broadcast, change identity/relay/output settings, or request help"
        );
    }
    let allowed = matches!(
        (args[0].as_str(), args[1].as_str()),
        ("channels", "list" | "get" | "search" | "members")
            | ("messages", "get" | "thread" | "search")
            | ("users", "get" | "presence")
            | ("canvas", "get" | "history")
            | ("repos", "get" | "list")
            | ("feed", "get")
            | ("dms", "list")
    );
    ensure!(
        allowed,
        "as only supports read commands; use guarded post or assistant profile for writes"
    );
    Ok(())
}

pub fn discover(env: &Env, home: &Path) -> Result<PathBuf> {
    let candidate = if let Some(path) = env.get("BUZZ_KIT_BUZZ_CLI") {
        PathBuf::from(path)
    } else if cfg!(target_os = "macos")
        && Path::new("/Applications/Buzz.app/Contents/MacOS/buzz").exists()
    {
        PathBuf::from("/Applications/Buzz.app/Contents/MacOS/buzz")
    } else {
        // install-buzz-cli will maintain this explicitly managed link in M2.
        let managed = home.join(".local/share/buzz-kit/buzz-cli/current");
        if cfg!(target_os = "linux") && managed.exists() {
            managed
        } else {
            keystore::find_program("buzz").ok_or_else(||anyhow::anyhow!("Buzz CLI not found; install Buzz Desktop on macOS or run install-buzz-cli on Linux"))?
        }
    };
    ensure!(
        candidate.is_absolute()
            && candidate.is_file()
            && candidate.metadata()?.permissions().mode() & 0o111 != 0,
        "Buzz CLI path must name an absolute executable file"
    );
    Ok(candidate)
}
impl Buzz {
    pub fn read(&self, args: &[String], secret: &Secret) -> Result<serde_json::Value> {
        validate_read(args)?;
        self.read_json(&args.iter().map(String::as_str).collect::<Vec<_>>(), secret)
    }
    pub fn profile(
        &self,
        profile: &crate::cli::Profile,
        scanner: &crate::guards::Scanner,
        secret: &Secret,
    ) -> Result<serde_json::Value> {
        let mut args = vec!["users".to_owned(), "set-profile".to_owned()];
        for (flag, value) in [
            ("--name", &profile.profile_name),
            ("--about", &profile.about),
            ("--avatar", &profile.avatar),
        ] {
            if let Some(value) = value {
                scanner.check(value.as_bytes())?;
                if value.starts_with('-') {
                    args.push(format!("{flag}={value}"));
                } else {
                    args.extend([flag.to_owned(), value.clone()]);
                }
            }
        }
        ensure!(args.len() > 2, "provide at least one profile field");
        let payload = serde_json::to_vec(
            &serde_json::json!({"name":profile.profile_name,"about":profile.about,"picture":profile.avatar}),
        )?;
        ensure!(
            payload.len() <= crate::post::MAX_BYTES,
            "profile fields exceed 65,536 bytes"
        );
        let response =
            self.read_json(&args.iter().map(String::as_str).collect::<Vec<_>>(), secret)?;
        ensure!(
            response["accepted"].as_bool() == Some(true),
            "profile was not accepted by the relay"
        );
        Ok(response)
    }
    pub(crate) fn read_json(&self, args: &[&str], secret: &Secret) -> Result<serde_json::Value> {
        self.execute(args, secret, None)
    }
    pub(crate) fn execute(
        &self,
        args: &[&str],
        secret: &Secret,
        input: Option<&[u8]>,
    ) -> Result<serde_json::Value> {
        let stdout = self.execute_sensitive(args, secret, input)?;
        serde_json::from_slice(&stdout)
            .map_err(|_| anyhow::anyhow!("Buzz returned invalid JSON; check CLI compatibility"))
    }
    pub(crate) fn execute_sensitive(
        &self,
        args: &[&str],
        secret: &Secret,
        input: Option<&[u8]>,
    ) -> Result<Zeroizing<Vec<u8>>> {
        let relay = config::normalize_relay(&self.relay)?;
        let mut child = Command::new(&self.executable)
            .args(["--relay", &relay, "--format", "json"])
            .args(args)
            .env_remove("BUZZ_AUTH_TAG")
            .env_remove("BUZZ_RELAY_URL")
            .env("BUZZ_PRIVATE_KEY", secret.expose())
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| anyhow::anyhow!("cannot run Buzz CLI"))?;
        if let Some(bytes) = input {
            let result = child
                .stdin
                .take()
                .ok_or_else(|| anyhow::anyhow!("Buzz stdin unavailable"))?
                .write_all(bytes);
            if result.is_err() {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("Buzz stdin write failed; delivery may be uncertain");
            }
        }
        let output = child
            .wait_with_output()
            .map_err(|_| anyhow::anyhow!("Buzz process failed; delivery may be uncertain"))?;
        let stdout = Zeroizing::new(output.stdout);
        let _stderr = Zeroizing::new(output.stderr);
        if !output.status.success() {
            return Err(Failure {
                code: output.status.code(),
            }
            .into());
        }
        Ok(stdout)
    }
    pub fn channels(&self, secret: &Secret) -> Result<Vec<Channel>> {
        serde_json::from_value(self.read_json(&["channels", "list"], secret)?)
            .map_err(|_| anyhow::anyhow!("Buzz channel-list output shape changed"))
    }
    pub fn channel(&self, channel: &str, secret: &Secret) -> Result<Channel> {
        let value: Channel = serde_json::from_value(
            self.read_json(&["channels", "get", "--channel", channel], secret)?,
        )
        .map_err(|_| anyhow::anyhow!("Buzz channel output shape changed"))?;
        ensure!(
            value.channel_id == channel,
            "Buzz returned a different channel"
        );
        Ok(value)
    }
}

pub fn reachable(relay: &str) -> Result<()> {
    let relay = config::normalize_relay(relay)?;
    let output = Command::new("/usr/bin/curl")
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "10",
            "--proto",
            "=http,https",
            "--header",
            "Accept: application/nostr+json",
            "--",
            &relay,
        ])
        .stdin(Stdio::null())
        .output()
        .map_err(|_| anyhow::anyhow!("system curl is unavailable"))?;
    ensure!(
        output.status.success(),
        "server is unreachable; check relay URL and network"
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|_| anyhow::anyhow!("server did not return NIP-11 JSON"))?;
    ensure!(value.is_object(), "server did not return NIP-11 metadata");
    Ok(())
}
