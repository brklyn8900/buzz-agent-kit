# buzz-agent-kit Implementation Plan

> Execution: use superpowers:executing-plans, task by task, on `master`, as Ron requested. No additional plan-approval gate. Record red/green evidence and a conventional commit after each task in `docs/build-notes.md`.

**Goal:** Implement the approved spec, pass M4 acceptance items 1–9, and publish v0.1.0 after explicit release approval.

**Architecture:** One Rust binary delegates protocol operations to Block's Buzz CLI. Two plugin manifests share skills and a POSIX launcher; all entry points use one explicitly managed binary cache and shim. Host adapters discover installed plugins from host CLI output, never by scanning caches.

**Tech stack:** Rust, POSIX sh, system curl, platform keystores, Claude Code and Codex plugin CLIs. Crates, Actions, and additional tools remain proposals until individually vetted and approved.

**Spec:** `docs/superpowers/specs/2026-09-27-buzz-agent-kit-design.md` (approved by Ron's implementation request; the file's older status line is unchanged).

## Global constraints and gates

- macOS and Linux only; default branch `master`; conventional commit after every task.
- Complete M0's real-host exit check before M1. Complete each milestone before the next.
- Preserve the approved spec. Propose the smallest correction when evidence contradicts it; obtain approval for any spec edit, particularly §2.
- Ask before repo creation, first push, every tag push, and every release publication, including RC releases. Local commits do not authorize any of those actions.
- Before adding any crate, Action, or tool: apply the supply-chain playbook and §11; record exact version, source, justification, downloads, owners/history, behavior/build scripts, transitives, advisory/behavior scan, and uncertainty in `docs/security/dependency-log.md`; obtain approval.
- Ask before nonthrowaway Keychain writes and before using a live scratch channel. Never read the desktop identity store. Never print/write assistant private keys or use `--broadcast`.
- Public artifacts contain no live-server details, channel UUIDs, secrets, or private community content. Existing input documents and Git history require a publication audit before the first push.
- No edits in the adjacent deployment repository. Migration caller changes go in `docs/handoff/koinosbuzz-migration.md`; cleanup waits for Ron's confirmation of caller reverification.
- Commit Cargo.lock; exact direct versions; all builds use `--locked`; no hooks or MCP servers. All added Actions have full SHA pins and least privilege.
- Release targets: aarch64-apple-darwin, x86_64-apple-darwin, x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu. Never move published tags.
- M0 uses a clearly labeled local development stub. It must never report production doctor or cryptographic verification success.
- v1.1 live agents, server operations, and Windows remain outside this finish line.

## Review focus

1. Paths containing spaces, symlinks, different working directories, absent HOME subdirectories: M0.2/M2.5 test that paths remain quoted and resolve to the actual kit.
2. Malformed host output, absent fields, repository impersonation, disabled plugins, path traversal in host metadata: M1.4/M2.6 must fail closed without cache scans.
3. Partial writes and interruption: M1.2/M2.4/M2.5 prove existing configuration, keys, foreign shims, and current binaries survive failures.
4. UTF-8, secret patterns spanning proposed split boundaries, and flags embedded in passthrough arguments: M2.1/M2.2 scan full bytes before splitting and never pass unchecked writes.
5. An installed plugin is not proof of loaded skills or working shell PATH: M0.3/M3.3/M4.4 require fresh-session evidence; stubs never substitute for acceptance.

## File ownership and interfaces

