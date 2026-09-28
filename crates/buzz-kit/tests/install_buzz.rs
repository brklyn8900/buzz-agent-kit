use buzz_kit::install_buzz::{self, Artifact};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "buzz-kit-deb-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
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
fn hash(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn deb(root: &Path, link: bool) -> PathBuf {
    fs::create_dir_all(root.join("payload/usr/bin")).unwrap();
    if link {
        symlink("/bin/sh", root.join("payload/usr/bin/buzz")).unwrap();
    } else {
        fs::write(root.join("payload/usr/bin/buzz"), b"fixture binary").unwrap();
    }
    fs::write(root.join("payload/usr/bin/other"), b"must not install").unwrap();
    assert!(
        Command::new("tar")
            .current_dir(root)
            .args(["-czf", "data.tar.gz", "-C", "payload", "usr"])
            .status()
            .unwrap()
            .success()
    );
    fs::write(root.join("debian-binary"), b"2.0\n").unwrap();
    assert!(
        Command::new("ar")
            .current_dir(root)
            .args(["qcS", "fixture.deb", "debian-binary", "data.tar.gz"])
            .status()
            .unwrap()
            .success()
    );
    root.join("fixture.deb")
}
#[test]
fn supported_arch_and_glibc_are_explicit() {
    assert!(install_buzz::supported("linux", "x86_64", "glibc 2.38\n").is_ok());
    assert!(install_buzz::supported("linux", "x86_64", "glibc 2.40").is_ok());
    for (os, arch, libc) in [
        ("linux", "aarch64", "glibc 2.40"),
        ("linux", "x86_64", "glibc 2.37"),
        ("linux", "x86_64", "musl 1.2"),
        ("linux", "x86_64", "glibc nope"),
        ("macos", "x86_64", "glibc 2.40"),
    ] {
        assert!(install_buzz::supported(os, arch, libc).is_err());
    }
}
#[test]
fn installs_only_verified_cli_and_reuses_intact_cache() {
    let t = Temp::new();
    let archive = deb(&t.0, false);
    let h = hash(&fs::read(&archive).unwrap());
    let b = hash(b"fixture binary");
    let a = Artifact {
        version: "0.5.25",
        archive_hash: &h,
        binary_hash: &b,
    };
    let home = t.0.join("home");
    let path = install_buzz::install_deb(&home, &archive, &a).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"fixture binary");
    assert!(!path.parent().unwrap().join("other").exists());
    assert_eq!(
        fs::canonicalize(home.join(".local/share/buzz-kit/buzz-cli/current")).unwrap(),
        fs::canonicalize(&path).unwrap()
    );
    assert_eq!(
        install_buzz::install_deb(&home, &archive, &a).unwrap(),
        path
    );
    fs::write(&path, b"corrupt").unwrap();
    assert!(install_buzz::install_deb(&home, &archive, &a).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"corrupt");
}
#[test]
fn checksum_and_links_refuse_without_installing() {
    let t = Temp::new();
    let archive = deb(&t.0, false);
    let h = hash(&fs::read(&archive).unwrap());
    let b = hash(b"fixture binary");
    let home = t.0.join("home");
    for a in [
        Artifact {
            version: "0.5.25",
            archive_hash: &"0".repeat(64),
            binary_hash: &b,
        },
        Artifact {
            version: "0.5.25",
            archive_hash: &h,
            binary_hash: &"0".repeat(64),
        },
    ] {
        assert!(install_buzz::install_deb(&home, &archive, &a).is_err());
        assert!(!home.join(".local/share/buzz-kit/buzz-cli/current").exists());
    }
    let u = Temp::new();
    let archive = deb(&u.0, true);
    let h = hash(&fs::read(&archive).unwrap());
    assert!(
        install_buzz::install_deb(
            &home,
            &archive,
            &Artifact {
                version: "0.5.25",
                archive_hash: &h,
                binary_hash: &b
            }
        )
        .is_err()
    );
}
#[test]
fn refuses_symlinked_destination_and_foreign_current() {
    let t = Temp::new();
    let archive = deb(&t.0, false);
    let h = hash(&fs::read(&archive).unwrap());
    let b = hash(b"fixture binary");
    let a = Artifact {
        version: "0.5.25",
        archive_hash: &h,
        binary_hash: &b,
    };
    let home = t.0.join("home");
    let base = home.join(".local/share/buzz-kit/buzz-cli");
    fs::create_dir_all(&base).unwrap();
    fs::write(base.join("current"), b"foreign").unwrap();
    assert!(install_buzz::install_deb(&home, &archive, &a).is_err());
    assert_eq!(fs::read(base.join("current")).unwrap(), b"foreign");
    fs::remove_file(base.join("current")).unwrap();
    symlink(&t.0, base.join("0.5.25")).unwrap();
    assert!(install_buzz::install_deb(&home, &archive, &a).is_err());
}

#[test]
fn explicit_buzz_path_wins_and_invalid_override_never_falls_back() {
    use std::os::unix::fs::PermissionsExt;
    let t = Temp::new();
    let path = t.0.join("selected-buzz");
    fs::write(&path, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let mut env = buzz_kit::config::Env::new();
    env.insert(
        "BUZZ_KIT_BUZZ_CLI".into(),
        path.to_string_lossy().into_owned(),
    );
    assert_eq!(buzz_kit::buzz::discover(&env, &t.0).unwrap(), path);
    env.insert("BUZZ_KIT_BUZZ_CLI".into(), "relative-buzz".into());
    assert!(buzz_kit::buzz::discover(&env, &t.0).is_err());
}

#[test]
#[ignore = "requires the approved official .deb supplied via BUZZ_KIT_TEST_DEB; extracts but never executes"]
fn approved_official_archive_extracts_exact_reviewed_binary() {
    let t = Temp::new();
    let path =
        PathBuf::from(std::env::var_os("BUZZ_KIT_TEST_DEB").expect("supply reviewed deb path"));
    let result = install_buzz::install_deb(
        &t.0,
        &path,
        &Artifact {
            version: install_buzz::VERSION,
            archive_hash: install_buzz::ARCHIVE_HASH,
            binary_hash: install_buzz::BINARY_HASH,
        },
    )
    .unwrap();
    assert_eq!(hash(&fs::read(result).unwrap()), install_buzz::BINARY_HASH);
}
