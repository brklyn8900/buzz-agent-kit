use anyhow::Result;
use buzz_kit::{
    assistant, identity,
    keystore::{KeyStore, Secret},
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};
use zeroize::Zeroizing;

fn key(n: u8) -> Secret {
    Secret::from_hex(Zeroizing::new(format!("{n:064x}"))).unwrap()
}
#[derive(Default)]
struct Store {
    values: RefCell<BTreeMap<String, Secret>>,
    writes: Cell<usize>,
    deletes: Cell<usize>,
    fail_put: Cell<bool>,
    corrupt_readback: Cell<bool>,
}
impl KeyStore for Store {
    fn put(&self, n: &str, s: &Secret) -> Result<()> {
        self.writes.set(self.writes.get() + 1);
        anyhow::ensure!(!self.fail_put.get(), "injected write failure");
        self.values.borrow_mut().insert(
            n.into(),
            Secret::from_hex(Zeroizing::new(s.expose().into()))?,
        );
        Ok(())
    }
    fn get(&self, n: &str) -> Result<Option<Secret>> {
        if self.corrupt_readback.get() && self.writes.get() > 0 {
            return Ok(Some(key(2)));
        }
        self.values
            .borrow()
            .get(n)
            .map(|s| Secret::from_hex(Zeroizing::new(s.expose().into())))
            .transpose()
    }
    fn delete(&self, n: &str) -> Result<()> {
        self.deletes.set(self.deletes.get() + 1);
        self.values.borrow_mut().remove(n);
        Ok(())
    }
    fn list(&self) -> Result<Vec<String>> {
        Ok(self.values.borrow().keys().cloned().collect())
    }
}
fn source() -> Store {
    let s = Store::default();
    s.values.borrow_mut().insert("legacy".into(), key(1));
    s
}

#[test]
fn secp256k1_generator_vector_and_npub_roundtrip() {
    let p = identity::public(&key(1)).unwrap();
    assert_eq!(
        p.hex,
        "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798"
    );
    let (hrp, bytes) = bech32::decode(&p.npub).unwrap();
    assert_eq!(hrp.as_str(), "npub");
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        p.hex
    );
    assert!(identity::public(&key(0)).is_err());
    let generated = identity::generate().unwrap();
    assert_eq!(identity::public(&generated).unwrap().hex.len(), 64);
}

#[test]
fn create_show_duplicate_and_remove_confirmation() {
    let s = Store::default();
    let p = assistant::create(&s, "agent").unwrap();
    assert_eq!(assistant::show(&s, "agent").unwrap(), p);
    assert!(assistant::create(&s, "agent").is_err());
    assert_eq!(s.writes.get(), 1);
    assert!(assistant::remove(&s, "agent", false).is_err());
    assert_eq!(s.deletes.get(), 0);
    assistant::remove(&s, "agent", true).unwrap();
    assert!(s.get("agent").unwrap().is_none());
}

#[test]
fn import_copy_verified_and_idempotent() {
    let source = source();
    let dest = Store::default();
    let public = assistant::import(&source, "legacy", &dest, "agent").unwrap();
    assert_eq!(public, identity::public(&key(1)).unwrap());
    assert_eq!(dest.writes.get(), 1);
    assert_eq!(source.deletes.get(), 0);
    assistant::import(&source, "legacy", &dest, "agent").unwrap();
    assert_eq!(dest.writes.get(), 1);
    assert_eq!(dest.deletes.get(), 0);
}

#[test]
fn import_refuses_different_destination_without_mutation() {
    let source = source();
    let dest = Store::default();
    dest.values.borrow_mut().insert("agent".into(), key(2));
    let error = assistant::import(&source, "legacy", &dest, "agent")
        .unwrap_err()
        .to_string();
    assert!(error.contains(&identity::public(&key(1)).unwrap().hex));
    assert!(error.contains(&identity::public(&key(2)).unwrap().hex));
    assert!(!error.contains(key(1).expose()));
    assert_eq!(dest.writes.get(), 0);
    assert_eq!(dest.deletes.get(), 0);
    assert_eq!(source.deletes.get(), 0);
}

#[test]
fn failed_import_preserves_source_and_cleans_only_created_copy() {
    let source = source();
    let dest = Store::default();
    dest.fail_put.set(true);
    assert!(assistant::import(&source, "legacy", &dest, "agent").is_err());
    assert_eq!(source.deletes.get(), 0);
    assert!(dest.values.borrow().is_empty());
    dest.fail_put.set(false);
    dest.writes.set(0);
    dest.corrupt_readback.set(true);
    assert!(assistant::import(&source, "legacy", &dest, "agent").is_err());
    assert_eq!(dest.deletes.get(), 1);
    assert!(dest.values.borrow().is_empty());
    assert_eq!(source.deletes.get(), 0);
}

#[test]
fn cleanup_requires_identical_secret_not_merely_public_key_and_confirmation() {
    let source = source();
    let dest = Store::default();
    assert!(assistant::cleanup(&source, "legacy", &dest, "agent", true).is_err());
    dest.values.borrow_mut().insert("agent".into(), key(2));
    assert!(assistant::cleanup(&source, "legacy", &dest, "agent", true).is_err());
    dest.values.borrow_mut().insert("agent".into(), key(1));
    assert!(assistant::cleanup(&source, "legacy", &dest, "agent", false).is_err());
    assert_eq!(source.deletes.get(), 0);
    assistant::cleanup(&source, "legacy", &dest, "agent", true).unwrap();
    assert_eq!(source.deletes.get(), 1);
    assert_eq!(dest.deletes.get(), 0);
}
