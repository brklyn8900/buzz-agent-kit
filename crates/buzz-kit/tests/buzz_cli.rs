use buzz_kit::keystore::{FileStore, KeyStore, Secret};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Output, Stdio},
};
use zeroize::Zeroizing;

#[test]
fn safe_reads_and_profiles_use_selected_key_and_guard_all_profile_fields() {
    let home = std::env::temp_dir().join(format!("buzz-kit-boundary-{}", std::process::id()));
    fs::create_dir(&home).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(home.clone());
    let store = FileStore {
        directory: home.join(".config/buzz-kit/keys"),
    };
    store
        .put(
            "agent",
            &Secret::from_hex(Zeroizing::new(format!("{:064x}", 1))).unwrap(),
        )
        .unwrap();
    let helper = home.join("buzz");
    fs::write(&helper,r#"#!/bin/sh
set -eu
[ -n "${BUZZ_PRIVATE_KEY:-}" ] || exit 20
[ -z "${BUZZ_AUTH_TAG:-}" ] || exit 21
for arg in "$@"; do [ "$arg" != "$BUZZ_PRIVATE_KEY" ] || exit 22; done
printf '%s\n' "$@" > "$HOME/args"
case "$*" in
*'users set-profile'*) printf '{"accepted":true,"event_id":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}' ;;
*) printf '[]' ;;
esac
"#).unwrap();
    fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
    let run = |args: &[&str]| -> Output {
        Command::new(env!("CARGO_BIN_EXE_buzz-kit"))
            .args(["--as", "agent", "--relay", "https://example.com"])
            .args(args)
            .env_clear()
            .env("HOME", &home)
            .env("BUZZ_KIT_KEYSTORE", "file")
            .env("BUZZ_KIT_BUZZ_CLI", &helper)
            .env("BUZZ_AUTH_TAG", "must-remove")
            .current_dir(&home)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let read = run(&["as", "agent", "--", "channels", "list"]);
    assert!(
        read.status.success(),
        "{}",
        String::from_utf8_lossy(&read.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&read.stdout).unwrap(),
        serde_json::json!([])
    );
    assert!(
        !run(&["as", "agent", "--", "messages", "send"])
            .status
            .success()
    );
    assert!(run(&["search", "example phrase"]).status.success());
    let profile = run(&[
        "assistant",
        "profile",
        "agent",
        "--profile-name",
        "Test Agent",
        "--about",
        "local test only",
    ]);
    assert!(
        profile.status.success(),
        "{}",
        String::from_utf8_lossy(&profile.stderr)
    );
    let args = fs::read_to_string(home.join("args")).unwrap();
    assert!(args.contains("--name\nTest Agent\n"));
    assert!(!args.contains("--private-key"));
    let secret = format!("sk-ant-{}", "x".repeat(20));
    for field in ["--profile-name", "--about", "--avatar"] {
        assert!(
            !run(&["assistant", "profile", "agent", field, &secret])
                .status
                .success()
        );
        assert_eq!(
            fs::read_to_string(home.join("args")).unwrap(),
            args,
            "blocked profile reached subprocess"
        );
    }
    assert!(
        run(&["assistant", "profile", "agent", "--about=--help"])
            .status
            .success()
    );
    assert!(
        fs::read_to_string(home.join("args"))
            .unwrap()
            .contains("--about=--help"),
        "profile data can be mistaken for a CLI flag"
    );
    let before = fs::read_to_string(home.join("args")).unwrap();
    let oversized = "x".repeat(65_537);
    assert!(
        !run(&["assistant", "profile", "agent", "--about", &oversized])
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(home.join("args")).unwrap(), before);
}