| Files | Responsibility and interface |
|---|---|
| `.claude-plugin/{plugin,marketplace}.json`, `.agents/plugins/marketplace.json`, `plugin.json` | Shared plugin identity/version and host-specific catalog metadata |
| `.version-bump.json`, `scripts/bump-version`, `scripts/check-version` | Canonical version in Claude manifest; explicit file/field map including Cargo and catalogs where versioned |
| `bin/buzz-kit`, `install.sh` | Launcher bootstrap and installer; argument/exit status preservation; explicit-path calls |
| `tests/m0.sh`, `tests/launcher.sh`, `tests/installer.sh` | Isolated shell contract tests; no network or live credentials |
| `crates/buzz-kit/src/{main,cli,config,identity,assistant,doctor}.rs` | Parse CLI; resolve configuration/identity; ordered diagnostic report |
| `crates/buzz-kit/src/keystore/{mod,keychain,secret_service,file}.rs` | `KeyStore { put, get, delete, list }`; secrets never in Debug/output |
| `crates/buzz-kit/src/{buzz,guards,post,init,update,host,ci_bot,install_buzz}.rs` | Guarded subprocess boundary, message bytes, merge-only writes, host discovery and explicit updates |
| `crates/buzz-kit/tests/`, `tests/fixtures/` | Public anonymized CLI fixtures; integration tests using fake subprocesses/stores |
| `skills/{setup,room}/SKILL.md`, `templates/`, `docs/` | Human/agent workflows; generic examples only |
| `dist-workspace.toml`, `deny.toml`, `.github/workflows/`, `release/checksums.txt` | Audited builds, two immutable release tags, attested artifact checks |

Rust tests use the public command boundary or small module APIs; define precise internal types when their task begins, after real CLI observations. Do not build speculative adapters from help text alone. Secret-bearing process responses stay in memory; only public schema/exit summaries enter fixtures.

## M0 — package and host proof

### M0.1: Native CLI inventory and plugin skeleton

Files: the four manifest/catalog JSON files, `.version-bump.json`, `skills/setup/SKILL.md`, `tests/m0.sh`, `docs/build-notes.md`, anonymized host fixtures.

- [x] Run installed `claude --version`, `codex --version`, plugin add/list/marketplace help and list JSON. Record versions, output field types, optional fields, and absence of cargo-dist/shellcheck without installing anything.
- [x] Write `tests/m0.sh` assertions for the required files, two manifest names/versions, both catalog identities, shared skill presence, no hook/MCP declarations, and executable launcher/installer. Run `sh tests/m0.sh`; expect missing-file failure.
- [x] Add the minimal manifests/catalogs, version map, and a setup skill explicitly marked M0-only. Use `AVAILABLE`/`ON_INSTALL` for the native catalog.
- [x] Run `claude plugin validate .`; expect exit 0. Validate JSON with already installed tooling, without adding a package.
- [x] Capture public fixtures from real list output, preserving shape and replacing account paths/repos/IDs. Never save raw host config or environment dumps.
- [x] Commit `chore: scaffold dual-host plugin packaging` after the skeleton checks pass.

### M0.2: Development bootstrap and installer smoke tests

Files: `bin/buzz-kit`, `scripts/m0-dev-binary`, `install.sh`, `tests/m0.sh`.

- [x] Write isolated shell tests: absent shim/cache, HOME with spaces, invocation outside checkout, explicit bootstrap, 3-line marked shim, foreign shim refusal without mutation, untagged install refusal, and `--dry-run` without writes. Expect failure before implementation.
- [x] Implement only M0's local stub bootstrap using an explicitly supplied development binary. Use the §5 shared-cache layout and atomic current symlink. Do not download or claim release verification.
- [x] Implement installer ordering: identify checkout tag (or explicit `--dev`), preflight host name collisions, explicit-path bootstrap, exact-tag host installs, absolute shim doctor, clear fresh-session/PATH instructions. `--dry-run` must be mutation-free.
- [x] Run `sh -n bin/buzz-kit install.sh scripts/m0-dev-binary`; run `sh tests/m0.sh`; expect all assertions pass. Record that development doctor is a stub.
- [x] Commit `feat: add development bootstrap for M0 host proof`.

### M0.3: Real tag installation and fresh-session discovery

Files: `docs/build-notes.md`; temporary probe workspaces outside published content.

