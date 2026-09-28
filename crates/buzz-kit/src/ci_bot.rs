use crate::{
    buzz::Buzz,
    cli::CiMode,
    keystore::{KeyStore, Secret},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use zeroize::Zeroizing;

pub const WORKFLOW: &str = "name: Buzz GitHub notifications\ntrigger:\n  on: webhook\nsteps:\n  - id: notify\n    action: send_message\n    text: '[{{trigger.repo}}] {{trigger.event}}: {{trigger.title}} ({{trigger.actor}}) {{trigger.url}}'\n";
pub struct Webhook {
    pub workflow_id: String,
    pub secret: Zeroizing<String>,
}
fn secret_text<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<Zeroizing<String>, D::Error> {
    String::deserialize(d).map(Zeroizing::new)
}
#[derive(Deserialize)]
struct Envelope {
    accepted: bool,
    #[serde(deserialize_with = "secret_text")]
    message: Zeroizing<String>,
    workflow_id: Option<String>,
}
#[derive(Deserialize)]
struct Response {
    workflow_id: String,
    #[serde(deserialize_with = "secret_text")]
    webhook_secret: Zeroizing<String>,
}
pub fn parse_webhook(raw: &[u8]) -> Result<Webhook> {
    let envelope: Envelope = serde_json::from_slice(raw)
        .map_err(|_| anyhow::anyhow!("unexpected workflow response shape; secret suppressed"))?;
    ensure!(
        envelope.accepted,
        "workflow creation was not accepted; response suppressed"
    );
    let nested = envelope
        .message
        .strip_prefix("response:")
        .ok_or_else(|| anyhow::anyhow!("workflow response has no secret envelope"))?;
    let response: Response = serde_json::from_str(nested)
        .map_err(|_| anyhow::anyhow!("invalid nested workflow response; secret suppressed"))?;
    ensure!(
        crate::init::is_uuid(&response.workflow_id),
        "invalid workflow identifier"
    );
    ensure!(
        envelope
            .workflow_id
            .as_ref()
            .is_none_or(|id| id == &response.workflow_id),
        "workflow identifiers disagree"
    );
    ensure!(
        (16..=256).contains(&response.webhook_secret.len())
            && response
                .webhook_secret
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b)),
        "invalid webhook secret encoding"
    );
    Ok(Webhook {
        workflow_id: response.workflow_id,
        secret: response.webhook_secret,
    })
}
pub fn render(mode: CiMode, workflow: &str) -> Result<String> {
    ensure!(
        !workflow.trim().is_empty() && workflow.len() <= 256,
        "provide a CI workflow name up to 256 bytes"
    );
    let template = match mode {
        CiMode::Webhook => include_str!("../../../templates/github/buzz-notify.yml"),
        CiMode::BotKey => include_str!("../../../templates/github/buzz-notify.bot-key.yml"),
    };
    Ok(template.replace("__CI_WORKFLOW_JSON__", &serde_json::to_string(workflow)?))
}
#[derive(Serialize, Deserialize)]
pub struct State {
    pub mode: String,
    pub repo: String,
    pub workflow_id: Option<String>,
    pub phase: String,
}
pub fn state_path(home: &Path, root: &Path) -> Result<PathBuf> {
    let root = fs::canonicalize(root)?;
    let id = Sha256::digest(root.as_os_str().as_encoded_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Ok(home.join(".config/buzz-kit/ci").join(format!("{id}.json")))
}
fn directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) => ensure!(
            m.is_dir() && !m.file_type().is_symlink(),
            "unsafe CI setup directory"
        ),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Some(p) = path.parent() {
                directory(p)?;
            }
            fs::create_dir(path)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
