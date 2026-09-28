use buzz_kit::keystore::{FileStore, KeyStore, Secret};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};
use zeroize::Zeroizing;

fn run_doctor(auth_fail: bool) -> serde_json::Value {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let home = std::env::temp_dir().join(format!(
        "buzz-kit-doctor-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
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
    fs::write(
        home.join(".config/buzz-kit/config.json"),
        r#"{"assistants":{"codex":"agent"},"keystore":"file"}"#,
    )
    .unwrap();
    fs::create_dir(home.join(".buzz")).unwrap();
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let relay = format!("http://{}", server.local_addr().unwrap());
    fs::write(
        home.join(".buzz/config.json"),
        serde_json::json!({"relay":relay,"channel":{"id":"fixture-channel","name":"fixture-room"}})
            .to_string(),
    )
    .unwrap();
    let serve = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(8);
        while Instant::now() < until {
            if let Ok((mut stream, _)) = server.accept() {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0u8; 4096];
                let n = stream.read(&mut request).unwrap();
                assert!(
                    String::from_utf8_lossy(&request[..n])
                        .to_lowercase()
                        .contains("accept: application/nostr+json")
                );
                let body = r#"{"supported_nips":[1,11]}"#;
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/nostr+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    });
    let buzz = home.join("buzz-fixture");
    let script = format!(
        r#"#!/bin/sh
set -eu
[ -n "${{BUZZ_PRIVATE_KEY:-}}" ] || exit 20
[ -z "${{BUZZ_AUTH_TAG:-}}" ] || exit 21
for arg in "$@"; do [ "$arg" != --private-key ] || exit 22; done
{}
case "$*" in
  *'channels list'*) printf '[{{"channel_id":"fixture-channel","name":"fixture-room"}}]' ;;
  *'channels get'*) printf '{{"channel_id":"fixture-channel","name":"fixture-room"}}' ;;
  *) exit 23 ;;
esac
"#,
        if auth_fail { "exit 3" } else { "" }
    );
    fs::write(&buzz, script).unwrap();
    fs::set_permissions(&buzz, fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_buzz-kit"))
        .args(["doctor", "--json"])
        .env_clear()
        .env("HOME", &home)
        .env("PATH", &home)
        .env("BUZZ_KIT_BUZZ_CLI", &buzz)
        .env("BUZZ_AUTH_TAG", "must-be-removed")
        .current_dir(&home)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    serve.join().unwrap();
    assert_eq!(
        output.status.success(),
        !auth_fail,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout)
        .expect("doctor should produce a structured report even on failure")
}
#[test]
fn ordered_doctor_succeeds_with_file_and_path_warnings() {
    let report = run_doctor(false);
    assert_eq!(report["ok"], true);
    let checks = report["checks"].as_array().unwrap();
    let names: Vec<_> = checks
        .iter()
        .take(9)
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "platform",
            "buzz-cli",
            "personal-config",
            "project-config",
            "server",
            "assistant-key",
            "membership",
            "channel",
            "keystore"
        ]
    );
    assert_eq!(checks[8]["status"], "warning");
    assert!(
        checks
            .iter()
            .any(|c| c["name"] == "path" && c["status"] == "warning")
    );
}
#[test]
fn auth_exit_three_is_a_membership_failure() {
    let report = run_doctor(true);
    assert_eq!(report["ok"], false);
    let check = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "membership")
        .unwrap();
    assert_eq!(check["status"], "fail");
    assert!(check["message"].as_str().unwrap().contains("operator"));
}
