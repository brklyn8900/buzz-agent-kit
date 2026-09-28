use crate::keystore::Secret;
use anyhow::Result;
use k256::elliptic_curve::sec1::ToSec1Point;
use serde::Serialize;
use zeroize::Zeroizing;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Identity {
    pub hex: String,
    pub npub: String,
}

pub fn public(secret: &Secret) -> Result<Identity> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    for (i, pair) in secret.expose().as_bytes().chunks_exact(2).enumerate() {
        let digit = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
        bytes[i] = digit(pair[0]) * 16 + digit(pair[1]);
    }
    let key = k256::SecretKey::from_slice(&*bytes)
        .map_err(|_| anyhow::anyhow!("invalid secp256k1 assistant key"))?;
    let point = key.public_key().to_sec1_point(true);
    let x = point
        .x()
        .ok_or_else(|| anyhow::anyhow!("invalid public point"))?;
    Ok(Identity {
        hex: x.iter().map(|b| format!("{b:02x}")).collect(),
        npub: bech32::encode::<bech32::Bech32>(bech32::Hrp::parse("npub")?, x)?,
    })
}

pub fn generate() -> Result<Secret> {
    loop {
        let mut bytes = Zeroizing::new([0u8; 32]);
        getrandom::fill(&mut *bytes)?;
        if k256::SecretKey::from_slice(&*bytes).is_err() {
            continue;
        }
        let mut text = Zeroizing::new(String::with_capacity(64));
        const HEX: &[u8] = b"0123456789abcdef";
        for b in bytes.iter() {
            text.push(HEX[(b >> 4) as usize] as char);
            text.push(HEX[(b & 15) as usize] as char);
        }
        return Secret::from_hex(text);
    }
}
