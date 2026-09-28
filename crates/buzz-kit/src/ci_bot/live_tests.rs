//! Opt-in integration evidence. Requires an explicitly authorized scratch channel.
//! Operational configuration and evidence must live outside the public checkout.
use super::*;
use crate::config::{Backend, Project};
use serde_json::{Value, json};

fn request(url: &str, secret: &str, marker: &str) -> Result<u16> {
    let body = serde_json::to_string(&json!({
        "repo":"buzz-kit-acceptance", "event":"webhook lifecycle",
        "title":marker, "actor":"integration test", "url":""
    }))?;
    let header = Zeroizing::new(format!("x-webhook-secret: {secret}\n"));
    let mut child = Command::new("curl")
        .args([
            "--disable",
            "--silent",
            "--show-error",
            "--max-time",
            "20",
            "--proto",
            "=https",
            "--header",
            "@-",
            "--header",
            "Content-Type: application/json",
            "--data-binary",
            &body,
            "--output",
            "/dev/null",
            "--write-out",
            "%{http_code}",
            "--url",
            url,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().unwrap().write_all(header.as_bytes())?;
    let output = child.wait_with_output()?;
    let _stderr = Zeroizing::new(output.stderr);
    ensure!(
        output.status.success(),
        "webhook transport failed; diagnostics suppressed"
    );
    Ok(std::str::from_utf8(&output.stdout)?.parse()?)
}

struct Cleanup<'a> {
    client: &'a Buzz,
    key: &'a Secret,
    id: String,
    deleted: bool,
}
impl Cleanup<'_> {
    fn delete(&mut self) -> Result<()> {
        let raw = self.client.execute_sensitive(
            &["workflows", "delete", "--workflow", &self.id],
            self.key,
            None,
        )?;
        let value: Value = serde_json::from_slice(&raw)?;
        ensure!(
            value["accepted"] == true,
            "disposable workflow deletion not accepted"
        );
        self.deleted = true;
        Ok(())
    }
}
impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        if !self.deleted && self.delete().is_err() {
            eprintln!("Disposable workflow cleanup failed; inspect the private recovery record.");
        }
    }
}

