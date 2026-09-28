use buzz_kit::{ci_bot, cli::CiMode};
#[test]
fn nested_secret_is_parsed_and_never_returned_in_errors() {
    let raw=br#"{"accepted":true,"workflow_id":"00000000-0000-4000-8000-000000000001","message":"response:{\"workflow_id\":\"00000000-0000-4000-8000-000000000001\",\"webhook_secret\":\"synthetic-secret-for-testing\"}"}"#;
    let v = ci_bot::parse_webhook(raw).unwrap();
    assert_eq!(&*v.secret, "synthetic-secret-for-testing");
    assert!(v.workflow_id.ends_with("0001"));
    for bad in [br#"{"accepted":false,"message":"synthetic-secret-for-testing"}"#.as_slice(),br#"{"accepted":true,"message":"response:{broken synthetic-secret-for-testing"}"#,br#"{"accepted":true,"message":"response:{\"workflow_id\":\"bad\",\"webhook_secret\":\"synthetic-secret-for-testing\"}"}"#]{
  match ci_bot::parse_webhook(bad){Ok(_)=>panic!("invalid response accepted"),Err(e)=>assert!(!e.to_string().contains("synthetic-secret"))}
 }
}
#[test]
fn templates_are_distinct_and_workflow_name_is_yaml_data() {
    let s = ci_bot::render(CiMode::Webhook, "CI \"trusted\"\nname").unwrap();
    assert!(s.contains("workflows: [\"CI \\\"trusted\\\"\\nname\"]"));
    assert!(!s.contains("uses:"));
    assert!(!s.contains(".deb"));
    assert!(s.contains("--fail-with-body"));
    assert!(s.contains("permissions: {}"));
    let b = ci_bot::render(CiMode::BotKey, "CI").unwrap();
    assert!(b.contains(buzz_kit::install_buzz::ARCHIVE_HASH));
    assert!(b.contains(buzz_kit::install_buzz::BINARY_HASH));
    assert!(!b.contains("--broadcast"));
}

use buzz_kit::keystore::{FileStore, KeyStore, Secret};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "buzz-kit-ci-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("bin")).unwrap();
        fs::create_dir(root.join(".buzz")).unwrap();
        fs::write(root.join(".buzz/config.json"),r#"{"relay":"https://example.com","channel":{"id":"00000000-0000-4000-8000-000000000001","name":"test"}}"#).unwrap();
        let store = FileStore {
            directory: root.join(".config/buzz-kit/keys"),
        };
        store
            .put(
                "agent",
                &Secret::from_bytes(format!("{:064x}", 1).as_bytes()).unwrap(),
            )
            .unwrap();
        let buzz = r#"#!/usr/bin/env python3
import os,sys,json,pathlib
root=pathlib.Path(os.environ['HOME']);args=sys.argv[1:]
assert os.environ.get('BUZZ_PRIVATE_KEY') and not os.environ.get('BUZZ_AUTH_TAG')
assert os.environ['BUZZ_PRIVATE_KEY'] not in args
if 'create' in args:
 with (root/'calls').open('a') as f:f.write('create\n')
 yaml=sys.stdin.read();assert 'on: webhook' in yaml and 'action: send_message' in yaml and '{{trigger.repo}}' in yaml
 assert 'call_webhook' not in yaml and '--broadcast' not in args
 if (root/'malformed').exists():
  print(json.dumps({'accepted':True,'workflow_id':'00000000-0000-4000-8000-000000000002','message':'response:broken'}));sys.exit(0)
 print(json.dumps({'accepted':True,'workflow_id':'00000000-0000-4000-8000-000000000002','message':'response:'+json.dumps({'workflow_id':'00000000-0000-4000-8000-000000000002','webhook_secret':'synthetic-secret-for-testing'})}))
elif 'get' in args and 'channels' in args:print(json.dumps({'channel_id':'00000000-0000-4000-8000-000000000001','name':'test'}))
else:print('[]')
"#;
        let gh = r#"#!/usr/bin/env python3
import sys,os,pathlib
root=pathlib.Path(os.environ['HOME']);args=sys.argv[1:];data=sys.stdin.read()
assert 'synthetic-secret-for-testing' not in args
if args[:2]==['secret','set']:
 if args[2]=='BUZZ_WEBHOOK_SECRET':assert data=='synthetic-secret-for-testing'
 else:assert len(data)==64 and all(c in '0123456789abcdef' for c in data)
 with (root/'calls').open('a') as f:f.write('secret:'+args[2]+'\n')
 if os.environ.get('FAIL_GH'):
  print('synthetic-secret-for-testing',file=sys.stderr);sys.exit(1)
elif args[:2]==['variable','set']:
 with (root/'calls').open('a') as f:f.write('variable:'+args[2]+'\n')
else:sys.exit(9)
"#;
        for (name, text) in [("buzz", buzz), ("gh", gh)] {
            let p = root.join("bin").join(name);
            fs::write(&p, text).unwrap();
            fs::set_permissions(p, fs::Permissions::from_mode(0o755)).unwrap();
        }
        Self { root }
    }
    fn run(&self, args: &[&str], fail: bool) -> Output {
        Command::new(env!("CARGO_BIN_EXE_buzz-kit"))
            .args([
                "--as",
                "agent",
                "ci-bot",
                "init",
                "--repo",
                "example/project",
            ])
            .args(args)
            .current_dir(&self.root)
            .env_clear()
            .env("HOME", &self.root)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.root.join("bin").display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .env("BUZZ_KIT_KEYSTORE", "file")
            .env("BUZZ_KIT_BUZZ_CLI", self.root.join("bin/buzz"))
            .env("FAIL_GH", if fail { "1" } else { "" })
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn default_webhook_exports_stdin_and_preserves_existing_workflow() {
    let f = Fixture::new();
    let result = f.run(&[], false);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!String::from_utf8_lossy(&result.stdout).contains("synthetic-secret"));
    let workflow = f.root.join(".github/workflows/buzz-notify.yml");
    let before = fs::read(&workflow).unwrap();
    assert!(!String::from_utf8_lossy(&before).contains("synthetic-secret"));
    assert!(String::from_utf8_lossy(&before).contains("BUZZ_WEBHOOK_SECRET"));
    let calls = fs::read_to_string(f.root.join("calls")).unwrap();
    assert_eq!(
        calls,
        "create\nsecret:BUZZ_WEBHOOK_SECRET\nvariable:BUZZ_WEBHOOK_URL\n"
    );
    assert!(!f.run(&[], false).status.success());
    assert_eq!(fs::read(workflow).unwrap(), before);
    assert_eq!(fs::read_to_string(f.root.join("calls")).unwrap(), calls);
}
#[test]
fn failed_secret_export_reports_public_recovery_and_prevents_duplicate_creation() {
    let f = Fixture::new();
    let result = f.run(&[], true);
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(!error.contains("synthetic-secret"));
    assert!(error.contains("00000000-0000-4000-8000-000000000002"));
    assert!(!f.root.join(".github/workflows/buzz-notify.yml").exists());
    let before = fs::read_to_string(f.root.join("calls")).unwrap();
    assert!(!f.run(&[], false).status.success());
    assert_eq!(fs::read_to_string(f.root.join("calls")).unwrap(), before);
}
#[test]
fn explicit_bot_key_mode_exports_only_selected_bot_and_keeps_local_key() {
    let f = Fixture::new();
    let result = f.run(&["--mode", "bot-key", "--name", "ci-test"], false);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let calls = fs::read_to_string(f.root.join("calls")).unwrap();
    assert_eq!(
        calls,
        "secret:BUZZ_CI_KEY\nvariable:BUZZ_RELAY_URL\nvariable:BUZZ_CHANNEL_ID\n"
    );
    let store = FileStore {
        directory: f.root.join(".config/buzz-kit/keys"),
    };
    let key = store.get("ci-test").unwrap().unwrap();
    assert!(!String::from_utf8_lossy(&result.stdout).contains(key.expose()));
    assert!(f.root.join(".github/workflows/buzz-notify.yml").is_file());
}

