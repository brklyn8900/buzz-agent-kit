use buzz_kit::init::{merge_settings, write_project};
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::symlink,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
const REPO: &str = "brklyn8900/buzz-agent-kit";
fn marketplace(repo: &str, version: &str) -> Value {
    json!({"source":{"source":"github","repo":repo,"ref":version}})
}
#[test]
fn add_preserves_existing_key_order_and_never_adds_privileged_settings() {
    let existing: Value =
        serde_json::from_str(r#"{"z":1,"a":2,"permissions":{"deny":["something"]}}"#).unwrap();
    let result = merge_settings(existing.clone(), "0.1.0", false).unwrap();
    assert_eq!(result.marketplace, "buzz-agent-kit");
    assert_eq!(
        result.value["extraKnownMarketplaces"]["buzz-agent-kit"],
        marketplace(REPO, "v0.1.0")
    );
    assert_eq!(
        result.value["enabledPlugins"]["buzz-kit@buzz-agent-kit"],
        true
    );
    assert_eq!(
        result
            .value
            .as_object()
            .unwrap()
            .keys()
            .take(3)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["z", "a", "permissions"]
    );
    for key in ["hooks", "mcpServers", "env"] {
        assert!(result.value.get(key).is_none());
    }
    assert_eq!(result.value["permissions"], existing["permissions"]);
}
#[test]
fn aliases_disabled_flags_and_ref_conflicts_are_preserved_until_explicit_update() {
    let existing = json!({"extraKnownMarketplaces":{"team-kit":marketplace(REPO,"v0.0.1")},"enabledPlugins":{"buzz-kit@team-kit":false},"unrelated":9});
    let unchanged = merge_settings(existing.clone(), "0.1.0", false).unwrap();
    assert_eq!(unchanged.value, existing);
    assert_eq!(unchanged.marketplace, "team-kit");
    assert!(
        unchanged
            .notices
            .iter()
            .any(|s| s.contains("v0.0.1") && s.contains("v0.1.0"))
    );
    assert!(unchanged.notices.iter().any(|s| s.contains("disabled")));
    let updated = merge_settings(existing.clone(), "0.1.0", true).unwrap();
    let mut expected = existing;
    expected["extraKnownMarketplaces"]["team-kit"]["source"]["ref"] = "v0.1.0".into();
    assert_eq!(updated.value, expected);
    assert_eq!(
        merge_settings(updated.value.clone(), "0.1.0", false)
            .unwrap()
            .value,
        updated.value
    );
}
#[test]
fn impersonation_ambiguity_and_invalid_shapes_refuse() {
    for existing in [
        json!({"extraKnownMarketplaces":{"buzz-agent-kit":marketplace("other/repo","v0.1.0")}}),
        json!({"extraKnownMarketplaces":{"alias-a":marketplace(REPO,"v0.1.0"),"alias-b":marketplace(REPO,"v0.1.0")}}),
        json!({"extraKnownMarketplaces":[]}),
        json!({"enabledPlugins":true}),
        json!({"enabledPlugins":{"buzz-kit@buzz-agent-kit":"yes"}}),
        json!([]),
    ] {
        assert!(merge_settings(existing, "0.1.0", false).is_err());
    }
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "buzz-kit-init-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn invalid_settings_preflight_preserves_project_and_valid_writes_have_newlines() {
    let t = Temp::new();
    fs::create_dir(t.0.join(".claude")).unwrap();
    fs::write(t.0.join(".claude/settings.json"), "{").unwrap();
    fs::create_dir(t.0.join(".buzz")).unwrap();
    fs::write(
        t.0.join(".buzz/config.json"),
        "{\"relay\":\"https://old.example.com\"}\n",
    )
    .unwrap();
    let old = fs::read(t.0.join(".buzz/config.json")).unwrap();
    let project = json!({"relay":"https://new.example.com"});
    assert!(write_project(&t.0, &project, "0.1.0", false, false).is_err());
    assert_eq!(fs::read(t.0.join(".buzz/config.json")).unwrap(), old);
    assert_eq!(
        fs::read_to_string(t.0.join(".claude/settings.json")).unwrap(),
        "{"
    );
    write_project(&t.0, &project, "0.1.0", false, true).unwrap();
    assert_eq!(
        fs::read_to_string(t.0.join(".claude/settings.json")).unwrap(),
        "{"
    );
    fs::write(t.0.join(".claude/settings.json"), "{\"existing\":true}\n").unwrap();
    write_project(&t.0, &project, "0.1.0", false, false).unwrap();
    for file in [".buzz/config.json", ".claude/settings.json"] {
        let bytes = fs::read(t.0.join(file)).unwrap();
        assert!(bytes.ends_with(b"\n"));
    }
}
#[test]
fn symlinks_refuse_before_either_config_is_changed() {
    let t = Temp::new();
    let outside = Temp::new();
    fs::write(outside.0.join("settings.json"), "{}").unwrap();
    symlink(&outside.0, t.0.join(".claude")).unwrap();
    assert!(write_project(&t.0, &json!({}), "0.1.0", false, false).is_err());
    assert!(!t.0.join(".buzz").exists());
    assert_eq!(
        fs::read_to_string(outside.0.join("settings.json")).unwrap(),
        "{}"
    );
}

#[test]
fn no_verify_init_needs_no_key_and_can_skip_claude_settings() {
    use std::process::{Command, Stdio};
    let t = Temp::new();
    let channel = format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", 1, 2, 3, 4, 5);
    let output = Command::new(env!("CARGO_BIN_EXE_buzz-kit"))
        .args([
            "init",
            "--relay",
            "wss://example.com",
            "--channel",
            &channel,
            "--no-verify",
            "--no-claude-settings",
        ])
        .env_clear()
        .env("HOME", &t.0)
        .current_dir(&t.0)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let project: Value =
        serde_json::from_slice(&fs::read(t.0.join(".buzz/config.json")).unwrap()).unwrap();
    assert_eq!(project["relay"], "https://example.com");
    assert_eq!(project["channel"]["id"], channel);
    assert!(!t.0.join(".claude").exists());
    let original = fs::read(t.0.join(".buzz/config.json")).unwrap();
    let failed = Command::new(env!("CARGO_BIN_EXE_buzz-kit"))
        .args([
            "init",
            "--relay",
            "http://127.0.0.1:1",
            "--channel",
            &channel,
        ])
        .env_clear()
        .env("HOME", &t.0)
        .current_dir(&t.0)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert_eq!(fs::read(t.0.join(".buzz/config.json")).unwrap(), original);
    assert!(!t.0.join(".claude").exists());
}

#[test]
fn staging_failure_keeps_existing_project_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let t = Temp::new();
    fs::create_dir(t.0.join(".buzz")).unwrap();
    fs::create_dir(t.0.join(".claude")).unwrap();
    let original = b"{\"relay\":\"https://old.example.com\"}\n";
    fs::write(t.0.join(".buzz/config.json"), original).unwrap();
    fs::write(t.0.join(".claude/settings.json"), "{}").unwrap();
    fs::set_permissions(t.0.join(".claude"), fs::Permissions::from_mode(0o500)).unwrap();
    let result = write_project(
        &t.0,
        &json!({"relay":"https://new.example.com"}),
        "0.1.0",
        false,
        false,
    );
    fs::set_permissions(t.0.join(".claude"), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(t.0.join(".buzz/config.json")).unwrap(), original);
    assert_eq!(
        fs::read_to_string(t.0.join(".claude/settings.json")).unwrap(),
        "{}"
    );
}
