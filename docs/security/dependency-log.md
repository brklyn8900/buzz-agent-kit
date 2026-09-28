# Dependency decisions

## Policy (2026-09-27)

Apply the local supply-chain-hardening playbook and spec §11 before adding any crate, Action, or tool. Obtain Ron's explicit approval for each concrete version/revision before adding it. Ron approved the exact Rust batch below on 2026-09-27. Other dependencies remain gated.

For each proposal record: purpose and why a small local implementation is insufficient; official registry/source and exact version/hash; downloads; maintainer count and ownership-transfer evidence over the last 90 days; release/source activity; advisory and behavioral scan; build/install scripts; transitive count and noteworthy behavior; license; reviewer; approval and residual uncertainties. An unavailable signal stays unknown, never becomes a pass.

## M0 baseline

Uses already installed host CLIs and system utilities only; no third-party package download/install. Local shell tests use the existing shell/utilities. Python 3 may be used interactively to inspect JSON, without introducing it as a shipped runtime dependency.

`cargo-dist` and `shellcheck` are absent. Their installation is deferred until vetted and approved. Planned §11 crates, cargo-audit, cargo-deny, Actions and the Linux Buzz artifact are not yet vetted or approved.

## Approved M1–M2 Rust batch

Review date: 2026-09-27. Reviewer: Codex. Decision: recommend this exact batch, with the limitations below explicitly accepted. Nothing has been added to the kit Cargo manifests or compiled. No extra tool, Action, CI bot, persistent Keychain write, or release is authorized by this proposal.

A temporary resolver-only manifest used the existing Cargo 1.89.0 to resolve and download source archives for inspection. No build script or procedural macro was executed. The resolution has **10 direct crates + 47 transitive packages = 57 registry packages**. Every archive SHA-256 matches its lock checksum; all sources are crates.io. All declared MSRVs fit the installed Rust 1.89.0.