- [x] Audit all would-be public files AND reachable Git history. Prepare the exact generic replacement/publication proposal for any private input; preserve originals locally. Obtain approval before modifying the approved spec or rewriting history.
- [x] Present the concrete public tree, branch commit, and operations. Ask to create `brklyn8900/buzz-agent-kit`; separately obtain first-push approval. A local catalog install is useful evidence but does not prove the specified GitHub tag path.
- [x] Obtain approval for an M0-only tag push, clearly distinct from `v0.0.1-rc1` and final releases. Do not consume the immutable RC tag before the real release pipeline exists.
- [x] From a clean isolated user environment on this Mac, verify no shim, cache, or local-bin PATH entry. Checkout the approved tag and invoke `install.sh` from another directory with the M0 dev binary. Record installation outputs and plugin list entries from both real CLIs.
- [x] Start fresh Claude/Codex sessions. Invoke the placeholder setup skill and run `command -v buzz-kit` plus stub doctor. Record only relevant runtime marker names/booleans and path results, not all environment values.
- [x] Probe native-vs-Claude catalog priority in throwaway copies with distinguishable entries; probe missing/mismatched entries. Probe `INSTALLED_BY_DEFAULT` only in an isolated catalog, then restore `AVAILABLE` in the shipping tree.
- [x] Probe literal `PLUGIN_ROOT` substitution with a harmless diagnostic skill; separately test `bin/` PATH before adding the shim path. Record each §14 result as confirmed, contradicted, or unresolved with evidence.
- [x] Update dependent tasks from findings and commit `docs: record M0 dual-host tag acceptance` only when observations are recorded.

**M0 exit:** Real tag-ref install and loaded skill in BOTH hosts, starting with no shim/cache; shell and both runtimes resolve the development command. Quote commands, versions, tag/commit, and outcomes. No M1 work until this passes.

## M1 — Rust core and identities

### M1.1: Approve dependencies, crate entrypoint, and configuration

