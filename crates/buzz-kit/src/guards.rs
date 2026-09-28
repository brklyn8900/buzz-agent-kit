use crate::keystore::{KeyStore, Secret};
use anyhow::{Result, ensure};
use regex::bytes::Regex;

pub struct Scanner {
    keys: Vec<Secret>,
    patterns: Regex,
}
impl Scanner {
    pub fn from_store(store: &dyn KeyStore) -> Result<Self> {
        let mut keys = Vec::new();
        for name in store.list()? {
            if let Some(key) = store.get(&name)? {
                keys.push(key);
            }
        }
        let patterns = Regex::new(concat!(
            "nsec1[02-9ac-hj-np-z]{58}|",
            "-----BEGIN [A-Z ]*PRIVATE KEY-----|",
            "ghp_[A-Za-z0-9]{36}|github_pat_[A-Za-z0-9_]{82}|",
            "sk-ant-[A-Za-z0-9_-]{20,}|sk-proj-[A-Za-z0-9_-]{20,}|",
            "AKIA[0-9A-Z]{16}|xox[abprs]-[A-Za-z0-9-]{10,}"
        ))?;
        Ok(Self { keys, patterns })
    }
    pub fn check(&self, bytes: &[u8]) -> Result<()> {
        let exact = self.keys.iter().any(|key| {
            bytes
                .windows(key.expose().len())
                .any(|window| window.eq_ignore_ascii_case(key.expose().as_bytes()))
        });
        ensure!(
            !exact && !self.patterns.is_match(bytes),
            "payload contains a private key or credential; remove it before posting (no override is available)"
        );
        Ok(())
    }
}
