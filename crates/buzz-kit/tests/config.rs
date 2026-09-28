use buzz_kit::config::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

fn personal() -> Personal {
    serde_json::from_str(r#"{"assistants":{"claude":"c3po","codex":"r2d2"},"default_relay":"wss://personal.example.com","autopost":"ask"}"#).unwrap()
}
fn project() -> Project {
    serde_json::from_str(
        r#"{"relay":"wss://project.example.com","channel":{"name":"test","id":"test-channel"}}"#,
    )
    .unwrap()
}

#[test]
fn precedence_and_environment_settings() {
    let mut env = Env::from([("BUZZ_KIT_RELAY".into(), "wss://env.example.com".into())]);
    let flags = Flags {
        relay: Some("wss://flag.example.com".into()),
        ..Flags::default()
    };
    assert_eq!(
        resolve(personal(), project(), &flags, &env)
            .unwrap()
            .relay
            .as_deref(),
        Some("https://flag.example.com")
    );
    assert_eq!(
        resolve(personal(), project(), &Flags::default(), &env)
            .unwrap()
            .relay
            .as_deref(),
        Some("https://env.example.com")
    );
    env.clear();
    assert_eq!(
        resolve(personal(), project(), &Flags::default(), &env)
            .unwrap()
            .relay
            .as_deref(),
        Some("https://project.example.com")
    );
    assert_eq!(
        resolve(personal(), Project::default(), &Flags::default(), &env)
            .unwrap()
            .relay
            .as_deref(),
        Some("https://personal.example.com")
    );
    env.insert("BUZZ_KIT_AUTOPOST".into(), "status".into());
    env.insert("BUZZ_KIT_KEYSTORE".into(), "file".into());
    let c = resolve(personal(), project(), &Flags::default(), &env).unwrap();
    assert_eq!(c.autopost, Autopost::Status);
    assert_eq!(c.backend, Backend::File);
}

#[test]
fn runtime_selection_explicit_override_and_ambiguity() {
    let mut env = Env::from([("CODEX_THREAD_ID".into(), "test-thread".into())]);
    let c = resolve(personal(), project(), &Flags::default(), &env).unwrap();
    assert_eq!(active_assistant(&c, &env).unwrap(), "r2d2");
    env.insert("CLAUDECODE".into(), "1".into());
    assert!(active_assistant(&c, &env).is_err());
    let flags = Flags {
        assistant: Some("other".into()),
        ..Flags::default()
    };
    assert_eq!(
        active_assistant(&resolve(personal(), project(), &flags, &env).unwrap(), &env).unwrap(),
        "other"
    );
    env.clear();
    let err = active_assistant(&c, &env).unwrap_err().to_string();
    assert!(err.contains("c3po") && err.contains("r2d2"));
    let mut p = personal();
    p.assistants.insert("claude".into(), "r2d2".into());
    assert_eq!(
        active_assistant(
            &resolve(p, project(), &Flags::default(), &env).unwrap(),
            &env
        )
        .unwrap(),
        "r2d2"
    );
    env.insert("BUZZ_KIT_AS".into(), "env-agent".into());
    assert_eq!(
        active_assistant(
            &resolve(personal(), project(), &Flags::default(), &env).unwrap(),
            &env
        )
        .unwrap(),
        "env-agent"
    );
}

#[test]
fn relay_normalizes_without_accepting_credentials_or_control_characters() {
    assert_eq!(
        normalize_relay("wss://example.com/").unwrap(),
        "https://example.com"
    );
    assert_eq!(
        normalize_relay("ws://localhost:8080/path/").unwrap(),
        "http://localhost:8080/path"
    );
    assert_eq!(
        normalize_relay("https://[::1]:8080").unwrap(),
        "https://[::1]:8080"
    );
    for bad in [
        "file:///tmp/x",
        "https://",
        "https://u:password@example.com",
        "https://example.com?token=secret",
        "https://example.com#x",
        "https://example.com\n",
        "https://example.com\\evil",
        "https://example.com:abc",
        "https://[broken",
    ] {
        assert!(normalize_relay(bad).is_err(), "accepted bad relay");
    }
}

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "buzz-kit-config-{}-{}",
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
fn config_loading_optional_project_explicit_path_and_redacted_errors() {
    let t = Temp::new();
    let home = t.0.join("home");
    let cwd = t.0.join("project/nested");
    fs::create_dir_all(home.join(".config/buzz-kit")).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    let personal_path = home.join(".config/buzz-kit/config.json");
    fs::write(&personal_path, "{}").unwrap();
    assert!(
        load(&home, &cwd, &Flags::default(), &Env::new())
            .unwrap()
            .project
            .channel
            .is_none()
    );
    fs::create_dir_all(cwd.parent().unwrap().join(".buzz")).unwrap();
    fs::write(
        cwd.parent().unwrap().join(".buzz/config.json"),
        r#"{"relay":"wss://project.example.com"}"#,
    )
    .unwrap();
    assert_eq!(
        load(&home, &cwd, &Flags::default(), &Env::new())
            .unwrap()
            .relay
            .as_deref(),
        Some("https://project.example.com")
    );
    let custom = t.0.join("custom.json");
    fs::write(&custom, r#"{"relay":"https://custom.example.com"}"#).unwrap();
    let flags = Flags {
        config: Some(custom),
        ..Flags::default()
    };
    assert_eq!(
        load(&home, &cwd, &flags, &Env::new())
            .unwrap()
            .relay
            .as_deref(),
        Some("https://custom.example.com")
    );
    for bad in [
        "{",
        r#"{"autopost":"private-value-do-not-echo"}"#,
        r#"{"keystore":"private-value-do-not-echo"}"#,
        r#"{"private_key":"private-value-do-not-echo"}"#,
    ] {
        fs::write(&personal_path, bad).unwrap();
        let err = load(&home, &cwd, &flags, &Env::new())
            .err()
            .expect("invalid config accepted");
        assert!(!format!("{err:#}").contains("private-value-do-not-echo"));
    }
}

#[test]
fn invalid_environment_is_rejected_without_echo() {
    let env = Env::from([("BUZZ_KIT_KEYSTORE".into(), "do-not-echo".into())]);
    let err = resolve(personal(), project(), &Flags::default(), &env)
        .err()
        .unwrap();
    assert!(!format!("{err:#}").contains("do-not-echo"));
}