Files: `docs/security/dependency-log.md`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/buzz-kit/{Cargo.toml,src/main.rs,src/cli.rs,src/config.rs}`.

- [x] Vet each proposed direct crate from §11 at an exact version and its dependency graph; record unresolved ownership/behavior signals honestly. Obtain Ron's approval before adding any crate or toolchain download.
- [x] Write tests for flags > env > project > personal; invalid JSON/enums; missing optional project; explicit config path; relay normalization; runtime mapping > sole assistant fallback; ambiguous choices. Run `cargo test --locked`; expect assertion failures.
- [x] Implement typed parsing/config resolution and runtime marker behavior proven in M0. Use nonempty CODEX_THREAD_ID for Codex detection (M0 verified). Keep secrets out of configuration and errors. Add canonical version checks including Cargo.
- [x] Run `cargo test --locked` and `cargo build --locked`; commit `feat: add CLI and configuration resolution`.

### M1.2: Keystore backends and safe subprocess handling

Files: `src/keystore/`, backend tests, public CLI-shape notes.

- [x] Inspect `/usr/bin/security` command help; test `security -i` only with a unique throwaway service/account and synthetic secret held in memory. Verify read-back and cleanup; no real assistant key is involved.
- [x] Write fake-store tests for put/get/delete/list, service denylist (`buzz-desktop` and prefix), malformed names, errors without secret echo, and secret zeroing. Write real file tests for 0700/0600, wrong owner, symlink refusal, and partial-write preservation.
- [x] Implement Keychain stdin command quoting with narrowly validated identifiers; secret-service stdin storage; opt-in file backend. Auto selection errors rather than silently downgrading to files.
- [x] Run unit suite and ignored throwaway Keychain test. Record exact output/exit without secret values. Commit `feat: add guarded platform keystores`.

### M1.3: Assistant lifecycle, public identity, and copy-only import

Files: `src/{identity,assistant}.rs`, `tests/assistant.rs`.

- [x] Write known-vector secp256k1 public-key and npub round-trip tests; duplicate new refusal; show/list public-only; remove confirmation.
- [x] Write migration fake-store cases: identical destination no-op, different destination refusal naming public keys, failed write/read-back removes only newly created copy, legacy untouched, cleanup refuses unequal keys.
- [x] Implement `assistant new/list/show/remove/import/cleanup-legacy`, secret buffers zeroed and never formatted. Import never removes source; cleanup is separate and confirmed.
- [x] Run `cargo test --locked`; commit `feat: add assistant identity lifecycle and safe migration`.

### M1.4: Doctor, Buzz discovery, and host adapters

Files: `src/{doctor,buzz,host}.rs`, `tests/fixtures/`, doctor/adapter tests.

- [x] Check actual Buzz help/output shapes using synthetic/local responses first and authorized read calls later. Record HTTP relay semantics, error exit codes, channel metadata and JSON wrappers. Do not infer successful server JSON from help.
- [x] Write ordered doctor assertions for platform, Buzz lookup order, config, NIP-11 reachability via curl, readable key, membership (exit 3), visible channel, backend, current/pin, host status, and PATH warning without failure.
- [x] Write host fixture tests for Claude-only/Codex-only/both; unknown/missing fields; missing marketplace source; same-name wrong repo; disabled plugin; manifest mismatch; path traversal; only listed installed locations.
- [x] Implement strict `Candidate { host, version, launcher, kit_dir }` / `Unavailable(reason)` adapters per observed schemas. Never scan caches; validate manifests and checksums file presence.
- [x] Run unit suite and all-green throwaway-config doctor where infrastructure exists; commit `feat: add ordered diagnostics and host discovery`.

**M1 exit:** Rust suite and manual throwaway Keychain test pass. Record the security stdin result for §14; propose and approve a fallback dependency if needed.

## M2 — guarded collaboration and binary lifecycle

### M2.1: Exact-byte guards and splitter

Files: `src/guards.rs`, `src/post.rs`, unit tests.

- [x] Write scanner corpus for every §8 pattern and exact keys, negative 64-hex event IDs, UTF-8, 65,536-byte boundary, and secrets crossing split boundaries. Test all split labels and 60 KB preferred cuts (heading, blank line, newline) without broken UTF-8.
- [x] Implement whole-payload scan before splitting, no override, and numbered chunks each within 65,536 bytes including labels.
- [x] Run `cargo test --locked`; commit `feat: guard and split message payloads`.

### M2.2: Buzz process boundary, safe reads, and profiles

Files: `src/buzz.rs`, `src/assistant.rs`, passthrough/profile integration tests.

- [x] Inspect installed `buzz messages {get,thread,search,send} --help`, channel read commands and `users set-profile --help`; verify local synthetic JSON/error shapes now; real-server verification remains at M2.3’s authorized live gate.
- [x] Write tests for every allowed read pair and refusal of write/upload/edit/delete/unknown pairs, broadcast anywhere, and attempts to override private-key/relay identity through passthrough options. Child args never contain a key.
- [x] Implement read/search/as and guarded profile fields using one subprocess boundary. Inject key only in child environment; sanitize inherited auth variables and errors. Preserve safe CLI stdout and exits.
- [x] Run the suite; commit `feat: add read-only Buzz passthrough and guarded profiles`.

### M2.3: Posting, thread kinds, and live smoke

M1 CLI finding: `channels get` omits type. Use `channels search --query <exact name> --exact`, match the configured channel ID, and consume its verified `channel_type`; unknown/missing type must not silently default to a stream.

Files: `src/post.rs`, payload integration tests, build notes.

- [x] Write stdin-capture tests proving checked bytes equal sent bytes and no file path/--file argument reaches Buzz. Cover stream 9, forum root 45001/reply 45003, explicit kind, first split root, supplied thread root and later-part failure.
- [x] Implement post, preserving returned event IDs and surfacing partial-send information without automatic duplication/retry.
- [x] Ask Ron for a private scratch channel UUID and permission for required persistent assistant-key creation/import. Keep those inputs outside repo/logs. Verify membership before the first post.
- [x] Post a synthetic claim and reply only in the scratch thread; read back and verify public author and root/reply relationships. Record sanitized evidence, not channel UUID/server address or private content.
- [x] Run suite and commit guarded posting implementation; live author/thread gate now passed (see build notes).

### M2.4: Project init and merge-only teammate settings

Files: `src/init.rs`, config/settings tests, example template.

- [x] Write every §5.1a conflict-table test, including alias marketplace names, false enabled flag, differing ref without/with explicit update, wrong repo, invalid JSON, existing key order, no forbidden new keys, and atomic failure behavior.
- [x] Write reachability/channel visibility tests, no-verify and no-claude-settings behavior, existing project preservation and trailing newline.
- [x] Implement `.buzz/config.json` and settings with preflight validation before mutation; print actual marketplace name and pinned Codex commands.
- [x] Run suite; commit `feat: initialize project and merge teammate settings safely`.

### M2.5: Verified launcher, cache, and shim

Files: `bin/buzz-kit`, `tests/launcher.sh`, `release/checksums.txt`.

- [x] Replace M0-only behavior via failing shell tests for all four targets, unsupported target, checksum mismatch cleanup/current unchanged, cache hit, development override, optional gh attestation refusal, symlink invocation and different cwd.
- [x] Add tests for missing local-bin, foreign shim, interrupted/invalid archive extraction, unsafe tar paths, cached executable integrity, and atomic current switch. Use synthetic archives; no unapproved downloads.
- [x] Implement §5 bootstrap from checkout-owned checksums, immutable target cache, marked 3-line shim and shell-specific PATH guidance. Ordinary invocation executes current without implicit version update.
- [x] Run shell suite, sh syntax checks and shellcheck only after approved installation if unavailable. Commit `feat: verify release bootstrap and shared binary cache`.

### M2.6: Update discovery, rollback, pin, and pruning

Files: `src/update.rs`, host/launcher integration tests.

- [x] Write plugin-list-driven upgrade tests, obsolete higher cache ignored, manifest mismatch unavailable, highest enabled semantic version across hosts, no candidates and already-current outcomes.
- [x] Write rollback integrity, pin-before-any-mutation, byte-identical refused update, explicit unpin, failure preservation, current/pin/two-recent preservation and no-prune tests.
- [x] Implement explicit candidate launcher bootstrap, safe version components and pruning only after success. Doctor reports newer offered binaries and current/pin.
- [x] Run Rust/shell suites; commit `feat: add explicit updates rollback and pinning`.

### M2.7: Pinned Linux Buzz CLI installer

Files: `src/install_buzz.rs`, artifact dependency-log entry and tests.

- [x] Vet exact official desktop .deb asset/hash and installed extraction tools for install-buzz-cli and optional bot-key CI mode; obtain dependency approval. The default webhook notifier has no .deb dependency. Never copy an old background checksum without current official verification.
- [x] After approval of the exact x86_64 artifact and its limits, write tests for checksum refusal, unsupported architecture/libc refusal (upstream requires glibc >= 2.38), extraction-only usr/bin/buzz, install location and discovery precedence; macOS existing-app guidance. Use existing ar/tar, no new extraction tool. Other Linux architectures require a supplied compatible Buzz CLI; all four kit binary targets remain required.
- [x] Implement pinned artifact installation without package-manager/root operations. Run suite; commit `feat: install checksum-pinned Buzz CLI on Linux`.

**M2 exit:** All guard, allowlist, payload, merge, bootstrap and update checks pass; real scratch-thread post verified. No substituted/faked live evidence.

## M3 — skills, templates, installer, and setup

### M3.1: Shared skills and generic documentation

Files: both skills, `templates/{buzz.config.example.json,AGENTS.snippet.md}`, `docs/{collaboration,operators}.md`, README, LICENSE.

- [x] Write frontmatter/packaging checks for name/folder/length and forbidden hook/MCP references. Check templates contain only generic values.
- [x] Replace M0 placeholder with setup's ordered doctor/fix flow, public identity handoff, profile, runtime config, init, channel display-name/hex guidance and optional webhook-default CI notification setup (bot-key is an explicit alternative). Never request a private key or webhook secret pasted into chat.
- [x] Implement room skill with untrusted-channel boundary, read/search/claim/thread/change-only updates, ask/status/off, five templates, no attachments, no broadcast and size limits.
- [x] Document all operator caveats from §10; manual pinned installs and DO_NOT_TRACK=1 skills-only fallback limitations; Apache-2.0 license.
- [x] Validate manifests/skills and inspect rendered docs; commit `docs: add setup room and operator workflows`.

### M3.2: Webhook-default CI initialization and optional bot-key mode

Files: `src/ci_bot.rs`, `src/doctor.rs`, `templates/github/buzz-notify.yml`, `templates/github/buzz-notify.bot-key.yml`, tests, operator docs.

Approved change: spec commit `0e44110`; do not implement the superseded bot-key-only default.

- [ ] Inspect actual `buzz workflows create/update/get/list --help`, real/local response shapes and `gh secret set`/variable help before coding. Add synthetic fixtures for the JSON `message` containing `response:{...}`. Secrets must stay in memory and never enter fixture output, files or errors.
- [ ] Write red tests for default webhook selection and explicit bot-key mode. Webhook creation must use only the fixed send_message template and flat trigger fields repo/event/title/url/actor; it must not accept arbitrary workflow payloads from channel content.
- [ ] Write fake-process tests proving the webhook secret and optional bot key go only to `gh secret set` stdin; no argv/log/file echo; URL is a separate variable; existing workflow files are preserved; partial creation/export failures report recoverable public workflow IDs without exposing secrets or silently creating duplicates.
- [ ] Write event-matrix tests for PR open/reopen/ready/merged/closed, issue open/closed, release published and failed CI only. Malicious titles remain data through env and `jq -n --arg`; payload is flat JSON; `curl --fail-with-body`; no third-party Actions, default permissions empty, and missing fork secrets skip safely.
- [ ] Implement `ci-bot init --mode webhook|bot-key`, default webhook. Create the Buzz webhook workflow in the configured project channel as the active assistant; parse/redact the nested secret; set BUZZ_WEBHOOK_SECRET and BUZZ_WEBHOOK_URL; write the default template. Track public workflow metadata for doctor visibility warnings without storing the secret.
- [ ] Implement optional named bot-key mode with the separately vetted .deb template, stdin-only key export, relay/channel variables, operator membership/channel steps and local-key deletion offer. Do not require .deb download, bot identity or membership setup for webhook mode.
- [ ] Verify §14 secret rotation behavior using an explicitly authorized disposable scratch workflow; document results and relay-authored message display. Keep ci-bot rotate out of scope unless approved after findings. Document owner-membership dependency and why built-in GitHub webhooks are unsupported.
- [ ] Run suite; commit `feat: initialize webhook and optional bot-key notifications`. Real workflow creation/GitHub secret writes use their explicit authorized test scope; persistent Keychain writes retain Ron's gate.

### M3.3: Production installer and fresh setup walkthrough

Files: `install.sh`, `tests/installer.sh`, docs evidence.

- [ ] Extend M0 installer tests for Claude-only/Codex-only/both/neither, tag/ref agreement, dry-run, all collision types, marketplace backup/preservation, host failure, absolute doctor, and missing PATH as warning.
- [ ] Replace M0 assumptions with real verified bootstrap and production doctor. Finish with distinct Installed/Next steps and fresh-session instructions; never edit shell rc files.
- [ ] Run authorized tag installation and fresh setup skill in both hosts. Record missing human/operator steps honestly and complete them before claiming the walkthrough passed.
- [ ] Run all suites and plugin validation; commit `feat: complete verified dual-host installation`.

**M3 exit:** Fresh-session setup walkthrough succeeds in Claude and Codex with real doctor evidence.

## M4 — release pipeline, migration, and acceptance

### M4.1: CI and two-tag release pipeline

Files: approved dependency log, `deny.toml`, `dist-workspace.toml`, `.github/workflows/`, version/checksum scripts.

- [ ] Vet exact cargo-dist, cargo-audit, cargo-deny and each Action revision/transitives; obtain approval before installation/addition. Then run actual `cargo dist --help`/plan output before generating workflow configuration.
- [ ] Write version mismatch, malformed checksum, wrong/missing target, mismatched bin/plugin version and attest-failure checks. Test that checksum PR changes only `release/checksums.txt`.
- [ ] Configure native locked builds, `github-attestations = true`, `tag-namespace = "bin"`, four targets; pin generated Actions, permissions per job, no auto-merge. CI runs audit/deny/tests/format/manifests/frontmatter/shell checks.
- [ ] Configure approved public repository immutable releases and protected v*/bin-v* tags. Preserve human review of artifact-checksum PRs.
- [ ] Commit `ci: add audited builds and two-tag releases`; push only under existing explicit approval.

### M4.2: Real RC pipeline before final acceptance

Files: versioned manifests/Cargo, checksums via reviewed PR, build evidence.

- [ ] Prepare 0.0.1-rc1 on master with unchanged previous checksums (bootstrap first-release state must be explicitly documented/validated; do not invent trusted hashes).
- [ ] Ask before pushing bin-v0.0.1-rc1 and publishing its binary release. Verify four published downloads/attestations; generated checksum-only PR receives human review and is merged.
- [ ] Ask before pushing v0.0.1-rc1 and publishing the plugin release. Verify installed launcher fetches the named binary artifacts and committed hashes. Acceptance 9 passes only here.
- [ ] Repeat approved full sequence for rc2, required by acceptance 8. Do not skip or reuse tags.
- [ ] Record links/digests/run IDs and commit `docs: record RC release provenance and installation`.

### M4.3: Migration implementation and C3PO handoff

Files: `docs/handoff/koinosbuzz-migration.md`, sanitized evidence.

- [ ] Obtain approval for persistent destination Keychain writes. Run copy-and-read-back import for both assistant identities; report only public-key equality and exits.
- [ ] Write the exact replacement wrapper: parse legacy `--as`; dispatch read commands through `buzz-kit as`; translate existing post calls to guarded `buzz-kit post` with stdin, channel/thread/kind preserved; refuse unsupported writes. Include exact AGENTS/skill replacements and both-runtime scratch read/post reverification commands.
- [ ] Give handoff to Ron for C3PO to apply. Never edit the adjacent repo. Wait for confirmation/evidence of updated callers and both assistants' read/post verification.
- [ ] Only after that confirmation, run explicit cleanup with identical-key check; doctor green for both. Commit `docs: record verified assistant migration and handoff`.

### M4.4: Acceptance environments and all nine evidence records

Files: `docs/acceptance.md`, sanitized fixture/project templates.

- [ ] Acceptance 1: both real plugin installs via install.sh, matching tag, loaded skills.
- [ ] Acceptance 2: ordered import/caller handoff/reverification/cleanup plus green doctors, using M4.3 evidence.
- [ ] Acceptance 3: scratch claim + reply with server-confirmed author and threading.
- [ ] Acceptance 4: throwaway HOME on a Linux machine, file-store permissions, actual pinned CLI installation and green doctor. Obtain machine access through Ron if unavailable; no root SSH/live server changes.
- [ ] Acceptance 5: authorized disposable test repo and scratch-only default webhook notifier; actual PR-opened and merged messages, wrong-secret request rejected, plus one PR-opened message in optional bot-key mode. Verify authorship appropriate to each mode; never post in other channels.
- [ ] Acceptance 6: fresh project clone/workspace trust in Claude, then VS Code/Copilot first chat; record actual pinned recommendation/install prompts. Human UI evidence is required where automation cannot observe it.
- [ ] Acceptance 7: clean user environment with no cache/shim/local-bin/PATH entry, different cwd invocation; actual Installed and green doctor with PATH warning; after printed PATH added by user, fresh shell and both hosts succeed.
- [ ] Acceptance 8: Codex-only account rc1 → rc2 marketplace upgrade/reinstall → newer-version doctor → update via derived adapter → pin refusal with unchanged current.
- [ ] Acceptance 9: link M4.2's real two-tag RC provenance and verified launcher downloads.
- [ ] Each record includes date, command/environment, expected/actual result and public evidence reference; keep private inputs outside repo. Pending is never pass. Commit `test: record full cross-platform release acceptance` when complete.

### M4.5: Publish v0.1.0 and final report

- [ ] Run complete locked Rust/shell/manifest/skill/security gates and fresh independent review. Fix substantive findings, rerun affected suites, and record reviewer/runtime/evidence.
- [ ] Prepare v0.1.0 version PR, obtain needed external-write authorization, pass CI and merge under Ron's release review process.
- [ ] Ask before bin-v0.1.0 tag/release; verify four artifacts/attestations and human-reviewed checksum-only PR.
- [ ] Ask before v0.1.0 tag/release; verify pinned tag/checksum/assets consistency and published install documentation.
- [ ] Recheck published install/download and acceptance evidence affected by final-version changes. Commit `docs: record v0.1.0 release verification` if evidence changes.
- [ ] Final response: each acceptance item 1–9 PASS/FAIL with evidence, release link, approved spec deviations and open follow-ups. Distinguish implementation, tests, CI, publication and human acceptance.

**M4 exit / 100%:** Every acceptance item passes and v0.1.0 is publicly published after approval. A blocked human/release gate leaves the milestone incomplete.
