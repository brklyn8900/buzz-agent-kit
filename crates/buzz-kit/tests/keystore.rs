use buzz_kit::keystore::*;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use zeroize::Zeroizing;

fn key(byte: char) -> Secret {
    Secret::from_hex(Zeroizing::new(std::iter::repeat_n(byte, 64).collect())).unwrap()
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "buzz-kit-keys-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn store(&self) -> FileStore {
        FileStore {
            directory: self.0.join("keys"),
        }
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn file_lifecycle_permissions_and_no_overwrite() {
    let t = Temp::new();
    let s = t.store();
    assert!(s.get("agent").unwrap().is_none());
    assert!(s.list().unwrap().is_empty());
    s.put("agent", &key('1')).unwrap();
    assert_eq!(
        fs::metadata(&s.directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(s.directory.join("agent.key"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(s.list().unwrap(), vec!["agent"]);
    assert!(s.put("agent", &key('2')).is_err());
    assert!(s.get("agent").unwrap().unwrap().expose() == key('1').expose());
    s.delete("agent").unwrap();
    assert!(s.get("agent").unwrap().is_none());
}

#[test]
fn unsafe_permissions_and_symlinks_refused_without_touching_target() {
    let t = Temp::new();
    let s = t.store();
    s.put("agent", &key('1')).unwrap();
    let path = s.directory.join("agent.key");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(s.get("agent").is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    symlink(&path, s.directory.join("alias.key")).unwrap();
    assert!(s.get("alias").is_err());
    assert!(s.put("alias", &key('2')).is_err());
    assert!(s.delete("alias").is_err());
    assert!(s.get("agent").unwrap().unwrap().expose() == key('1').expose());
    fs::set_permissions(&s.directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(s.get("agent").is_err());
    assert!(s.put("other", &key('2')).is_err());
    let alias = t.0.join("alias");
    symlink(&s.directory, &alias).unwrap();
    assert!(FileStore { directory: alias }.get("agent").is_err());
}

#[test]
fn identifiers_and_secret_values_fail_closed_without_echo() {
    let t = Temp::new();
    let s = t.store();
    for name in ["", "../escape", "a/b", "a\ncommand", "a'quote", "a\"quote"] {
        assert!(s.get(name).is_err());
        assert!(s.put(name, &key('1')).is_err());
        assert!(s.delete(name).is_err());
    }
    for service in ["buzz-desktop", "buzz-desktop-anything", "x/y", "x\ncommand"] {
        assert!(validate_service(service).is_err());
    }
    assert!(validate_service("buzz-kit").is_ok());
    assert!(validate_service("koinosbuzz-assistant").is_ok());
    assert!(
        Secret::from_hex(Zeroizing::new("do-not-echo".into()))
            .err()
            .unwrap()
            .to_string()
            .find("do-not-echo")
            .is_none()
    );
}

#[test]
fn malformed_file_and_hard_link_refused() {
    let t = Temp::new();
    let s = t.store();
    s.put("agent", &key('1')).unwrap();
    fs::hard_link(s.directory.join("agent.key"), s.directory.join("alias.key")).unwrap();
    assert!(s.get("agent").is_err());
    fs::remove_file(s.directory.join("alias.key")).unwrap();
    fs::write(s.directory.join("agent.key"), "private-malformed-input").unwrap();
    let err = s.get("agent").err().unwrap();
    assert!(!format!("{err:#}").contains("private-malformed-input"));
}

#[test]
fn symlinked_parent_is_refused() {
    let t = Temp::new();
    let s = t.store();
    s.put("agent", &key('1')).unwrap();
    let alias = t.0.join("parent-alias");
    symlink(&t.0, &alias).unwrap();
    let via_alias = FileStore {
        directory: alias.join("keys"),
    };
    assert!(via_alias.get("agent").is_err());
}

#[test]
fn secret_service_stdin_roundtrip_and_failed_readback_cleanup() {
    let t = Temp::new();
    let executable = t.0.join("secret-tool");
    fs::write(
        &executable,
        r#"#!/bin/sh
set -eu
cd -- "$(dirname -- "$0")"
case "$1" in
store) [ "$#" = 6 ]; cat > state ;;
lookup)
  if [ -f fail-read ] && [ -f state ]; then echo private-diagnostic >&2; exit 1; fi
  if [ -f state ]; then cat state; else exit 1; fi ;;
clear) rm -f state ;;
*) exit 20 ;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let s = SecretServiceStore::new(executable, t.0.join("inventory")).unwrap();
    s.put("agent", &key('1')).unwrap();
    assert!(s.get("agent").unwrap().unwrap().expose() == key('1').expose());
    assert!(s.put("agent", &key('2')).is_err());
    assert_eq!(s.list().unwrap(), vec!["agent"]);
    fs::write(t.0.join("fail-read"), "").unwrap();
    let err = s
        .get("agent")
        .err()
        .expect("helper error mistaken for absent key");
    assert!(!format!("{err:#}").contains("private-diagnostic"));
    s.delete("agent").unwrap();
    assert!(s.put("agent", &key('1')).is_err());
    assert!(
        !t.0.join("state").exists(),
        "failed readback did not clean up copy"
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "writes only a uniquely named throwaway Keychain item"]
fn real_keychain_stdin_roundtrip_and_cleanup() {
    let t = Temp::new();
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).unwrap();
    let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let service = format!("buzz-kit-test-{suffix}");
    let store = KeychainStore::new(&service, t.0.join("inventory")).unwrap();
    struct Cleanup<'a>(&'a KeychainStore);
    impl Drop for Cleanup<'_> {
        fn drop(&mut self) {
            let _ = self.0.delete("throwaway");
        }
    }
    let cleanup = Cleanup(&store);
    assert!(store.get("throwaway").unwrap().is_none());
    store.put("throwaway", &key('1')).unwrap();
    assert!(store.get("throwaway").unwrap().unwrap().expose() == key('1').expose());
    assert_eq!(store.list().unwrap(), vec!["throwaway"]);
    assert!(store.put("throwaway", &key('2')).is_err());
    assert!(store.get("throwaway").unwrap().unwrap().expose() == key('1').expose());
    drop(cleanup);
    assert!(store.get("throwaway").unwrap().is_none());
    assert!(store.list().unwrap().is_empty());
}
