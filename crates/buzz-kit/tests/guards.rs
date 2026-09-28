use anyhow::Result;
use buzz_kit::{
    guards::Scanner,
    keystore::{KeyStore, Secret},
    post::{self, MAX_BYTES},
};
use zeroize::Zeroizing;
struct Store(bool);
impl KeyStore for Store {
    fn put(&self, _: &str, _: &Secret) -> Result<()> {
        unreachable!()
    }
    fn get(&self, _: &str) -> Result<Option<Secret>> {
        Ok(Some(Secret::from_hex(Zeroizing::new("a".repeat(64)))?))
    }
    fn delete(&self, _: &str) -> Result<()> {
        unreachable!()
    }
    fn list(&self) -> Result<Vec<String>> {
        Ok(if self.0 { vec!["test".into()] } else { vec![] })
    }
}
#[test]
fn specified_secret_patterns_and_exact_keys_are_blocked_without_echo() {
    let scanner = Scanner::from_store(&Store(true)).unwrap();
    let corpus = [
        format!("nsec1{}", "q".repeat(58)),
        "-----BEGIN RSA PRIVATE KEY-----".into(),
        "-----BEGIN PRIVATE KEY-----".into(),
        format!("ghp_{}", "a".repeat(36)),
        format!("github_pat_{}", "a".repeat(82)),
        format!("sk-ant-{}", "a".repeat(20)),
        format!("sk-proj-{}", "a".repeat(20)),
        format!("AKIA{}", "A".repeat(16)),
        format!("xoxb-{}", "a".repeat(10)),
        "a".repeat(64),
        "A".repeat(64),
    ];
    for secret in corpus {
        let payload = format!("before {secret} after");
        let err = scanner.check(payload.as_bytes()).unwrap_err().to_string();
        assert!(!err.contains(&secret));
    }
    assert!(
        scanner.check("b".repeat(64).as_bytes()).is_ok(),
        "event IDs must not be blocked"
    );
    assert!(
        scanner
            .check(b"ordinary discussion of ghp_ and a short sk-ant-example")
            .is_ok()
    );
}
#[test]
fn scan_full_payload_before_any_split_and_keep_utf8_and_byte_limits() {
    let scanner = Scanner::from_store(&Store(false)).unwrap();
    let mut text = "x".repeat(59_995);
    text.push_str(&format!("ghp_{}", "a".repeat(36)));
    text.push_str(&"x".repeat(10_000));
    assert!(post::prepare(text.as_bytes(), true, &scanner).is_err());
    let safe = "🦀".repeat(40_000);
    let parts = post::prepare(safe.as_bytes(), true, &scanner).unwrap();
    assert!(parts.len() > 1);
    let mut joined = String::new();
    for (i, part) in parts.iter().enumerate() {
        assert!(part.len() <= MAX_BYTES);
        let text = std::str::from_utf8(part).unwrap();
        let label = format!("(part {} of {})\n", i + 1, parts.len());
        joined.push_str(text.strip_prefix(&label).unwrap());
    }
    assert_eq!(joined, safe);
}
#[test]
fn boundaries_no_split_and_preferred_markdown_cuts() {
    let scanner = Scanner::from_store(&Store(false)).unwrap();
    let exact = vec![b'x'; MAX_BYTES];
    assert_eq!(post::prepare(&exact, false, &scanner).unwrap(), vec![exact]);
    assert!(post::prepare(&vec![b'x'; MAX_BYTES + 1], false, &scanner).is_err());
    assert!(post::prepare(&[0xff], false, &scanner).is_err());
    let text = format!("{}\n# Heading\n{}", "x".repeat(50_000), "y".repeat(30_000));
    let parts = post::prepare(text.as_bytes(), true, &scanner).unwrap();
    assert!(
        std::str::from_utf8(&parts[1])
            .unwrap()
            .contains("\n# Heading\n")
    );
    assert_eq!(
        post::prepare(b"small", true, &scanner).unwrap(),
        vec![b"small".to_vec()]
    );
}
