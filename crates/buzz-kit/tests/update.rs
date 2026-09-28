use buzz_kit::{
    host::Candidate,
    update::{self, Options},
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "buzz-kit-update-{}-{}",
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
fn cache(data: &Path, tag: &str, time: u64) {
    let dir = data.join("bin").join(tag);
    fs::create_dir_all(&dir).unwrap();
    let bytes = b"#!/bin/sh\necho fixture\n";
    fs::write(dir.join("buzz-kit"), bytes).unwrap();
    fs::set_permissions(dir.join("buzz-kit"), fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        dir.join("binary.sha256"),
        format!(
            "{}\n",
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
    )
    .unwrap();
    fs::write(dir.join("archive.sha256"), format!("{}\n", "a".repeat(64))).unwrap();
    fs::write(dir.join("target"), "aarch64-apple-darwin\n").unwrap();
    fs::write(dir.join("installed-at"), time.to_string()).unwrap();
}
fn candidate(root: &Path, version: &str, tag: &str, fail: bool) -> Candidate {
    let kit = root.join(format!("plugin-{version}"));
    fs::create_dir_all(kit.join("bin")).unwrap();
    fs::create_dir_all(kit.join("release")).unwrap();
    let mut manifest = format!("bin_tag: {tag}\n");
    for target in [
        "aarch64-apple-darwin",
        "x86_64-apple-darwin",
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
    ] {
        manifest.push_str(&format!("{}  buzz-kit-{target}.tar.gz\n", "a".repeat(64)));
    }
    fs::write(kit.join("release/checksums.txt"), manifest).unwrap();
    let script = if fail {
        "#!/bin/sh\nexit 1\n".into()
    } else {
        format!(
            "#!/bin/sh\nset -eu\n[ \"$1\" = bootstrap ]\n[ -z \"${{BUZZ_KIT_BIN:-}}\" ]\nrm -f \"$XDG_DATA_HOME/buzz-kit/current\"\nln -s 'bin/{tag}' \"$XDG_DATA_HOME/buzz-kit/current\"\n"
        )
    };
    let launcher = kit.join("bin/buzz-kit");
    fs::write(&launcher, script).unwrap();
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
    Candidate {
        host: "test".into(),
        version: version.into(),
        launcher,
        kit_dir: kit,
    }
}
#[test]
fn semantic_selection_ignores_obsolete_cache_and_orders_prereleases() {
    let t = Temp::new();
    let candidates = vec![
        candidate(&t.0, "0.1.0-rc.2", "bin-v0.1.0-rc.2", false),
        candidate(&t.0, "0.1.0-rc.10", "bin-v0.1.0-rc.10", false),
    ];
    assert_eq!(update::highest(candidates).unwrap().version, "0.1.0-rc.10");
    let release = candidate(&t.0, "0.1.0", "bin-v0.1.0", false);
    assert_eq!(
        update::highest(vec![
            release,
            candidate(&t.0, "0.1.0-rc11", "bin-v0.1.0-rc11", false)
        ])
        .unwrap()
        .version,
        "0.1.0"
    );
    assert!(update::highest(vec![]).is_err());
}
#[test]
fn pin_refuses_before_discovery_and_changes_no_bytes() {
    let t = Temp::new();
    let data = t.0.join("share/buzz-kit");
    cache(&data, "bin-v0.1.0", 1);
    symlink("bin/bin-v0.1.0", data.join("current")).unwrap();
    fs::write(data.join("pin"), "bin-v0.1.0\n").unwrap();
    let before = fs::read(data.join("pin")).unwrap();
    let link = fs::read_link(data.join("current")).unwrap();
    assert!(
        update::run(&t.0, &data, &Options::default(), || panic!(
            "discovery ran before pin refusal"
        ))
        .is_err()
    );
    assert_eq!(fs::read(data.join("pin")).unwrap(), before);
    assert_eq!(fs::read_link(data.join("current")).unwrap(), link);
}
#[test]
fn rollback_checks_integrity_and_persists_pin_then_explicit_unpin_upgrades() {
    let t = Temp::new();
    let data = t.0.join("share/buzz-kit");
    cache(&data, "bin-v0.1.0", 1);
    cache(&data, "bin-v0.2.0", 2);
    symlink("bin/bin-v0.2.0", data.join("current")).unwrap();
    update::run(
        &t.0,
        &data,
        &Options {
            to: Some("bin-v0.1.0".into()),
            ..Options::default()
        },
        || panic!("rollback must not discover plugins"),
    )
    .unwrap();
    assert_eq!(
        fs::read_link(data.join("current")).unwrap(),
        PathBuf::from("bin/bin-v0.1.0")
    );
    assert_eq!(
        fs::read_to_string(data.join("pin")).unwrap().trim(),
        "bin-v0.1.0"
    );
    let c = candidate(&t.0, "0.2.0", "bin-v0.2.0", false);
    update::run(
        &t.0,
        &data,
        &Options {
            unpin: true,
            ..Options::default()
        },
        || Ok(vec![c]),
    )
    .unwrap();
    assert!(!data.join("pin").exists());
    assert_eq!(
        fs::read_link(data.join("current")).unwrap(),
        PathBuf::from("bin/bin-v0.2.0")
    );
    fs::write(data.join("bin/bin-v0.1.0/buzz-kit"), "bad").unwrap();
    assert!(
        update::run(
            &t.0,
            &data,
            &Options {
                to: Some("bin-v0.1.0".into()),
                ..Options::default()
            },
            || panic!()
        )
        .is_err()
    );
    assert!(!data.join("pin").exists());
    assert_eq!(
        fs::read_link(data.join("current")).unwrap(),
        PathBuf::from("bin/bin-v0.2.0")
    );
}
#[test]
fn failed_upgrade_keeps_pin_and_current_and_success_prunes_only_old_versions() {
    let t = Temp::new();
    let data = t.0.join("share/buzz-kit");
    for n in 1..=5 {
        cache(&data, &format!("bin-v0.{n}.0"), n);
    }
    symlink("bin/bin-v0.1.0", data.join("current")).unwrap();
    fs::write(data.join("pin"), "bin-v0.1.0\n").unwrap();
    let c = candidate(&t.0, "0.5.0", "bin-v0.5.0", true);
    assert!(
        update::run(
            &t.0,
            &data,
            &Options {
                unpin: true,
                ..Options::default()
            },
            || Ok(vec![c])
        )
        .is_err()
    );
    assert!(data.join("pin").exists());
    assert_eq!(
        fs::read_link(data.join("current")).unwrap(),
        PathBuf::from("bin/bin-v0.1.0")
    );
    let c = candidate(&t.0, "0.5.0", "bin-v0.5.0", false);
    update::run(
        &t.0,
        &data,
        &Options {
            unpin: true,
            ..Options::default()
        },
        || Ok(vec![c]),
    )
    .unwrap();
    assert!(data.join("bin/bin-v0.5.0").exists());
    assert!(data.join("bin/bin-v0.4.0").exists());
    assert!(data.join("bin/bin-v0.3.0").exists());
    assert!(!data.join("bin/bin-v0.2.0").exists());
}

#[test]
fn malformed_current_from_launcher_is_rolled_back() {
    let t = Temp::new();
    let data = t.0.join("share/buzz-kit");
    cache(&data, "bin-v0.1.0", 1);
    cache(&data, "bin-v0.2.0", 2);
    symlink("bin/bin-v0.1.0", data.join("current")).unwrap();
    let c = candidate(&t.0, "0.2.0", "bin-v0.2.0", false);
    fs::write(&c.launcher,"#!/bin/sh\nrm -f \"$XDG_DATA_HOME/buzz-kit/current\"\nln -s ../../invalid \"$XDG_DATA_HOME/buzz-kit/current\"\n").unwrap();
    assert!(update::run(&t.0, &data, &Options::default(), || Ok(vec![c])).is_err());
    assert_eq!(
        fs::read_link(data.join("current")).unwrap(),
        PathBuf::from("bin/bin-v0.1.0")
    );
}

#[test]
fn no_prune_up_to_date_and_doctor_offer_ignore_unlisted_cache() {
    let t = Temp::new();
    let data = t.0.join("share/buzz-kit");
    for n in [1, 2, 3, 4, 99] {
        cache(&data, &format!("bin-v0.{n}.0"), n);
    }
    symlink("bin/bin-v0.1.0", data.join("current")).unwrap();
    let c = candidate(&t.0, "0.2.0", "bin-v0.2.0", false);
    assert_eq!(
        update::newer_offer(&data, vec![c.clone()]).unwrap(),
        Some("bin-v0.2.0".into())
    );
    update::run(
        &t.0,
        &data,
        &Options {
            no_prune: true,
            ..Options::default()
        },
        || Ok(vec![c.clone()]),
    )
    .unwrap();
    assert_eq!(
        fs::read_link(data.join("current")).unwrap(),
        PathBuf::from("bin/bin-v0.2.0")
    );
    assert!(data.join("bin/bin-v0.1.0").exists());
    fs::write(&c.launcher, "#!/bin/sh\nexit 77\n").unwrap();
    assert!(
        update::run(&t.0, &data, &Options::default(), || Ok(vec![c.clone()]))
            .unwrap()
            .starts_with("Up to date")
    );
    fs::write(data.join("pin"), "bin-v0.2.0\n").unwrap();
    assert_eq!(
        update::newer_offer(&data, vec![candidate(&t.0, "0.3.0", "bin-v0.3.0", false)]).unwrap(),
        None
    );
}