struct Staged(PathBuf);
impl Staged {
    fn keep(mut self) {
        self.0.clear();
    }
}
impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn file(path: &Path, bytes: &[u8]) -> Result<Staged> {
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    let staged = Staged(path.to_owned());
    f.write_all(bytes)?;
    f.sync_all()?;
    Ok(staged)
}
fn save_state(path: &Path, state: &State) -> Result<()> {
    let temp = path.with_extension(format!("{}.tmp", std::process::id()));
    let staged = file(&temp, &serde_json::to_vec_pretty(state)?)?;
    fs::rename(&staged.0, path)?;
    Ok(())
}
fn gh(gh: &Path, root: &Path, args: &[&str], input: &[u8]) -> Result<()> {
    let mut child = Command::new(gh)
        .current_dir(root)
        .args(args)
        .env_remove("BUZZ_PRIVATE_KEY")
        .env_remove("BUZZ_AUTH_TAG")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| anyhow::anyhow!("cannot start GitHub CLI"))?;
    if child.stdin.take().unwrap().write_all(input).is_err() {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("GitHub input failed; diagnostics suppressed");
    }
    ensure!(
        child.wait()?.success(),
        "GitHub setup failed; diagnostics suppressed"
    );
    Ok(())
}
pub fn repository(gh: &Path, root: &Path, explicit: Option<&str>) -> Result<String> {
    let name = if let Some(value) = explicit {
        value.to_owned()
    } else {
        let out = Command::new(gh)
            .current_dir(root)
            .args([
                "repo",
                "view",
                "--json",
                "nameWithOwner",
                "--jq",
                ".nameWithOwner",
            ])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()?;
        ensure!(
            out.status.success(),
            "cannot identify GitHub repository; provide --repo owner/repo"
        );
        String::from_utf8(out.stdout)?.trim().to_owned()
    };
    let parts: Vec<_> = name.split('/').collect();
    ensure!(
        parts.len() == 2
            && parts.iter().all(|s| !s.is_empty()
                && *s != "."
                && *s != ".."
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))),
        "repository must be owner/repo"
    );
    Ok(name)
}
pub struct Setup<'a> {
    pub home: &'a Path,
    pub root: &'a Path,
    pub gh: &'a Path,
    pub repo: &'a str,
    pub ci_workflow: &'a str,
    pub mode: CiMode,
    pub name: &'a str,
    pub client: &'a Buzz,
    pub channel: &'a str,
    pub assistant: Option<&'a Secret>,
    pub store: &'a dyn KeyStore,
}
#[derive(Serialize)]
pub struct Initialized {
    pub mode: String,
    pub workflow_id: Option<String>,
    pub bot: Option<crate::identity::Identity>,
    pub next: String,
}
pub fn initialize(setup: Setup<'_>) -> Result<Initialized> {
    crate::config::validate_name(setup.name)?;
    repository(setup.gh, setup.root, Some(setup.repo))?;
    ensure!(
        crate::init::is_uuid(setup.channel),
        "invalid configured channel"
    );
    let relay = crate::config::normalize_relay(&setup.client.relay)?;
    if matches!(setup.mode, CiMode::Webhook) {
        ensure!(
            relay.starts_with("https://"),
            "webhook setup requires HTTPS to protect its secret"
        );
    }
    let template = render(setup.mode, setup.ci_workflow)?;
    crate::guards::Scanner::from_store(setup.store)?.check(template.as_bytes())?;
    let destination = setup.root.join(".github/workflows/buzz-notify.yml");
    ensure!(
        fs::symlink_metadata(&destination).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
        "notification workflow already exists or cannot be inspected; refusing overwrite"
    );
    // Inspect each project directory, even if the final directory already exists.
    directory(&setup.root.join(".github"))?;
    directory(destination.parent().unwrap())?;
    let state_path = state_path(setup.home, setup.root)?;
    ensure!(
        !state_path.try_exists()? && fs::symlink_metadata(&state_path).is_err(),
        "CI setup record already exists at {}; inspect/recover it before retrying to avoid duplicate workflows",
        state_path.display()
    );
    let staged = file(
        &destination.with_file_name(format!(".buzz-notify-{}.tmp", std::process::id())),
        template.as_bytes(),
    )?;
    for path in [
        setup.home.join(".config"),
        setup.home.join(".config/buzz-kit"),
        setup.home.join(".config/buzz-kit/ci"),
    ] {
        directory(&path)?;
    }
    let mut state = State {
        mode: match setup.mode {
            CiMode::Webhook => "webhook",
            CiMode::BotKey => "bot-key",
        }
        .into(),
        repo: setup.repo.into(),
        workflow_id: None,
        phase: "pending".into(),
    };
    // Reserve before any remote mutation. Uncertain delivery remains recorded.
    let reservation = file(&state_path, &serde_json::to_vec_pretty(&state)?)?;
    reservation.keep();
    let result = (|| -> Result<Initialized> {
        let mut bot = None;
        match setup.mode {
            CiMode::Webhook => {
                let secret = setup
                    .assistant
                    .ok_or_else(|| anyhow::anyhow!("active assistant required"))?;
                setup.client.channel(setup.channel, secret)?;
                let definition = WORKFLOW.replacen(
                    "name: Buzz GitHub notifications",
                    &format!("name: {}", setup.name),
                    1,
                );
                let raw = setup.client.execute_sensitive(
                    &[
                        "workflows",
                        "create",
                        "--channel",
                        setup.channel,
                        "--yaml",
                        "-",
                    ],
                    secret,
                    Some(definition.as_bytes()),
                )?;
                // Preserve a validated public ID even if the nested secret shape changed.
                if let Ok(envelope) = serde_json::from_slice::<Envelope>(&raw) {
                    if envelope.accepted {
                        if let Some(id) = envelope.workflow_id.filter(|id| crate::init::is_uuid(id))
                        {
                            state.workflow_id = Some(id);
                            state.phase = "created; secret parsing pending".into();
                            save_state(&state_path, &state)?;
                        }
                    }
                }
                let webhook = parse_webhook(&raw)?;
                state.workflow_id = Some(webhook.workflow_id.clone());
                state.phase = "created; GitHub export pending".into();
                save_state(&state_path, &state)?;
                gh(
                    setup.gh,
                    setup.root,
                    &["secret", "set", "BUZZ_WEBHOOK_SECRET", "--repo", setup.repo],
                    webhook.secret.as_bytes(),
                )?;
                let url = format!(
                    "{}/hooks/{}",
                    relay.trim_end_matches('/'),
                    webhook.workflow_id
                );
                gh(
                    setup.gh,
                    setup.root,
                    &["variable", "set", "BUZZ_WEBHOOK_URL", "--repo", setup.repo],
                    url.as_bytes(),
                )?;
            }
            CiMode::BotKey => {
                let public = crate::assistant::create(setup.store, setup.name)?;
                state.phase = format!("bot {} created; GitHub export pending", setup.name);
                save_state(&state_path, &state)?;
                let secret = setup
                    .store
                    .get(setup.name)?
                    .ok_or_else(|| anyhow::anyhow!("new bot key unavailable"))?;
                gh(
                    setup.gh,
                    setup.root,
                    &["secret", "set", "BUZZ_CI_KEY", "--repo", setup.repo],
                    secret.expose().as_bytes(),
                )?;
                gh(
                    setup.gh,
                    setup.root,
                    &["variable", "set", "BUZZ_RELAY_URL", "--repo", setup.repo],
                    relay.as_bytes(),
                )?;
                gh(
                    setup.gh,
                    setup.root,
                    &["variable", "set", "BUZZ_CHANNEL_ID", "--repo", setup.repo],
                    setup.channel.as_bytes(),
                )?;
                bot = Some(public);
            }
        }
        fs::hard_link(&staged.0,&destination).map_err(|_|anyhow::anyhow!("cannot install notification workflow without overwriting; remote configuration already exists"))?;
        state.phase = "complete".into();
        save_state(&state_path, &state)?;
        Ok(Initialized{mode:state.mode.clone(),workflow_id:state.workflow_id.clone(),bot,next:match setup.mode{
            CiMode::Webhook=>"Review and commit the workflow, then verify a GitHub event. Keep the initializing assistant in the channel.".into(),
            CiMode::BotKey=>format!("Have the operator enroll this public identity, then add it to the channel by hex/display name with role bot. Review and commit the workflow. Local key retained; remove with assistant remove {} after export if desired.",setup.name)
        }})
    })();
    result.map_err(|e|anyhow::anyhow!("{}; setup incomplete. Public workflow ID: {}. Recovery record: {}. Inspect remote state before retrying; secrets cannot be recovered from this record.",e,state.workflow_id.as_deref().unwrap_or("unknown (delivery may be uncertain)"),state_path.display()))
}
pub fn visibility(
    home: &Path,
    root: &Path,
    client: &Buzz,
    secret: &Secret,
) -> Result<Option<bool>> {
    let path = state_path(home, root)?;
    if !path.try_exists()? {
        return Ok(None);
    }
    let m = fs::symlink_metadata(&path)?;
    ensure!(
        m.is_file() && !m.file_type().is_symlink(),
        "unsafe CI record"
    );
    let state: State = serde_json::from_slice(&fs::read(path)?)?;
    if state.mode != "webhook" {
        return Ok(None);
    }
    let Some(id) = state.workflow_id else {
        return Ok(Some(false));
    };
    ensure!(crate::init::is_uuid(&id), "invalid recorded workflow");
    let value = client.read_json(&["workflows", "get", "--workflow", &id], secret)?;
    Ok(Some(
        value["workflow_id"].as_str() == Some(&id) && state.phase == "complete",
    ))
}
