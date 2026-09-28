use crate::guards::Scanner;
use anyhow::{Result, ensure};
pub const MAX_BYTES: usize = 65_536;
const PREFERRED_BYTES: usize = 60_000;

pub fn prepare(bytes: &[u8], split: bool, scanner: &Scanner) -> Result<Vec<Vec<u8>>> {
    // Always scan the complete input before cutting or returning any sendable part.
    scanner.check(bytes)?;
    let text =
        std::str::from_utf8(bytes).map_err(|_| anyhow::anyhow!("message must be valid UTF-8"))?;
    ensure!(!text.is_empty(), "message is empty");
    if bytes.len() <= MAX_BYTES {
        return Ok(vec![bytes.to_vec()]);
    }
    ensure!(split, "message exceeds 65,536 bytes; use --split");
    let mut slices = Vec::new();
    let mut remaining = text;
    while remaining.len() > PREFERRED_BYTES {
        let mut end = PREFERRED_BYTES;
        while !remaining.is_char_boundary(end) {
            end -= 1;
        }
        let prefix = &remaining[..end];
        let heading = prefix.rmatch_indices("\n#").find_map(|(i, _)| {
            let rest = &prefix[i + 1..];
            let hashes = rest.bytes().take_while(|b| *b == b'#').count();
            (hashes <= 6 && rest.as_bytes().get(hashes) == Some(&b' ')).then_some(i + 1)
        });
        let cut = heading
            .or_else(|| prefix.rfind("\n\n").map(|i| i + 2))
            .or_else(|| prefix.rfind('\n').map(|i| i + 1))
            .filter(|i| *i > 0)
            .unwrap_or(end);
        slices.push(&remaining[..cut]);
        remaining = &remaining[cut..];
    }
    if !remaining.is_empty() {
        slices.push(remaining);
    }
    let count = slices.len();
    slices
        .into_iter()
        .enumerate()
        .map(|(i, slice)| {
            let part = format!("(part {} of {count})\n{slice}", i + 1).into_bytes();
            ensure!(part.len() <= MAX_BYTES, "split label exceeds message limit");
            Ok(part)
        })
        .collect()
}
