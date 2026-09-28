use crate::guards::Scanner;
use anyhow::{Result, ensure};
pub const MAX_BYTES: usize = 65_536;
const PREFERRED_BYTES: usize = 60_000;

pub struct Request {
    pub channel: String,
    pub thread: Option<String>,
    pub split: bool,
    pub kind: Option<u16>,
}
pub fn send(
    client: &crate::buzz::Buzz,
    store: &dyn crate::keystore::KeyStore,
    secret: &crate::keystore::Secret,
    request: &Request,
    bytes: &[u8],
) -> Result<Vec<String>> {
    if let Some(thread) = &request.thread {
        ensure!(
            event_id(thread),
            "thread must be a 64-character hex event ID"
        );
    }
    let scanner = Scanner::from_store(store)?;
    let parts = prepare(bytes, request.split, &scanner)?;
    let channel_type = if request.kind.is_none() {
        let channel = client.channel(&request.channel, secret)?;
        let result = client.read_json(
            &["channels", "search", "--query", &channel.name, "--exact"],
            secret,
        )?;
        let channels = result
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Buzz channel search shape changed"))?;
        let matching: Vec<_> = channels
            .iter()
            .filter(|c| c["channel_id"].as_str() == Some(&request.channel))
            .collect();
        ensure!(
            matching.len() == 1,
            "cannot uniquely resolve channel metadata"
        );
        match matching[0]["channel_type"].as_str() {
            Some("stream") => "stream",
            Some("forum") => "forum",
            _ => anyhow::bail!("channel type is unknown; verify it and provide --kind explicitly"),
        }
    } else {
        "explicit"
    };
    let mut root = request.thread.clone();
    let mut ids = Vec::new();
    for part in parts {
        let kind = request
            .kind
            .unwrap_or(if channel_type == "forum" {
                if root.is_some() { 45003 } else { 45001 }
            } else {
                9
            })
            .to_string();
        let mut args = vec![
            "messages",
            "send",
            "--channel",
            request.channel.as_str(),
            "--kind",
            &kind,
            "--content",
            "-",
        ];
        if let Some(root) = &root {
            args.extend(["--reply-to", root]);
        }
        let sent = (|| -> Result<String> {
            let response = client.execute(&args, secret, Some(&part))?;
            ensure!(
                response["accepted"].as_bool() == Some(true),
                "relay did not accept the message"
            );
            let id = response["event_id"]
                .as_str()
                .filter(|id| event_id(id))
                .ok_or_else(|| anyhow::anyhow!("Buzz send response has no valid event ID"))?;
            Ok(id.to_owned())
        })();
        match sent {
            Ok(id) => {
                if root.is_none() {
                    root = Some(id.clone());
                }
                ids.push(id);
            }
            Err(_) => anyhow::bail!(
                "posting stopped after {} confirmed part(s); delivery of the current part is uncertain; do not retry automatically; confirmed event IDs: {}",
                ids.len(),
                if ids.is_empty() {
                    "none".into()
                } else {
                    ids.join(", ")
                }
            ),
        }
    }
    Ok(ids)
}
fn event_id(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

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
