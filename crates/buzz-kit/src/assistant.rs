use crate::{
    config::validate_name,
    identity::{self, Identity},
    keystore::{KeyStore, Secret, validate_service},
};
use anyhow::{Result, ensure};

fn required(store: &dyn KeyStore, name: &str) -> Result<Secret> {
    validate_name(name)?;
    store
        .get(name)?
        .ok_or_else(|| anyhow::anyhow!("assistant key not found"))
}
pub fn create(store: &dyn KeyStore, name: &str) -> Result<Identity> {
    validate_name(name)?;
    ensure!(
        store.get(name)?.is_none(),
        "assistant already exists; refusing replacement"
    );
    let secret = identity::generate()?;
    let public = identity::public(&secret)?;
    store.put(name, &secret)?;
    Ok(public)
}
pub fn show(store: &dyn KeyStore, name: &str) -> Result<Identity> {
    identity::public(&required(store, name)?)
}
pub fn remove(store: &dyn KeyStore, name: &str, confirmed: bool) -> Result<()> {
    validate_name(name)?;
    ensure!(
        confirmed,
        "deletion requires confirmation; pass --yes for noninteractive use"
    );
    required(store, name)?;
    store.delete(name)
}
pub fn import(
    source: &dyn KeyStore,
    account: &str,
    dest: &dyn KeyStore,
    name: &str,
) -> Result<Identity> {
    validate_name(name)?;
    let secret = required(source, account)?;
    let public = identity::public(&secret)?;
    if let Some(existing) = dest.get(name)? {
        if existing.expose() == secret.expose() {
            return Ok(public);
        }
        let existing_public = identity::public(&existing)?;
        anyhow::bail!(
            "destination differs: legacy public key {}, destination public key {}; no changes made",
            public.hex,
            existing_public.hex
        );
    }
    // Each backend's put owns rollback of an incomplete creation. Never delete
    // after a put error: another writer may have won a no-overwrite race.
    dest.put(name, &secret)?;
    match dest.get(name) {
        Ok(Some(copy))
            if copy.expose() == secret.expose() && identity::public(&copy)? == public =>
        {
            Ok(public)
        }
        _ => {
            dest.delete(name).map_err(|_| {
                anyhow::anyhow!(
                    "import verification failed; new-copy cleanup failed; legacy is untouched"
                )
            })?;
            anyhow::bail!(
                "import read-back verification failed; new copy removed; legacy is untouched"
            )
        }
    }
}
pub fn cleanup(
    source: &dyn KeyStore,
    account: &str,
    dest: &dyn KeyStore,
    name: &str,
    confirmed: bool,
) -> Result<()> {
    ensure!(
        confirmed,
        "legacy cleanup requires explicit confirmation; pass --yes only after callers are re-verified"
    );
    let legacy = required(source, account)?;
    let copy = required(dest, name)?;
    ensure!(
        legacy.expose() == copy.expose(),
        "legacy and destination keys differ; refusing cleanup"
    );
    identity::public(&copy)?;
    source.delete(account)
}
pub fn legacy_parts(value: &str) -> Result<(&str, &str)> {
    let (service, account) = value
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("legacy reference must be service/account"))?;
    validate_service(service)?;
    validate_name(account)?;
    ensure!(
        service != "buzz-kit",
        "legacy source must differ from the destination service"
    );
    Ok((service, account))
}
