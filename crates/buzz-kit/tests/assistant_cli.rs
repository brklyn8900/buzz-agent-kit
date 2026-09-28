use std::{
    fs,
    process::{Command, Output, Stdio},
};

#[test]
fn lifecycle_cli_outputs_public_json_and_requires_delete_confirmation() {
    let home = std::env::temp_dir().join(format!("buzz-kit-cli-{}", std::process::id()));
    fs::create_dir(&home).unwrap();
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(home.clone());
    let run = |args: &[&str]| -> Output {
        Command::new(env!("CARGO_BIN_EXE_buzz-kit"))
            .args(args)
            .env_clear()
            .env("HOME", &home)
            .env("BUZZ_KIT_KEYSTORE", "file")
            .current_dir(&home)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    };
    let output = run(&["assistant", "new", "test-agent", "--json"]);
    assert!(
        output.status.success(),
        "new failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let identity: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(identity["hex"].as_str().unwrap().len(), 64);
    assert!(identity["npub"].as_str().unwrap().starts_with("npub1"));
    let secret = zeroize::Zeroizing::new(
        fs::read_to_string(home.join(".config/buzz-kit/keys/test-agent.key")).unwrap(),
    );
    for args in [
        vec!["assistant", "show", "test-agent", "--json"],
        vec!["assistant", "list", "--json"],
        vec!["assistant", "new", "test-agent"],
    ] {
        let output = run(&args);
        assert!(!String::from_utf8_lossy(&output.stdout).contains(secret.as_str()));
        assert!(!String::from_utf8_lossy(&output.stderr).contains(secret.as_str()));
    }
    assert!(!run(&["assistant", "remove", "test-agent"]).status.success());
    assert!(run(&["assistant", "show", "test-agent"]).status.success());
    assert!(
        run(&["assistant", "remove", "test-agent", "--yes"])
            .status
            .success()
    );
    assert!(!run(&["assistant", "show", "test-agent"]).status.success());
}
