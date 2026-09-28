use buzz_kit::{
    buzz::Buzz,
    keystore::{FileStore, KeyStore, Secret},
    post::{self, Request},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use zeroize::Zeroizing;

struct Fixture {
    root: PathBuf,
    client: Buzz,
    store: FileStore,
    secret: Secret,
}
impl Fixture {
    fn new(kind: &str, fail_second: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "buzz-kit-post-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let helper = root.join("buzz");
        let script = format!(
            r#"#!/bin/sh
set -eu
cd -- "$(dirname -- "$0")"
[ -n "${{BUZZ_PRIVATE_KEY:-}}" ] || exit 20
[ -z "${{BUZZ_AUTH_TAG:-}}" ] || exit 21
for arg in "$@"; do [ "$arg" != "$BUZZ_PRIVATE_KEY" ] || exit 22; done
case "$*" in
*'channels get'*) printf '{{"channel_id":"fixture-channel","name":"fixture-room"}}' ;;
*'channels search'*) printf '[{{"channel_id":"fixture-channel","name":"fixture-room","channel_type":"{}"}}]' ;;
*'messages send'*)
  n=0; if [ -f count ]; then n=$(cat count); fi; n=$((n+1)); printf '%s' "$n" > count
  printf '%s\n' "$@" > "args.$n"
  cat > "payload.$n"
  {}
  if [ "$n" = 1 ]; then printf '{{"accepted":true,"event_id":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}}'
  else printf '{{"accepted":true,"event_id":"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"}}'; fi ;;
*) exit 23 ;;
esac
"#,
            kind,
            if fail_second {
                "if [ \"$n\" = 2 ]; then exit 2; fi"
            } else {
                ""
            }
        );
        fs::write(&helper, script).unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o700)).unwrap();
        let store = FileStore {
            directory: root.join("keys"),
        };
        let secret = Secret::from_hex(Zeroizing::new(format!("{:064x}", 1))).unwrap();
        store.put("agent", &secret).unwrap();
        Self {
            root,
            client: Buzz {
                executable: helper,
                relay: "https://example.com".into(),
            },
            store,
            secret,
        }
    }
    fn request(&self) -> Request {
        Request {
            channel: "fixture-channel".into(),
            thread: None,
            split: false,
            kind: None,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn exact_checked_bytes_reach_stdin_with_kind_and_without_file_or_secret_args() {
    for (channel_type, expected) in [("stream", "9"), ("forum", "45001")] {
        let f = Fixture::new(channel_type, false);
        let bytes = "line one\n🦀 exact trailing newlines\n\n".as_bytes();
        let ids = post::send(&f.client, &f.store, &f.secret, &f.request(), bytes).unwrap();
        assert_eq!(ids, vec!["c".repeat(64)]);
        assert_eq!(fs::read(f.root.join("payload.1")).unwrap(), bytes);
        let args = fs::read_to_string(f.root.join("args.1")).unwrap();
        assert!(args.contains(&format!("--kind\n{expected}\n")));
        assert!(args.contains("--content\n-\n"));
        assert!(!args.contains("--file"));
        assert!(!args.contains(f.secret.expose()));
    }
}
#[test]
fn forum_reply_and_split_parts_keep_the_original_root() {
    let f = Fixture::new("forum", false);
    let mut request = f.request();
    request.split = true;
    let bytes = vec![b'x'; 90_000];
    let ids = post::send(&f.client, &f.store, &f.secret, &request, &bytes).unwrap();
    assert_eq!(ids.len(), 2);
    let second = fs::read_to_string(f.root.join("args.2")).unwrap();
    assert!(second.contains("--kind\n45003\n"));
    assert!(second.contains(&format!("--reply-to\n{}\n", ids[0])));
    let f = Fixture::new("forum", false);
    let mut request = f.request();
    request.thread = Some("a".repeat(64));
    request.split = true;
    post::send(&f.client, &f.store, &f.secret, &request, &bytes).unwrap();
    for n in [1, 2] {
        let args = fs::read_to_string(f.root.join(format!("args.{n}"))).unwrap();
        assert!(args.contains("--kind\n45003\n"));
        assert!(args.contains(&format!("--reply-to\n{}\n", "a".repeat(64))));
    }
}
#[test]
fn explicit_kind_and_partial_failure_are_reported_without_retry() {
    let f = Fixture::new("unknown", false);
    let mut request = f.request();
    request.kind = Some(9);
    post::send(&f.client, &f.store, &f.secret, &request, b"explicit kind").unwrap();
    let f = Fixture::new("stream", true);
    let mut request = f.request();
    request.split = true;
    let error = post::send(
        &f.client,
        &f.store,
        &f.secret,
        &request,
        &vec![b'x'; 150_000],
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains(&"c".repeat(64)));
    assert!(error.contains("do not retry"));
    assert_eq!(fs::read_to_string(f.root.join("count")).unwrap(), "2");
}
#[test]
fn blocked_payloads_never_start_send_and_unknown_types_refuse() {
    let f = Fixture::new("stream", false);
    let mut request = f.request();
    request.split = true;
    let payload = format!(
        "{}sk-proj-{}{}",
        "x".repeat(59_995),
        "a".repeat(20),
        "y".repeat(30_000)
    );
    assert!(post::send(&f.client, &f.store, &f.secret, &request, payload.as_bytes()).is_err());
    assert!(!f.root.join("count").exists());
    let f = Fixture::new("unknown", false);
    assert!(
        post::send(
            &f.client,
            &f.store,
            &f.secret,
            &f.request(),
            b"valid content"
        )
        .is_err()
    );
    assert!(!f.root.join("count").exists());
}

#[test]
fn post_cli_reads_file_once_and_passes_only_stdin_to_buzz() {
    use std::process::{Command, Stdio};
    let f = Fixture::new("stream", false);
    let store = FileStore {
        directory: f.root.join(".config/buzz-kit/keys"),
    };
    store.put("agent", &f.secret).unwrap();
    let input = f.root.join("message with spaces.txt");
    fs::write(&input, b"exact file bytes\n\n").unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_buzz-kit"))
            .args(["--as", "agent", "--relay", "https://example.com"])
            .args(args)
            .env_clear()
            .env("HOME", &f.root)
            .env("PATH", "/usr/bin:/bin")
            .env("BUZZ_KIT_KEYSTORE", "file")
            .env("BUZZ_KIT_BUZZ_CLI", &f.client.executable)
            .current_dir(&f.root)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let result = run(&[
        "post",
        "--channel",
        "fixture-channel",
        input.to_str().unwrap(),
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout).trim(),
        "c".repeat(64)
    );
    assert_eq!(
        fs::read(f.root.join("payload.1")).unwrap(),
        b"exact file bytes\n\n"
    );
    assert!(
        !fs::read_to_string(f.root.join("args.1"))
            .unwrap()
            .contains(input.to_str().unwrap())
    );
    assert!(
        !run(&["post", "--broadcast", input.to_str().unwrap()])
            .status
            .success()
    );
    assert_eq!(fs::read_to_string(f.root.join("count")).unwrap(), "1");
}