#[test]
fn malformed_secret_response_keeps_known_public_workflow_id() {
    let f = Fixture::new();
    fs::write(f.root.join("malformed"), "").unwrap();
    let output = f.run(&[], false);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("00000000-0000-4000-8000-000000000002")
    );
    let state = fs::read_to_string(ci_bot::state_path(&f.root, &f.root).unwrap()).unwrap();
    assert!(state.contains("00000000-0000-4000-8000-000000000002"));
    assert!(!state.contains("synthetic-secret"));
}
#[test]
fn symlinked_workflow_directory_refuses_before_external_calls() {
    let f = Fixture::new();
    fs::create_dir(f.root.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(f.root.join("elsewhere"), f.root.join(".github")).unwrap();
    assert!(!f.run(&[], false).status.success());
    assert!(!f.root.join("calls").exists());
}

#[test]
fn recorded_workflow_visibility_detects_missing_and_incomplete_setup() {
    let f = Fixture::new();
    let path = ci_bot::state_path(&f.root, &f.root).unwrap();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let helper = f.root.join("workflow-read");
    let client = buzz_kit::buzz::Buzz {
        executable: helper.clone(),
        relay: "https://example.com".into(),
    };
    let secret = Secret::from_bytes(format!("{:064x}", 1).as_bytes()).unwrap();
    assert_eq!(
        ci_bot::visibility(&f.root, &f.root, &client, &secret).unwrap(),
        None
    );
    let mut state = ci_bot::State {
        mode: "webhook".into(),
        repo: "example/project".into(),
        workflow_id: Some("00000000-0000-4000-8000-000000000002".into()),
        phase: "complete".into(),
    };
    fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
    fs::write(&helper,"#!/bin/sh\nprintf '%s' '{\"workflow_id\":\"00000000-0000-4000-8000-000000000002\",\"content\":\"{}\"}'\n").unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        ci_bot::visibility(&f.root, &f.root, &client, &secret).unwrap(),
        Some(true)
    );
    state.phase = "pending".into();
    fs::write(&path, serde_json::to_vec(&state).unwrap()).unwrap();
    assert_eq!(
        ci_bot::visibility(&f.root, &f.root, &client, &secret).unwrap(),
        Some(false)
    );
    fs::write(&helper, "#!/bin/sh\nprintf null\n").unwrap();
    assert_eq!(
        ci_bot::visibility(&f.root, &f.root, &client, &secret).unwrap(),
        Some(false)
    );
}