| Direct crate / exact pin | Recent registry downloads* | Listed owners | Published | Dependency closure** |
|---|---:|---|---|---:|
| [clap =4.6.7](https://crates.io/crates/clap/4.6.7) | 236,440,784 | kbknapp, github:rust-cli:maintainers, github:clap-rs:admins | 2026-09-14 | 9 |
| [serde =1.0.229](https://crates.io/crates/serde/1.0.229) | 324,121,223 | dtolnay, github:serde-rs:publish | 2026-07-18 | 6 |
| [serde_json =1.0.151](https://crates.io/crates/serde_json/1.0.151) | 326,091,414 | dtolnay, github:serde-rs:publish | 2026-07-20 | 13 |
| [k256 =0.14.0](https://crates.io/crates/k256/0.14.0) | 18,283,415 | tarcieri, github:rustcrypto:elliptic-curves | 2026-07-08 | 22 |
| [bech32 =0.12.0](https://crates.io/crates/bech32/0.12.0) | 14,529,252 | apoelstra, clarkmoody, TheBlueMatt | 2026-06-12 | 0 |
| [getrandom =0.4.3](https://crates.io/crates/getrandom/0.4.3) | 586,035,994 | dhardy, github:rust-random:maintainers | 2026-06-17 | 3 |
| [sha2 =0.11.0](https://crates.io/crates/sha2/0.11.0) | 267,457,869 | tarcieri, newpavlov, github:rustcrypto:hashes | 2026-03-25 | 11 |
| [anyhow =1.0.104](https://crates.io/crates/anyhow/1.0.104) | 220,568,938 | dtolnay | 2026-07-18 | 0 |
| [regex =1.13.1](https://crates.io/crates/regex/1.13.1) | 256,151,276 | BurntSushi, rust-lang-owner, github:rust-lang-nursery:regex-owners | 2026-07-15 | 4 |
| [zeroize =1.9.0](https://crates.io/crates/zeroize/1.9.0) | 183,295,904 | tarcieri, github:rustcrypto:utils | 2026-06-12 | 0 |

*`recent_downloads` is the registry API snapshot field, not an independently measured usage count. Owner entries can be GitHub teams; entry count is not a verified human maintainer count. **Closure counts include shared/target-conditional dependencies and must not be summed.

### Purpose, features, and source provenance

- **clap =4.6.7**: Typed subcommands, flags and help across the full CLI; robust parsing and diagnostics exceed a small local parser. Features: defaults off; std, derive, help, usage, error-context. Declared source: [clap-rs/clap](https://github.com/clap-rs/clap). Archive VCS reference resolves to [source commit](https://github.com/clap-rs/clap/commit/13f2db5072d600c11d8d6298e4e9ba53ffc6c1ab).
- **serde =1.0.229**: Typed, validated configuration and host-output decoding; implementing a general serialization framework locally is inappropriate. Features: default std; derive. Declared source: [serde-rs/serde](https://github.com/serde-rs/serde). Archive VCS reference resolves to [source commit](https://github.com/serde-rs/serde/commit/7fc3b4c30c94f73a96ebd1553f2b090d928fc3a8).
- **serde_json =1.0.151**: Strict JSON parsing and order-preserving settings merges; a small bespoke parser would be unsafe. Features: default std; preserve_order. Declared source: [serde-rs/json](https://github.com/serde-rs/json). Archive VCS reference resolves to [source commit](https://github.com/serde-rs/json/commit/de8500740cdcabffb9734f503e4889def823cf10).
- **k256 =0.14.0**: secp256k1 secret/public-key derivation; use reviewed cryptography rather than implement curve arithmetic. Features: defaults off; arithmetic only (Buzz handles signing). Declared source: [RustCrypto/elliptic-curves](https://github.com/RustCrypto/elliptic-curves). Archive VCS reference resolves to [source commit](https://github.com/RustCrypto/elliptic-curves/commit/788867b46f04f58304a07316ebcc8c5a0e05fce3).
- **bech32 =0.12.0**: Nostr npub encoding/decoding with standard checksum validation and test vectors. Features: default std. Declared source: [rust-bitcoin/rust-bech32](https://github.com/rust-bitcoin/rust-bech32). Archive omits VCS metadata; the packaged `src/lib.rs` matches the official `bech32-0.12.0` [tag commit](https://github.com/rust-bitcoin/rust-bech32/commit/50a11bf10734434598385b9c19f1d3856fa9ed4f) byte-for-byte.
- **getrandom =0.4.3**: Operating-system cryptographic randomness across macOS and Linux; no custom entropy implementation. Features: std. Declared source: [rust-random/getrandom](https://github.com/rust-random/getrandom). Archive VCS reference resolves to [source commit](https://github.com/rust-random/getrandom/commit/5e7cd5733536844a9856dc7259bd4696bbe5e3ae).
- **sha2 =0.11.0**: SHA-256 verification of cached/release bytes; do not implement a cryptographic hash locally. Features: defaults off. Declared source: [RustCrypto/hashes](https://github.com/RustCrypto/hashes). Archive VCS reference resolves to [source commit](https://github.com/RustCrypto/hashes/commit/ffe093984c004769747e998f77da8ff7c0e7a765).
- **anyhow =1.0.104**: Consistent contextual failure propagation across filesystem, subprocess, JSON and keystore boundaries; no secret values may enter contexts. Features: default std. Declared source: [dtolnay/anyhow](https://github.com/dtolnay/anyhow). Archive VCS reference resolves to [source commit](https://github.com/dtolnay/anyhow/commit/1dbe1862aae650423e3361fbd20b7d17c5109cc3).
- **regex =1.13.1**: Auditable implementation of the specified secret-pattern corpus; avoids bespoke matchers and their boundary errors. Features: defaults off; std, perf (ASCII secret patterns). Declared source: [rust-lang/regex](https://github.com/rust-lang/regex). Archive VCS reference resolves to [source commit](https://github.com/rust-lang/regex/commit/2b527599eb9eea0dcc288c704584f242f26a5c61).
- **zeroize =1.9.0**: Compiler-resistant secret-buffer clearing; ordinary manual stores may be optimized away. Features: default alloc; std; no derive macro. Declared source: [RustCrypto/utils](https://github.com/RustCrypto/utils). Archive VCS reference resolves to [source commit](https://github.com/RustCrypto/utils/commit/0b715735a660a8566ccd240bf42489fe2ed98efb).

### Integrity, advisories, and behavioral review

- [Exact 57-package proposal](dependency-proposal-2026-09-27.json) records every version, registry checksum, license, MSRV, procedural macro and build-script hash. Approval is for this snapshot; re-resolve/review any changed package or feature before adding it.
- [OSV querybatch](https://google.github.io/osv.dev/post-v1-querybatch/) checked all 57 exact package versions: **57 results, zero advisory matches, no remaining pages**. This is a dated advisory result, not a guarantee against undisclosed flaws. cargo-audit/cargo-deny remain later CI gates and are not installed by this proposal.
- Manually inspected all ten discovered build-script entry points and scanned the selected source trees for network, process, filesystem and credential-name capabilities. Network hits were serde network-type re-exports and a serde_json documentation example. No credential-name hits. Legitimate compiler probes, generated files, error examples and optional serialization I/O were present. This is a focused source review and heuristic scan, not a full independent code audit.
- [Socket clap](https://socket.dev/cargo/package/clap) and [Socket k256](https://socket.dev/cargo/package/k256) public pages were inspected but their visible versions lag the selected releases. No current-version green behavioral rating is claimed. An independent current behavioral scanner result remains unavailable.
- Nine archives carry VCS commits which resolve in their declared upstream repositories. bech32 uses the separately verified official release tag. Recent direct-release histories were inspected; no five-patches-in-a-week pattern appears among the latest stable releases reviewed.
- Allowed license choices in the resolved set are MIT, Apache-2.0, BSD-3-Clause and Unicode-3.0 as applicable; use the permissive branch of multi-license choices. No LGPL option is required.

### Approved build-script exceptions

| Exact crate | Reviewed behavior |
|---|---|
| anyhow 1.0.104 | rustc version/feature probes; temporary compiler output and cleanup under OUT_DIR |
| getrandom 0.4.3 | Reads Cargo sanitizer configuration and emits cfg flags |
| libc 0.2.189 | Target/ABI and rustc probing; also attempts emcc version discovery and optional FreeBSD detection via PATH |
| num-traits 0.2.19 | autocfg compiler probe for floating-point total_cmp |
| proc-macro2 1.0.107 | rustc and feature probes; temporary OUT_DIR files, compiler wrapper support, cleanup |
| quote 1.0.47 | rustc version probe and diagnostic cfg flag |
| serde 1.0.229 / serde_core 1.0.229 | Generate private module under OUT_DIR and inspect rustc compatibility |
| serde_json 1.0.151 | Reads target arch/pointer width and emits arithmetic cfg flags |
| zmij 1.0.23 | rustc/optimization-level compatibility probes |

Build-script code hashes are in the proposal JSON. The procedural macros `clap_derive 4.6.7` and `serde_derive 1.0.229` are also executable compiler inputs and part of this approval; they have not been executed. Build environments must not expose production secrets, and must use the reviewed lock with `--locked`. No network downloader was found in the reviewed build entry points.

### Explicit limitations to accept or resolve

- **Ownership history:** crates.io exposes current owners but does not establish absence of transfers during the preceding 90 days. That checklist item remains unverified for every crate. Known owner/team identities and source commits are evidence, not proof of unchanged control.
- **Single-owner concentration:** anyhow has one listed owner, dtolnay, who also participates in serde and macro tooling. Justification: widely used, narrowly scoped error handling, verified source and reviewed compatibility build script. This remains a concentration risk.
- **Release age:** sha2 0.11.0 was published 2026-03-25, just beyond the playbook six-month recency preference. It remains the registry latest stable release; a mature hash implementation does not need gratuitous updates.
- **Behavioral scan:** current-version Socket coverage is not verified. The alternative performed here is targeted manual build-script review plus capability scanning and OSV checks; full independent assurance is not claimed.
- **No build proof yet:** source/metadata resolution succeeded, but compilation waits for approval. Any required dependency change discovered during build returns through this gate.

Approval requested: all ten exact direct pins, the 47 exact transitive entries, the listed build-script/procedural-macro exceptions, and the stated review limitations. **Approved by Ron on 2026-09-27**, including the exact graph, compiler inputs and stated limitations. This approval permits compilation of this batch. Other tools, Actions, dependency changes and releases remain separate gates.

## Approved M2.7 Linux Buzz artifact

Review date: 2026-09-27. Reviewer: Codex. Decision: recommend the exact **x86_64** artifact below with the explicit limitations in this entry. Downloaded and extracted for static inspection only; not installed, executed, or added as a shipped dependency. Approval applies only to `install-buzz-cli` and optional bot-key CI. The default webhook notifier requires no Buzz download.

### Purpose and exact provenance

Use the official Buzz CLI for protocol/signing compatibility rather than reproduce its authenticated API locally. Extract only `usr/bin/buzz`; do not install the desktop package or its other sidecars.

- Official publisher: [block/buzz](https://github.com/block/buzz), Apache-2.0, active public repository. GitHub snapshot: 35,116 stars, 4,620 forks; first contributor page lists 100 accounts with multiple substantial contributors. This is not an exact authorized-maintainer count. CODEOWNERS names `@block/buzz-oss-team`; team membership and ownership-transfer history over the last 90 days are not publicly established.
- [Desktop release 0.5.25](https://github.com/block/buzz/releases/tag/desktop-v0.5.25), published 2026-09-24; tag commit `c8f73213089cbd5a0f1e675d3193558280d46e10`. Recent release history and five commits preceding the tag show ongoing work from multiple named contributors; reviewed releases do not show five patches in one week. Asset download count was 283 at inspection, a snapshot rather than monthly usage.
- Exact asset: `Buzz_0.5.25_amd64.deb`, 123,075,866 bytes, from `https://github.com/block/buzz/releases/download/desktop-v0.5.25/Buzz_0.5.25_amd64.deb`.
- SHA-256, independently recomputed from downloaded bytes and equal to the GitHub API/page digest: `0990e351453d7eb31e50a498df8efced5ede57931bb414fc50cc9ca56d672293`.
- Extracted `usr/bin/buzz`: 17,907,320 bytes; SHA-256 `f30dc8744e49faa3b4e1703fbc26f6af9285b36dcca498b41eb3680b5b9a25e6`.
- Upstream Cargo metadata still names `block/sprout`; requesting that repository through GitHub resolves to `block/buzz`. This is a stale renamed-repository reference, not evidence of a different publisher.

### Archive, behavior, dependencies, and advisories

- System `ar` and `tar` inspected the archive without executing package code. Members are `debian-binary`, `control.tar.gz`, and `data.tar.gz`. Control archive contains only `control` and `md5sums`: no maintainer install/remove scripts. Its maintainer field is the uninformative `you`; publisher trust comes from the official release source, not that field.
- CLI member is a regular executable, not a link. Desktop GTK/WebKit dependencies belong to the full package. CLI ELF dynamic dependencies are `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and `ld-linux-x86-64.so.2`; symbol versions include `GLIBC_2.38`. Minimum runtime compatibility must be checked before installation and proved on the later Linux test machine.
- Read the CLI entrypoint, manifests, release workflow and focused source paths for subprocess, environment, credential and file access across 33 CLI Rust files. Expected capabilities include relay HTTP/WebSocket access, signing from supplied credentials, optional file reads/uploads, and desktop channel-template reads. No CLI subprocess launcher or Keychain lookup surfaced in this focused scan. Kit command allowlists and credential guards remain necessary. This is not a complete source or binary audit.
- Conservative recursive CLI Cargo.lock closure: **335 packages including the root, 330 registry packages and five local workspace packages**. This includes dev/target/feature-unified edges, so it overestimates a particular CLI target and is not a binary SBOM. A network/signing CLI has a substantially larger graph than the kit wrapper.
- OSV queried all 330 exact registry versions: two matches, no remaining pages. [`cmov` 0.5.3 / GHSA-3rjw-m598-pq24](https://github.com/advisories/GHSA-3rjw-m598-pq24) describes incorrect conditional moves on **aarch64**, fixed in 0.5.4; this proposed artifact is x86_64. [`instant` 0.1.13 / RUSTSEC-2024-0384](https://rustsec.org/advisories/RUSTSEC-2024-0384.html) is an unmaintained-library notice, reached from `nostr`. The kit's own approved lock already uses `cmov` 0.5.4. No claim of a clean advisory result for upstream.
- Upstream [security page](https://github.com/block/buzz/security) lists no published repository advisories at review time. This does not supersede dependency advisories. No current binary-specific Socket behavioral rating is available; the manual capability/archive review above is the limited substitute.
- Upstream release workflow builds the CLI with other sidecars, runs `cargo update --workspace`, and builds without `--locked`. Transitive build scripts ran upstream, not during this review. We cannot establish exact source-to-binary reproducibility from the tag lockfile.
- `gh attestation verify <downloaded-deb> --repo block/buzz` failed with GitHub attestation API **HTTP 404**. Release API reports `immutable: false`. Pinning the reviewed bytes prevents silent replacement from being accepted, but is not signed build provenance. This limitation applies to this upstream dependency; it does not relax the kit's own required release attestations.

### Proposed installation limits and approval scope

- Support automatic extraction on Linux **x86_64 with glibc >= 2.38**. Refuse other architectures and incompatible libc clearly before replacing anything. No ARM64 Linux `.deb` exists in this release; inspected release workflow also builds only Linux x64. Keep all four kit binary targets as specified; users on other Linux targets must supply a compatible Buzz executable through the existing discovery path.
- Use existing system `curl`, `ar`, and `tar`, checking availability. No `dpkg-deb` download, package-manager invocation, root operation, desktop installation, or new crate. Verify the pinned outer digest and extracted CLI bytes; stage and atomically install only the CLI under the user cache.
- Approval requested for this artifact/hash and the documented upstream provenance, maintenance, architecture and libc limitations. **Approved by Ron on 2026-09-27**, including the documented limitations. No spec §2 change proposed; document these upstream installer constraints in usage and build notes. If broader automatic Linux installation is required, a separate upstream artifact/build proposal must be vetted and approved.
