use buzz_kit::host;
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "buzz-kit-host-{}-{}",
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
fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn kit(path: &std::path::Path) {
    fs::create_dir_all(path.join("bin")).unwrap();
    fs::create_dir_all(path.join("release")).unwrap();
    fs::write(path.join("bin/buzz-kit"), "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(path.join("bin/buzz-kit"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        path.join("plugin.json"),
        r#"{"name":"buzz-kit","version":"0.0.0-m0"}"#,
    )
    .unwrap();
    fs::write(path.join("release/checksums.txt"), "fixture presence only").unwrap();
}
#[test]
fn real_host_shapes_resolve_only_listed_verified_locations() {
    let t = Temp::new();
    let dir = t.0.join("claude-kit");
    kit(&dir);
    let mut cp = fixture("claude-plugin-list.json");
    cp[0]["installPath"] = dir.to_str().unwrap().into();
    cp[0]["futureField"] = true.into();
    let cm = fixture("claude-marketplace-list.json");
    let report = host::claude(&cp.to_string(), &cm.to_string());
    assert_eq!(report.candidates.len(), 1);
    assert_eq!(report.candidates[0].kit_dir, dir);
    let home = t.0.join("codex");
    let dir = home.join("plugins/cache/buzz-agent-kit/buzz-kit/0.0.0-m0");
    kit(&dir);
    let dp = fixture("codex-plugin-list.json");
    let dm = fixture("codex-marketplace-list.json");
    assert_eq!(
        host::codex(&dp.to_string(), &dm.to_string(), &home).candidates[0].kit_dir,
        dir
    );
    fs::remove_file(dir.join("release/checksums.txt")).unwrap();
    assert!(
        host::codex(&dp.to_string(), &dm.to_string(), &home)
            .candidates
            .is_empty()
    );
}
#[test]
fn codex_missing_source_wrong_repo_duplicate_marketplace_and_traversal_fail_closed() {
    let t = Temp::new();
    kit(&t.0.join("plugins/cache/buzz-agent-kit/buzz-kit/0.0.0-m0"));
    let dp = fixture("codex-plugin-list.json");
    let dm = fixture("codex-marketplace-list.json");
    for source in [
        Value::Null,
        serde_json::json!({"sourceType":"git","source":"https://github.com/attacker/buzz-agent-kit.git"}),
        serde_json::json!({"sourceType":"git","source":"https://github.com.evil/brklyn8900/buzz-agent-kit"}),
    ] {
        let mut bad = dm.clone();
        bad["marketplaces"][0]["marketplaceSource"] = source;
        assert!(
            host::codex(&dp.to_string(), &bad.to_string(), &t.0)
                .candidates
                .is_empty()
        );
    }
    let mut duplicate = dm.clone();
    duplicate["marketplaces"]
        .as_array_mut()
        .unwrap()
        .push(dm["marketplaces"][0].clone());
    assert!(
        host::codex(&dp.to_string(), &duplicate.to_string(), &t.0)
            .candidates
            .is_empty()
    );
    for field in ["version", "name", "marketplaceName"] {
        let mut bad = dp.clone();
        bad["installed"][0][field] = "../escape".into();
        assert!(
            host::codex(&bad.to_string(), &dm.to_string(), &t.0)
                .candidates
                .is_empty()
        );
    }
}
#[test]
fn disabled_plugins_and_mismatched_manifest_are_not_candidates() {
    let t = Temp::new();
    kit(&t.0);
    let mut p = fixture("claude-plugin-list.json");
    p[0]["installPath"] = t.0.to_str().unwrap().into();
    let m = fixture("claude-marketplace-list.json");
    p[0]["enabled"] = false.into();
    let r = host::claude(&p.to_string(), &m.to_string());
    assert!(r.candidates.is_empty());
    assert!(r.notices.iter().any(|s| s.contains("disabled")));
    p[0]["enabled"] = true.into();
    fs::write(
        t.0.join("plugin.json"),
        r#"{"name":"buzz-kit","version":"9.9.9"}"#,
    )
    .unwrap();
    assert!(
        host::claude(&p.to_string(), &m.to_string())
            .candidates
            .is_empty()
    );
    for bad in ["{", "{}", "null", r#"[{"id":"buzz-kit@buzz-agent-kit"}]"#] {
        assert!(host::claude(bad, &m.to_string()).candidates.is_empty());
    }
}