#[test]
#[ignore = "live scratch writes require explicit authorization and external configuration"]
fn webhook_lifecycle_live() -> Result<()> {
    ensure!(
        std::env::var("BUZZ_KIT_LIVE_ALLOW_WRITES").as_deref() == Ok("yes"),
        "explicit live write gate is required"
    );
    let path = fs::canonicalize(std::env::var("BUZZ_KIT_LIVE_CONFIG")?)?;
    let public_root = fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))?;
    ensure!(
        !path.starts_with(public_root),
        "live configuration must be outside the public repository"
    );
    let project: Project = serde_json::from_slice(&fs::read(&path)?)?;
    let relay = project
        .relay
        .ok_or_else(|| anyhow::anyhow!("relay required"))?;
    let channel = project
        .channel
        .ok_or_else(|| anyhow::anyhow!("scratch channel required"))?;
    let home = PathBuf::from(std::env::var_os("HOME").unwrap());
    let name = std::env::var("BUZZ_KIT_LIVE_ASSISTANT")?;
    let store = crate::keystore::open(Backend::Keychain, &home)?;
    let key = store
        .get(&name)?
        .ok_or_else(|| anyhow::anyhow!("existing enrolled assistant required"))?;
    let client = Buzz {
        executable: crate::buzz::discover(&Default::default(), &home)?,
        relay,
    };
    client.channel(&channel.id, &key)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let marker = format!("buzz-kit-lifecycle-{stamp}");
    let yaml = WORKFLOW.replace("Buzz GitHub notifications", &marker);
    // Reserve evidence before any mutation; refusing reuse prevents accidental duplicate runs.
    let record_path = path.parent().unwrap().join("webhook-lifecycle.json");
    let record = file(&record_path, b"{\"phase\":\"creation pending\"}")?;
    record.keep();
    let raw = client.execute_sensitive(
        &[
            "workflows",
            "create",
            "--channel",
            &channel.id,
            "--yaml",
            "-",
        ],
        &key,
        Some(yaml.as_bytes()),
    )?;
    let envelope: Envelope = serde_json::from_slice(&raw)
        .map_err(|_| anyhow::anyhow!("unexpected creation response; inspect authorized channel"))?;
    let id = envelope
        .workflow_id
        .ok_or_else(|| anyhow::anyhow!("no workflow recovery identifier"))?;
    ensure!(crate::init::is_uuid(&id), "invalid workflow identifier");
    let mut cleanup = Cleanup {
        client: &client,
        key: &key,
        id: id.clone(),
        deleted: false,
    };
    fs::write(
        &record_path,
        serde_json::to_vec_pretty(&json!({"workflow_id":id,"phase":"created"}))?,
    )?;
    let hook = parse_webhook(&raw)?;
    let url = format!("{}/hooks/{}", client.relay.trim_end_matches('/'), id);
    let wrong_marker = format!("{marker}-wrong-secret");
    let wrong = request(&url, "deliberately-invalid-test-secret", &wrong_marker)?;
    ensure!(
        wrong == 401,
        "wrong secret did not return 401 (status {wrong})"
    );
    let before = request(&url, &hook.secret, &format!("{marker}-before-update"))?;
    ensure!(
        before == 202,
        "valid secret did not return 202 (status {before})"
    );
    let updated_yaml = yaml.replace(&marker, &format!("{marker}-updated"));
    let update = client.execute_sensitive(
        &[
            "workflows",
            "update",
            "--channel",
            &channel.id,
            "--workflow",
            &id,
            "--yaml",
            "-",
        ],
        &key,
        Some(updated_yaml.as_bytes()),
    )?;
    let accepted: Envelope = serde_json::from_slice(&update)
        .map_err(|_| anyhow::anyhow!("unexpected update response; suppressed"))?;
    ensure!(accepted.accepted, "workflow update not accepted");
    let replacement = parse_webhook(&update).ok();
    let old_after = request(&url, &hook.secret, &format!("{marker}-old-after-update"))?;
    ensure!(
        [202, 401].contains(&old_after),
        "unexpected old-secret status {old_after}"
    );
    let mut new_after = None;
    let changed = replacement.as_ref().map(|new| *new.secret != *hook.secret);
    if let Some(new) = &replacement {
        ensure!(new.workflow_id == id, "update changed workflow ID");
        let status = request(&url, &new.secret, &format!("{marker}-new-after-update"))?;
        ensure!(status == 202, "replacement secret did not return 202");
        new_after = Some(status);
    }
    cleanup.delete()?;
    let revoked = request(&url, &hook.secret, &format!("{marker}-after-delete"))?;
    ensure!(
        revoked == 404,
        "deleted webhook did not return 404 (status {revoked})"
    );
    std::thread::sleep(std::time::Duration::from_secs(2));
    let events = client.read_json(
        &[
            "messages",
            "get",
            "--channel",
            &channel.id,
            "--limit",
            "100",
        ],
        &key,
    )?;
    let events = events
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected messages shape"))?;
    let matching: Vec<_> = events
        .iter()
        .filter(|e| e["content"].as_str().is_some_and(|c| c.contains(&marker)))
        .collect();
    ensure!(
        !matching
            .iter()
            .any(|e| e["content"].as_str().unwrap().contains(&wrong_marker)),
        "wrong-secret request posted a message"
    );
    ensure!(
        !matching
            .iter()
            .any(|e| e["content"].as_str().unwrap().contains("after-delete")),
        "deleted workflow posted a message"
    );
    let expected = 1 + usize::from(old_after == 202) + usize::from(new_after == Some(202));
    ensure!(
        matching.len() == expected,
        "expected {expected} live messages, got {}",
        matching.len()
    );
    let hashes: Vec<_> = matching
        .iter()
        .map(|e| {
            Sha256::digest(e["id"].as_str().unwrap().as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        })
        .collect();
    let evidence = json!({"phase":"deleted and verified", "workflow_id":id, "wrong_secret":wrong,
        "valid_before_update":before, "old_secret_after_update":old_after,
        "replacement_secret_returned":replacement.is_some(), "secret_changed":changed,
        "new_secret_after_update":new_after, "after_delete":revoked,
        "matching_event_count":matching.len(), "event_hashes":hashes});
    fs::write(record_path, serde_json::to_vec_pretty(&evidence)?)?;
    let mut public = evidence;
    public.as_object_mut().unwrap().remove("workflow_id");
    println!("{public}");
    Ok(())
}
