# Build notes

## Authority and current state

- 2026-09-27: Ron approved implementation of the existing design, M0 first, then M1–M4, with conventional commits on master. The spec's older `awaiting review` status is preserved pending permission to edit that file; the implementation request is authoritative.
- Plan: `docs/superpowers/plans/2026-09-27-buzz-agent-kit.md`.
- No repo creation, push, tag push, release, dependency addition, persistent Keychain write, or live-server test is authorized without the requested gate.
- Publication preflight finding: input docs and existing Git history contain deployment-specific details. Before publication, propose generic replacements and a clean-history strategy. Do not publish these inputs or silently rewrite the approved spec/history.

## Plan task — complete

- Read AGENTS.md, the approved spec, and the collaboration background; inspected clean `master` at `1da97a9` and no configured Git remote.
- Read the supply-chain-hardening playbook. No dependency installed or added.
- Created the M0–M4 plan, including every §12 acceptance item, named exit checks and approval boundaries.
- Execution stays native on master as requested; no new worktree or extra plan-approval step. The running log is this file.

## M0.1 — observations (not milestone acceptance)

- `claude --version`: `2.1.283 (Claude Code)`.
- `codex --version`: `codex-cli 0.157.1`.
- `rustc --version`: `rustc 1.89.0 (29483883e 2025-08-04)`; `cargo --version`: `cargo 1.89.0 (c24e10642 2025-06-23)`.
- Already installed: Rust/Cargo, Python 3, gh, Buzz. `cargo-dist` and `shellcheck` absent; not installed.
- Real `claude plugin list --json`, Claude marketplace list, Codex plugin list and Codex marketplace list all exit 0. Claude shapes match §5.3. Codex installed entries lack installPath as expected; its marketplace list contains entries with no marketplaceSource, plus local and git sources. Adapters must handle missing source as unavailable for identity matching, not crash or infer provenance from a cache path.
- Actual Codex marketplace add supports `--ref`; actual plugin add supports `plugin@marketplace` and `--json`. Claude marketplace add accepts paths/URLs/GitHub repos; install supports the expected identifier.
- Current process has marker names `CODEX_CI`, `CODEX_SESSION_ID`, `CODEX_THREAD_ID`, `CODEX_VERSION`. Values were not logged. Presence in this app session is preliminary evidence, not proof of the fresh CLI session contract.

### M0.1 packaging task — complete

- Red: `sh tests/m0.sh --packaging-only` → `FAIL: missing .claude-plugin/plugin.json`.
- Green: same command → `PASS: M0 package files and identity`.
- Added both plugin manifests, both marketplaces, explicit version map and development-only setup skill at `0.0.0-m0`.
- Existing Python JSON parser verified valid JSON and every mapped version equal to `0.0.0-m0`; no package installed or runtime dependency added.
- `claude plugin validate .` → `Validation passed`. Explicit `claude plugin validate .claude-plugin/plugin.json` → passed with the warning that root CLAUDE.md is not loaded as plugin context. Expected: the shared skill carries instructions; no reliance on CLAUDE.md loading.
- Four anonymized fixtures retain actual host output shapes. `tests/fixtures/README.md` records CLI versions and substitutions. They are parser inputs, not buzz-kit installation evidence.
- Shell behavior tests exist but remain red until M0.2. M0 milestone exit remains unpassed.

### M0.2 development bootstrap task — complete

- Red: full `sh tests/m0.sh` → `FAIL: missing executable bin/buzz-kit`.
- Green: `sh tests/m0.sh` passes packaging, bootstrap/current/shim, arguments, symlink invocation, foreign-file refusal, untagged refusal and mutation-free dry-run groups. HOME includes spaces and an apostrophe; repeated bootstrap also passes.
- `sh -n bin/buzz-kit install.sh scripts/m0-dev-binary tests/m0.sh` exits 0.
- Real local-source smoke (2026-09-27): `install.sh --dev` in a temporary HOME with isolated CODEX_HOME/CLAUDE_CONFIG_DIR/XDG_DATA_HOME, called from a different cwd, exits 0. Claude reports `Successfully installed plugin: buzz-kit@buzz-agent-kit (scope: user)`; Codex reports `Added plugin buzz-kit from marketplace buzz-agent-kit`. Both list it enabled at `0.0.0-m0`. The installer prints `Installed ✓ (M0 development package; production checks unavailable)`.
- No original user plugin directories were modified. Temporary installs were removed with their disposable HOME. Repeat with `sh tests/host-smoke.sh` (requires the existing host CLIs).
- M0 stub is opt-in through an absolute `BUZZ_KIT_BIN`; uses `bin/dev-m0`, never a release cache tag. It reports that production doctor is unimplemented and never accesses a server/keystore. M2 replaces this with verified release bootstrap.
- Installer checks both hosts for name collisions before bootstrap; never edits shell rc files. Native host commands own registration. A preexisting Claude marketplace file is backed up before the native add command.
- Remaining M0.3: GitHub tag-ref installation, fresh-session skill loading/PATH/marker observations and catalog policy probes. Local-source installation does not satisfy this exit check.

## §14 evidence register

### M0.3 partial probes and publication gate

- `sh tests/host-smoke.sh` passes both real local-source host installs, bare shell shim resolution after explicit PATH addition, and second-install collision refusal. This is reproducible local evidence, not remote tag or fresh-model-session evidence.
- In a disposable local catalog, assigned different marketplace names to the native and Claude catalogs. `codex plugin marketplace add <catalog> --json` exits 0 and returns the native catalog's name (`buzz-native-probe`). Codex therefore prefers the native catalog in this observed local-source case and does not require the two catalog names to match for registration. Git-source/fresh-runtime behavior remains to be checked.
- The same isolated native catalog used `INSTALLED_BY_DEFAULT`. Immediately after registration, `codex plugin list --json` returned `{"installed": [], "available": []}`. Registration alone did not auto-install it; this does not settle fresh-runtime policy behavior.
- Public audit found five real-hostname references across three input files, present throughout all nine audited commits. A latest-tree edit alone cannot make the original history public-safe. Proposed six-line cleanup (five generic hostname substitutions plus approved status) is reviewable privately; no approved spec or original instruction was changed.
- Stop before M1: M0's tag-ref/fresh-session exit remains incomplete. Next gate is Ron's approval of the source cleanup/private history preservation and public repo creation. First push and M0 tag push remain separate approvals.
- See `docs/security/publication-preflight.md` for the concrete proposal.

| Open item | Status | Evidence / next check |
|---|---|---|
| Codex runtime environment marker | Confirmed | Fresh Codex process with inherited CODEX markers removed injects CODEX_THREAD_ID, CODEX_SESSION_ID, CODEX_CI and CODEX_VERSION; use nonempty CODEX_THREAD_ID for runtime detection |
| PLUGIN_ROOT substitution in skill text | Confirmed for Codex | Fresh session reads literal PLUGIN_ROOT and CLAUDE_PLUGIN_ROOT tokens; skills must use the shim as planned |
| security -i secret on stdin | Deferred to M1 | Help available; no Keychain read/write performed |
| Native vs Claude marketplace precedence / entry matching | Confirmed in discovery | Native catalog wins despite differing catalog names; fresh app-server plugin/list returns native catalog path and identity. Real Git-tag installation accepts both formats present |
| Codex plugin bin on PATH | Confirmed absent | Fresh session diagnostic returns plugin_bin_on_path=no; command resolves through the installed shim |
| INSTALLED_BY_DEFAULT in repo marketplace | No auto-install observed | Fresh isolated app-server plugin/list recognizes the policy but reports installed=false/enabled=false; skills/list exposes no kit skill and creates no kit cache. Shipping policy remains AVAILABLE |
| Teammate marketplace ref honored | Deferred to M4 | Actual Claude trust and VS Code first-chat prompts required |

## Acceptance register

### M0 exit — PASS (2026-09-27)

Tag: [v0.0.0-m0](https://github.com/brklyn8900/buzz-agent-kit/tree/v0.0.0-m0), peeled commit `f8baecc7635771f256e05e386792944b455791a7`. Host versions: Claude Code 2.1.283; Codex CLI 0.157.1.

1. Preflight confirmed no kit shim/shared cache and no existing kit host installs on this Mac. Clone of the public tag, followed by explicit-path `install.sh` from `/tmp` with the M0 dev binary, exited 0. Both native plugin lists report enabled version `0.0.0-m0`.
2. Shell resolves `~/.local/bin/buzz-kit`; doctor exits 0 with `M0 development stub: launcher and argument dispatch work.`
3. Fresh `codex exec --ephemeral --sandbox read-only` session loaded the installed setup skill at the derived cache path and executed the diagnostic: shim resolved, same stub doctor text, exit 0, CODEX_THREAD_ID present, plugin_bin_on_path=no. Parent CODEX markers were removed before launching, so the runtime supplied them.
4. Fresh `claude -p /buzz-kit:setup` session loaded the skill from the installed plugin. Actual Bash tool outputs: `command -v buzz-kit` → shim path, `exit:0`; `buzz-kit doctor` → the same stub text, `exit:0`. The initialization event lists buzz-kit as loaded.
5. Isolated fresh Codex app-server `plugin/list` and `skills/list` settled native-catalog priority and no observed default auto-install, as recorded above.

The Claude test runner initially rejected extra diagnostic commands outside its narrow allowance. The successful final run used the skill's native commands, with allowed read-only output helpers. Its optional three-marker shell loop was still denied; no claim is made about that extra Claude environment check. Claude root-token substitution reports were inconsistent between model summaries, so no runtime contract relies on them. Required Codex marker/PATH/substitution checks are directly evidenced.

Scope: M0 validates the development launcher and host loading only. It is **not** production doctor, release-artifact verification, live-post acceptance, or the clean-user M4 acceptance.

Plan updates before M1: detect Codex using CODEX_THREAD_ID; preserve strict optional marketplace-source handling; consume the real tag-installed fixture fields; do not depend on default installation policy or skill-text root substitution. No §2 decision changes are needed.

### Approved first push and M0 tag — complete (2026-09-27)

- Ron explicitly approved publishing sanitized master `f8baecc7635771f256e05e386792944b455791a7` and development tag `v0.0.0-m0`.
- Both pushes succeeded. `git ls-remote --heads --tags origin` reports master and the peeled tag at that exact commit. The public commit has no parents; original private history is not reachable from it. No release was created.
- Real Mac preflight: no `~/.local/bin/buzz-kit`, no shared buzz-kit data cache, and no buzz-kit plugin registered in either host.
- Cloned the public GitHub tag with `--depth 1 --branch v0.0.0-m0`. Called checkout `install.sh` from `/tmp`, supplying the checkout's explicit M0 development binary. Exit 0; both hosts installed/enabled `0.0.0-m0`, and the shim's absolute-path doctor ran. The user's local-bin directory already existed on PATH; the stricter no-local-bin acceptance remains M4 item 7.
- Bare shell `command -v buzz-kit` resolves the shim; `buzz-kit doctor` reports `M0 development stub: launcher and argument dispatch work.` This is intentionally not production doctor evidence.
- Real tag-installed host JSON fixtures are refreshed with home paths anonymized.

### Tagged-checkout test correction — complete

- Red: running the full shell suite on the now-tagged public master failed `untagged checkout accepted without --dev`.
- Cause: the test mistakenly assumed its own checkout would never be tagged. Installer behavior was correct.
- Fix: create a dedicated temporary Git fixture, test refusal before tagging, then verify dry-run accepts its tag. The test now works in both tagged and untagged developer checkouts.
- Green: `sh tests/m0.sh` passes all groups. No installed runtime code changed and no published tag moved.

### Public repository creation — complete (2026-09-27)

- Ron approved the next repo-creation gate and requested continued progress toward completion.
- `gh repo create brklyn8900/buzz-agent-kit --public` succeeded. Read-back reports `isPrivate: false` and an empty default branch: the repository exists but contains no pushed code.
- Public-source scan found only GitHub/API, reserved example, and documented local-app hosts. No literal UUID, private-key PEM block, or GitHub classic token was found in tracked files. The original live hostname is absent from the current tree.
- A separate clean initial publication snapshot is prepared locally from the sanitized tracked tree. Original task history remains in the private working repository. First push and M0 tag publication still require approval.

### Source-address cleanup — complete (2026-09-27)

- Ron instructed that server addresses must not be published; generic examples suffice for the open-source kit.
- Replaced five live-hostname occurrences in AGENTS.md, background, and spec with `buzz.example.com`. These are examples, not deployment endpoints. Verified §2 remains byte-identical. Left the unrelated status-line edit out.
- This supersedes the earlier pending source-cleanup proposal. Original commits remain private in the existing local repository; publication must use a separate clean snapshot, never push the original history.
- No public repo, remote, tag or release was created. Repo creation/first push/tag approvals remain pending.

All nine M4 items remain **NOT RUN** as production acceptance. **M0 passed**; its development-stub proof does not replace production, live-server, fresh-user, or release-artifact evidence.

## M1.1 — dependency approval preparation

- Aligned the working master with the public sanitized history. Original local task history is retained under a private Git ref and in a verified private Git bundle; it is not an ancestor of public master. Future pushes must explicitly name master or an individually approved tag, never private refs.
- Prepared `docs/security/dependency-log.md` and `dependency-proposal-2026-09-27.json`: ten exact direct versions, 47 transitives, checksums, sources, owners, activity, feature choices, build-script exceptions and explicit unresolved review limitations.
- Resolver-only metadata review used existing Cargo; no project Cargo manifest added, no build scripts/macros executed, no dependency installed as a tool. Registry archive hashes matched for all 57 packages; OSV returned zero advisory matches and no pending pages.
- Stop at the user-requested dependency gate. Implementation of M1 starts after Ron approves this concrete batch. cargo-dist, audit/deny tools, Actions and Linux Buzz binaries remain separate future dependency proposals.

## M1.1 — CLI and configuration (complete, 2026-09-27)

- Ron approved the exact 10-direct/47-transitive Rust batch, reviewed compiler inputs and stated vetting limitations. Added only that graph, pinned Rust 1.89.0 already present on this Mac, and checked all 57 lock versions/checksums against the approved JSON.
- Red: `cargo test --locked --test config` reported `0 passed; 5 failed`, each at its unimplemented config operation. Green: five config tests plus the CLI flag test pass. `cargo build --locked` passes. `scripts/check-version` prints `Versions agree: 0.0.0-m0`; M0 shell suite passes.
- Implemented typed personal/project config, precedence, validated enums, credential-free relay normalization, and proven runtime marker selection. Invalid config errors do not echo input values. Conflicting runtime markers require an explicit identity.
- Ruling: `--config` / `BUZZ_KIT_CONFIG` selects the project config; personal config stays at the specified per-human location. The spec leaves the selected file type unspecified; this keeps credentials and runtime preferences separate from project data. Discovery walks ancestors only to the Git boundary.
- Added a Cargo workspace and developer-only version scripts using the Python 3 already used in M0. Version changes update the root package lock entry without resolving dependencies. The shipped plugin does not require Python. Production doctor remains explicitly unimplemented until M1.4; no false green status.

## M1.2 — guarded keystores (complete, 2026-09-27)

- Read the installed `/usr/bin/security help {add,find,delete}-generic-password` output. Real throwaway probe: `security -i` process exit 0; read exit 0; secret read-back equal; secret absent from interactive output. Duplicate creation exits 45; deleted-item lookup exits 44. Cleanup exits 0. Only unique synthetic throwaway items were used.
- Red: four initial keystore tests failed at the unimplemented operations. Additional failing regressions exposed a symlinked parent and a secret-service error mistaken for a missing item; another failure caught skipped cleanup after failed read-back. Fixed each, retaining the regressions.
- Green: `cargo test --locked` passes 13 tests (one real-Keychain test ignored by default); `cargo test --locked --test keystore real_keychain_stdin_roundtrip_and_cleanup -- --ignored --exact` passes 1/1, including read-back, duplicate preservation, listing, deletion and absence. `cargo build --locked` passes.
- File backend enforces owner/mode, rejects symlinks/hard links, and links a synced staging file into place without replacing existing bytes. Secrets use zeroizing buffers and have no Debug/Display/serialization implementation. Helper stdout/stderr stay captured and are zeroized; errors omit secret-bearing helper diagnostics.
- Ruling: keep a public-name-only identity inventory before a platform key write. This supports `list` and future exact-key scanning without dumping the user's Keychain. Interrupted writes can leave harmless stale names; list checks the specific account and filters absent entries. Legacy service reads are explicit and never enumerated.
- Linux behavior is tested with an isolated helper, not claimed as live Linux acceptance. Checked [GNOME's secret-tool source](https://github.com/GNOME/libsecret/blob/main/tool/secret-tool.c): store reads stdin; lookup returns 1 both for missing values and service errors, with diagnostics on stderr for errors. The adapter distinguishes these without printing diagnostics and never falls back to files. Native Linux acceptance remains M4.
- §14 security-stdin open item: confirmed on this Mac, no fallback crate required. No persistent assistant identity created or imported.

## M1.3 — assistant identities and copy-only migration (complete, 2026-09-27)

- Red: six library tests failed on the unimplemented identity/lifecycle operations; the CLI integration test failed with `unrecognized subcommand 'assistant'`. Green: `cargo test --locked` passes 20 tests (one real-Keychain test ignored by default); locked build passes.
- The secp256k1 secret-one vector derives the standard generator x-coordinate; npub uses Bech32 (not Bech32m) and decodes to the same public bytes. OS randomness generates valid nonzero scalars. Secret bytes and hex buffers are zeroized.
- Implemented assistant new/list/show/remove/import/cleanup-legacy. Integration proof uses a disposable, explicitly selected file keystore: valid public JSON, duplicate refusal, no key in stdout/stderr, and no noninteractive deletion without `--yes`.
- Fake migration proofs: copy and read-back; identical destination no-op; different destination refuses with public keys only; write failure leaves source intact; corrupt read-back removes only the newly created destination; cleanup requires identical secret bytes and confirmation. Backend put owns partial-write rollback, so an import never deletes an existing entry that won a concurrent creation race.
- Ruling: import defaults the destination name to the legacy account when no explicit `--as`/BUZZ_KIT_AS is supplied. Migration always targets Keychain, never silently exports to the optional file store. The desktop service denylist runs before helper invocation.
- No real legacy keys read, copied, or removed. Profile writes and new-command profile options remain in the guarded M2 profile task.

## M1.4 — ordered diagnostics and host adapters (complete, 2026-09-27)

- Inspected the installed Buzz CLI help: HTTP(S) relay URLs; JSON errors on stderr; exit codes 0–5, with 3 = auth. `--version` is unsupported; app metadata identifies Buzz 0.5.25.
- Executed the real CLI against a local synthetic HTTP `/query` responder with a throwaway in-memory key. `channels list`: JSON array, exit 0. `channels get --channel <synthetic UUID>`: one JSON object, exit 0. `channels search --query fixture-room --exact`: JSON array including `channel_type: forum`, exit 0. A synthetic HTTP 401 produces exit 3 and an `auth_error` JSON object. No live server contacted. Public fixtures retain shapes with the synthetic UUID replaced.
- Finding for M2: `channels get` omits channel type in this installed CLI. `channels search` exposes it. Resolve an exact-name search then match the configured channel ID before choosing forum/stream kinds; refuse unknown types. This fulfills the spec without a spec change.
- Red: three adapter tests failed at stub operations. Two doctor integration tests failed because doctor was unimplemented and emitted no report. Green: all adapter tests and both process-boundary doctor tests pass. Doctor emits the prescribed first nine checks, file/PATH warnings, current/pin and host status; auth exit 3 directs the human to the operator.
- Doctor integration uses a real system curl NIP-11 request to a local synthetic server, a disposable explicit file keystore and a fake Buzz process which asserts key-in-env only and removed owner auth tag. Required checks pass; optional PATH/cache/host warnings do not turn a usable shell installation into a failure. This is not live-server acceptance.
- Host adapters use the recorded Claude/Codex shapes and only listed/derived paths. Tests cover wrong sources, duplicate marketplaces, disabled plugins, missing fields/checksums, traversal and manifest version mismatch. No cache scan.

### M1 exit — PASS (2026-09-27)

`cargo test --locked`: 25 tests passed, 0 failed, 1 ignored (the separately run Keychain test). `cargo build --locked`, format and version checks pass. The manual/ignored real-Keychain test passed 1/1 with matching read-back and confirmed deletion. No real assistant identities or live channels were used. M2 starts next; all nine M4 acceptance items remain pending.

## M2.1 — exact-byte guards and splitting (complete, 2026-09-27)

- Red: all three initial guard/split tests failed at stub operations. Green: scanner corpus and split tests pass; full locked suite passes 28 tests, with the previously verified real Keychain test ignored by default.
- All §8 credential patterns and every inventoried kit key are blocked without echoing matched values. Stored hex keys are also caught in uppercase. A different bare 64-hex event ID remains allowed.
- The complete input is scanned before any part is returned. Tests place a credential across the proposed cut, exercise 65,536/65,537-byte boundaries, refuse invalid UTF-8, and reconstruct a 160 KB Unicode message exactly after removing numbered labels. Cuts prefer headings, blank lines, then newlines before 60,000 bytes; each labeled part remains within the server limit. A message that already fits is unchanged even with --split.

## M2.2 — read-only passthrough and guarded profiles (complete, 2026-09-27)

- Checked installed help for messages get/thread/search/send and users set-profile. Actual local synthetic-relay executions confirm profile and send success objects contain `accepted` and `event_id`; the send object also has `mention_pubkeys`. No live writes.
- Red: allowlist test failed at its stub, CLI integration failed on missing `as`, and profile regression caught data beginning with `--` being passed as a separate flag. Fixed the CLI profile-group name collision exposed by Clap's assertions; leading-hyphen profile values now use `--field=value`.
- Green: full locked suite passes 30 tests, 0 failed (one separately verified Keychain test ignored); locked build passes. Integration tests prove selected child key is in env and absent from argv, inherited owner auth is removed, safe reads succeed, writes refuse, profile fields are scanned before any child, and oversized profiles do not reach the helper.
- Implemented read/search/as, all 15 allowlisted read pairs, refusal of broadcast/identity/relay/output overrides and help, and guarded assistant profile. Buzz failure codes are preserved while helper diagnostics stay suppressed.
- New-command optional profile fields are guarded and emitted only as a shell-quoted follow-up profile command after membership; creation never contacts the server. No profile values are silently published before operator enrollment.

## M2.3 — guarded posting implementation; live gate pending (2026-09-27)

- Red: four posting tests failed at the send stub; the CLI file-input test failed on missing `post`. Green: full `cargo test --locked` passes **35 tests**, 0 failed, 1 ignored (the separately verified real-Keychain test). Locked build, format, version and whitespace checks pass.
- Process-boundary tests prove checked file bytes, including Unicode/trailing newlines, reach Buzz stdin unchanged via `--content -`; the file path, `--file`, and private key never reach argv. CLI broadcast is refused before any send.
- Verified default kinds: stream 9, forum root 45001, forum reply 45003. A split's first accepted event becomes the root; existing supplied roots remain the root for every part. Explicit kinds are supported. Missing/unknown channel type refuses unless explicitly supplied.
- Failed later parts stop without retry and report only confirmed event IDs plus uncertain delivery of the current part. The regression asserts exactly two send attempts after the second fails, with no third attempt. Full-payload secret scans happen before channel lookup or sends.
- **Not done:** the real-server claim/reply/read-back acceptance. Next user gate: obtain a private scratch channel UUID with the existing Codex assistant added, plus permission for a copy-only legacy assistant import into the kit Keychain service. No live post, persistent key write, legacy cleanup or deployment change has happened. M2 remains incomplete.
- Ron supplied approved spec commit `0e44110` while this task was running. It changes CI notifier default to a webhook workflow, preserving optional bot-key mode. The spec-only commit is already on master and has been preserved. After finishing this M2 task, read its change note and update M3's ci_bot.rs, both notifier templates and conditional .deb vetting tasks before M3 implementation.

## M2.3 — authorized live scratch test (PASS, 2026-09-27)

- Ron approved the private scratch channel and copy-only import for the Codex assistant. The channel UUID and server address were supplied only to running commands and an external private temporary recovery file; neither is recorded in this repository.
- `assistant import --legacy <legacy-service>/<assistant> --as <assistant> --json`: exit 0, source copied and destination read-back verified. A second import exited 0 with the same public identity and no changes, proving the legacy source remains readable. No cleanup-legacy ran; the deployment caller still depends on the legacy item.
- Authenticated `channels get` and exact-name search succeeded, with ID equality, private visibility and stream type confirmed before posting.
- `post --channel <approved-scratch> --json -` and `post --channel <approved-scratch> --thread <claim-id> --json -`: each accepted one synthetic message. `as <assistant> -- messages thread --channel <approved-scratch> --event <claim-id>` exited 0 and returned both signed-event objects.
- Read-back assertions all passed: both IDs present; both authors equal the imported public identity; both kind 9; exact claim/reply content; both channel tags equal the approved channel; reply references claim; claim has no parent. No broadcast, attachments, other channels, or server changes.
- Evidence anchors (SHA-256 of event IDs, not event IDs or channel identifiers): claim `2b16d5206ba5ef31692b05b16ec537391b2f55949c53e6c6e21b711ea18cecb5`; reply `6482a879e6b8dcac689d1a0824bc69491ae24bb148a2834860339b7341b40056`.
- Acceptance item 3's live author/thread check now has evidence. Overall M2 remains open for init, verified distribution, updates and Linux installation. Overall M4 remains open.

## Approved webhook spec change — plan reconciled

Read Ron-approved spec commit `0e44110` after the M2.3 live task. Updated M3.2 for default webhook workflow creation, nested-secret parsing/redaction, stdin-only GitHub secret delivery, flat JSON, fixed message template, two notifier templates, optional bot-key behavior and workflow visibility diagnostics. Updated acceptance item 5 to include webhook PR-opened/merged, wrong-secret refusal, and one bot-key PR-opened check. The .deb dependency gate remains necessary only for install-buzz-cli and optional bot-key mode; webhook mode downloads no Buzz binary. No additional spec edits or dependency additions.

## M2.4 — project init and teammate settings (complete, 2026-09-27)

- Red: five merge/write tests failed at stubs; the CLI test failed on missing init. Green: seven init tests pass; full locked suite passes 42 tests (one separately verified Keychain test ignored).
- Covers aliases, deliberately disabled plugins, same-ref no-op, differing-ref preservation/explicit update, wrong repository and ambiguous alias refusal, malformed shapes/JSON, key order, no added hooks/MCP/env/permissions, symlink refusal, trailing newlines and staging-failure preservation. Both files are preflighted and staged before replacement; a failed second rename restores the old project file.
- Actual authorized read-only live verification in an external temporary project: init resolved the scratch channel by name, checked visibility and wrote the current pinned teammate settings. Exit 0; ID/ref/enabled assertions passed; the private temporary project was removed. No channel UUID or server address entered the kit repo.
- --no-verify works without an assistant key; --no-claude-settings leaves even an invalid existing settings file untouched. Failed server verification leaves existing project bytes unchanged.
- Ruling: offline init requires a channel UUID or an existing configured channel, because an unseen name cannot safely yield its UUID without querying the server. No invented channel ID.

## M2.5 — verified bootstrap, cache and shim (complete, 2026-09-27)

- Red: release bootstrap tests failed at the M0-only development requirement. First-run regression tests then caught stdout contamination and a lock surviving exec. Both are fixed: implicit bootstrap reports on stderr and cleans its staging/lock before executing the binary.
- Green: `sh tests/launcher.sh` passes four-target selection, cache-hit/no-download, spaces/apostrophes, absolute invocation from another cwd, three-line shim, foreign shim preservation, checksum mismatch, gh attestation refusal, cached executable corruption, unsupported target, symlink/traversal tar entries and invalid archives. Existing M0 suite and sh syntax checks pass.
- Release archives are checked against the checkout's four-target manifest; if gh exists, attestation verification must pass. Cache records archive hash, target and binary hash; re-use checks binary bytes. Only a validated regular buzz-kit member is extracted to staging. Current switches atomically; a shim-write failure restores its previous target. Plain runs never compare/update offered versions.
- Checked installed `gh attestation verify --help` for artifact path and --repo semantics. No real release artifact was downloaded, no tag/release published. `release/checksums.txt` deliberately contains only a pre-release placeholder, so nondevelopment bootstrap refuses until the first reviewed checksum PR. Synthetic test hashes never become shipped trust anchors.
- Shellcheck is still unavailable and not installed; its reviewed-tool gate remains in M4. Tests use system sh/utilities. Cargo-dist's eventual actual archive layout remains a real RC validation requirement; the launcher accepts a single regular binary at archive root or one directory deep and rejects unsafe paths/links.

## M2.6 — explicit updates, rollback and pinning (complete, 2026-09-27)

- Six update tests cover semantic version selection from installed host candidates, no candidates, obsolete cache exclusion, already-current behavior, pin refusal before discovery, integrity-checked rollback, explicit unpin, failure preservation, two-recent pruning and --no-prune. Existing adapter fixtures cover provenance and manifest mismatches. Doctor reports newer offered binaries without updating implicitly.
- Test-first limitation: the initial update test run stopped at a compile error in digest formatting, rather than assertion failures at stubs. The later malformed-current regression did fail: a successful launcher pointing outside the cache bypassed restoration. Fixed it and verified restoration of the original current symlink.
- Green: `cargo test --locked` passes 48 tests, zero failed, one ignored (the separately verified real-Keychain test). `cargo build --locked`, format checks, launcher shell suite and whitespace checks pass. These are local fixtures, not the pending real two-release RC acceptance.
- No host cache scanning, new dependencies, real release downloads, or tag pushes. M2.7 still requires Linux artifact vetting and approval; M2 exit remains pending.

## M2.7 — Linux dependency vetting (approval pending, 2026-09-27)

- Statically inspected the official Desktop 0.5.25 x86_64 .deb and verified its GitHub-published SHA-256. Extracted CLI is a regular x86_64 ELF; no package maintainer scripts. Existing ar/tar suffice. No downloaded executable was run or installed. Full evidence and exact hashes are in docs/security/dependency-log.md.
- Findings: no ARM64 Linux artifact; glibc 2.38 required; no GitHub attestation (HTTP 404); upstream release is mutable and its build does not use --locked. Conservative source-lock OSV scan found two advisories: aarch64-only cmov correctness issue and instant unmaintained notice. Kit's own cmov is already the fixed 0.5.4.
- Plan updated with explicit architecture/libc checks and system extraction tools. Recommend this exact x86_64 artifact with the recorded limitations; await Ron's dependency approval before implementing its pin or installing it. Default webhook mode has no dependency on this artifact. No spec decisions changed.
- Current gates: M0 and M1 passed; M2.1–M2.6 passed, M2.7 pending dependency approval and implementation. M3/M4 and publication remain pending. Authorized live import/post proof is complete; legacy cleanup remains gated on the deployment handoff and re-verification.

## M2.7 — approved Linux CLI installer (complete, 2026-09-27)

- Ron approved the exact reviewed x86_64 .deb, checksums and documented limitations. No other dependency approval inferred.
- Red: supported-platform and install-success tests failed at stubs. Green: five ordinary installer/discovery tests pass. They exercise glibc/architecture refusal, archive and binary checksum refusal, extraction of only the CLI, intact-cache reuse, corrupt-cache refusal, symlink/foreign-current preservation and explicit discovery override precedence.
- Real official archive: ignored extraction test passed 1/1 against the approved downloaded bytes, into a disposable home, without executing the Linux binary. Exact extracted hash matched the vetting entry. macOS `buzz-kit install-buzz-cli --json` exited 0 and identified the existing application CLI.
- `cargo test --locked`: 53 passed, zero failed, two ignored (real Keychain and approved-archive tests each separately passed). Locked build and launcher suite pass. Installer uses existing curl/ar/tar, performs no package installation or root operation, and refuses unsupported Linux targets/libc. Actual Linux execution remains M4 acceptance item 4, not claimed here.

### M2 exit — PASS (2026-09-27)

All local guards, allowlists, payload, settings merge, bootstrap, update and installer tests pass. M2.3 records the real scratch-thread claim/reply author and parent verification. M0/M1/M2 are complete; M3 begins with the approved webhook default. Release acceptance and publication remain open.

## M3.1 — shared setup/room skills and generic documentation (complete, 2026-09-27)

- Packaging check first failed on M0's placeholder root-token text. Both skills now have valid bounded frontmatter, no root-token substitution, and preserved no-hook/no-MCP manifests. `python3 tests/skills.py` and version checks pass. This is packaging validation; fresh-host behavior remains M3.3.
- Setup follows doctor, human invite, per-runtime identity, operator enrollment, profile, personal config, channel init, agent snippet and optional webhook-default CI. Room defines ask/status/off, untrusted-channel authority, one claim/thread, five message templates, text-only and size/secret restrictions.
- Added generic collaboration/operator guides, manual tagged installation and skills-only fallback limitations, privacy guidance and official Apache-2.0 license text. Operator docs explicitly leave webhook rotation/display verification pending rather than inventing results. Read the version-matched upstream iOS gateway guide; linked its constrained app-profile prerequisites instead of promising universal push support.
- Ruling: prose validation is proportionate packaging/content review per AGENTS.md; no artificial agent-pressure ceremony for these documentation edits. Real Claude/Codex walkthroughs remain mandatory. No actual relay address, channel identifier, identity key or webhook secret is in these artifacts.

## M3.2 — CI initialization implementation (local checks PASS; live acceptance pending, 2026-09-27)

- Checked real workflows create/update/get/list and GitHub secret/variable help. Ran the installed Buzz CLI against a local synthetic relay: create exited 0 with accepted/event_id/message/workflow_id; message held nested response JSON with workflow_id/webhook_secret. Absent get returned null; list returned []. No live workflow created.
- Red: parser, rendering and three CLI integration tests failed at stubs. Later recovery regression failed when a malformed secret envelope discarded a valid public workflow ID; fixed by preserving the validated ID before secret parsing.
- Green: eight CI tests pass, covering default webhook, explicit bot-key, stdin-only export, no secret echo even on helper error, existing-file preservation, duplicate prevention after partial failure, known-ID recovery, symlink refusal and workflow visibility. Full locked suite: **61 passed**, zero failed, two previously separately verified ignored tests. Locked build, format/version/whitespace and skill packaging checks pass.
- `python3 tests/notifications.py`: shipped webhook shell with real jq and capturing curl passes PR opened/reopened/ready/merged/closed, issue opened/closed, release published and failed-CI cases; successful CI/unknown events/missing secrets skip. Hostile shell-looking titles remain exact JSON data, with no command execution. Templates use no third-party Actions and empty default permissions; optional bot-key template uses the approved outer and inner artifact hashes.
- Secrets stay in zeroizing buffers and go to GitHub stdin. Only public setup metadata is stored in the personal directory, not the project. A pending reservation prevents automatic duplicate remote creation after uncertain failure. Failed setup preserves recovery evidence; it does not silently revoke a workflow or delete a key.
- Ruling: --ci-workflow selects the existing CI workflow name (default CI); --name selects a safe notifier/bot name. CI metadata is local, because server/workflow identifiers do not belong in this public kit. No spec decision changed.
- Pending: authorized live disposable workflow update/rotation and relay-author display checks; private GitHub event delivery and bot membership acceptance. No real GitHub secret/variable writes, CI identity creation or notifier workflow writes occurred during these local tests. M3 exit is not passed. Continue the independent installer implementation before the concrete live-test gate.

## M3.3 — production installer implementation (local checks PASS; fresh setup pending, 2026-09-27)

- Red: installer fixture failed when doctor exit 1 suppressed installation success. Installer now distinguishes installed binary/shim/hosts from first-time configuration needs; unexpected doctor execution errors still fail. Release installation refuses development binary overrides, dirty tracked files and tag/manifest version disagreement.
- `python3 tests/installer.py` passes both/single/no-host, absolute doctor invocation without local-bin on PATH, setup-warning success, broken-doctor failure, tag agreement, mutation-free dry-run, plugin/marketplace/skill collisions, preservation backups and stop-on-host-failure cases.
- Actual isolated Codex marketplace probe confirmed that marketplace registration writes config.toml. Installer now backs up that file as well as Claude's known_marketplaces.json, preserving unrelated content; it refuses symlinked configuration.
- `sh tests/host-smoke.sh` passed real Claude and Codex local-source installs in a disposable home, shim resolution and repeat-install refusal. This uses the explicit M0 development stub solely as the host installer probe; it is not a fresh production setup or release-tag acceptance.
- Final local checks: 61 Rust tests pass, locked build, installer/event/skill tests, M0 shell regressions, shell syntax, version agreement and whitespace checks pass. Tracked and pending public files were checked against the external private live recovery identifiers: none found.
- Prepared docs/live-acceptance-plan.md and a generic private test-repository fixture. Remaining M3 gates are real CI delivery/rotation and fresh Claude/Codex setup with enrolled identities. Request the copy-only Claude identity import, temporary CI bot key scope and private GitHub test writes as a concrete batch; existing R2D2 and scratch-channel approvals persist. No new tags or releases requested yet.

## Continuation — session access blocker (2026-09-28)

- Ron directed continuation after clarification that the proposed identities support acceptance testing and the separate migration; the kit remains configurable for any compatible Buzz server. Existing scratch-channel and identity approvals remain in force. Tag/release, dependency and legacy-cleanup gates remain separate.
- Rechecked master at `08dca18`: working tree was clean before this note. M0/M1/M2 exit gates passed; M3 implementation is committed but its live/fresh-host exit remains pending; M4 has not started. Reported **60% by milestone gates (3 of 5)**, explicitly not a time estimate or a claim that M3 acceptance passed.
- Current session permissions changed to workspace-write with restricted networking, read-only Git metadata, and no normal home-directory writes. `gh api user --jq .login` failed with `error connecting to api.github.com`. The prepared live batch cannot execute end to end under this access profile. No new identity, remote repository, GitHub setting, workflow or post was created in this continuation.
- Resume with the prepared live acceptance plan when execution access is restored. This note remains uncommitted because this session disallows Git metadata writes; do not route around that restriction or treat it as a new project approval gate.

## M3 live acceptance — webhook evidence (2026-09-28)

- The earlier session access blocker is resolved: authenticated GitHub access and authorized filesystem/Git writes work. Ron directed continuation of the prepared concrete batch. No new dependency, tag, release or legacy-cleanup approval is inferred.
- C3PO's authorized legacy import exited 0 twice with identical public identity; destination read-back passed and legacy source remains intact. The sibling deployment checkout was not edited.
- Created the authorized **private** acceptance repository on `master`, seeded only from the synthetic fixture and actual initializer output. Default `ci-bot init` exited 0, created the scratch workflow and exported its secret directly to GitHub stdin. No operational relay/channel values or secrets are recorded here.
- Actual PR 1 opened and merged. Notification runs **36443095085** (opened) and **36443639082** (merged) both concluded `success`. Read-back content exactly matched the expected fixed template. Both event authors match the relay's NIP-11 `self` key; the separate `pubkey` field is null. SHA-256 of event IDs: opened `6d200885a201ede7827f9a64fe0b5fea006747506b6601fc9d60dedf7b8b0640`; merged `4372ea397c7838413667e2ec74eb3067c7c37add6bd4c8788fded866df6e7d35`.
- Added an explicitly ignored live integration probe using existing dependencies. It requires write authorization plus an external private configuration, reserves private recovery metadata before mutation, keeps secrets in zeroizing buffers, sends the header on curl stdin, and deletes only its own disposable workflow. A compile-only first attempt caught digest formatting incompatible with the pinned sha2 version; corrected before any live execution.
- Actual `cargo test --locked webhook_lifecycle_live -- --ignored --nocapture`: **1 passed, 0 failed**. Wrong secret **401**, valid secret **202**, update accepted, old secret after update **202**, no replacement secret returned, after deletion **404**. Exactly two expected messages were read back; neither wrong-secret nor post-deletion request posted. Event-ID hashes: `05445afa313b9bacb21734f039c48241b22c82be63570b43a2218aeb3770be75`, `15b30763043b086c9552a4b6bffaa624581293ad2950784ba083fc56e29f4ba9`.
- Optional bot-key initializer exited 0 in a separate private checkout, exported its generated key over stdin, and retained it for the approved test. Its generated notifier has not replaced the webhook workflow. Await operator enrollment before the bot event check. An initial unexported, unenrolled temporary key was removed before using the real initializer; only the final public identity was given for enrollment.
- Computer-use access to Buzz Desktop was denied by the tool. Asked Ron to enable it or report the exact displayed author label; did not substitute another UI automation route. Cryptographic author evidence above passes; desktop presentation remains unverified.
- §14 finding: ordinary workflow update does not rotate the secret on the tested deployment. Operators documentation now records observed behavior; no new rotate command or spec change. M3 exit remains pending fresh-host walkthroughs and remaining live checks; M4 has not started.

## M3.3 — fresh setup walkthroughs (development proof PASS, 2026-09-28)

- Real installer in an empty disposable HOME exited 0, bootstrapped the actual current Rust binary (not the M0 stub), and installed current skills in both native hosts. Initial doctor correctly reported missing setup and PATH. Installed Codex setup-skill bytes match the current source SHA-256.
- Fully replacing HOME hid the existing macOS Keychain identities: `assistant show` exited 0 under the real account and 1 under the disposable HOME. It also hid native Claude sign-in. The first Codex walkthrough honestly failed its assistant-key check; the superseded Claude attempt was stopped. No credentials were copied and no new identity was created to mask this test-environment limitation.
- Corrected walkthrough isolation: real account HOME for existing Keychain/authentication; disposable projects; isolated Codex plugin configuration; Claude project-only settings, no MCP servers, and the explicit current inline plugin. Bootstrapped the actual development binary into the managed user cache/shim, preserving previous shim/current metadata privately. Created personal runtime mappings for the two already enrolled identities, Keychain and autopost ask; operational values remain outside this public repository. No shell startup file edits.
- Fresh Codex session exited 0. Its trace contains the installed current setup-skill path and body. Verified init succeeded and the shipped AGENTS snippet was appended. Fresh Claude 2.1.283 session exited 0 and explicitly invoked `Skill` with `buzz-kit:setup`; initialization identified the inline plugin from this checkout. Its verified init returned **"Project configured and channel verified."** Both sessions used existing profiles/keys and sent no posts.
- Independent final shim invocations for each identity: `doctor --json` exited 0, `ok: true`; platform, CLI, personal/project config, server, assistant-key, membership, channel, keystore, current, pin and PATH all **pass**. Both temporary project AGENTS files contain the exact shipped snippet. Codex CLI is 0.157.1.
- Warnings are preserved: isolated local marketplace sources are not trusted published sources; existing old M0 cache entries cannot resolve as current release candidates; current development binary cannot compare a published release. These warnings do not invalidate live identity/channel setup, but production-tag host resolution and update acceptance remain M4 requirements. No metadata was edited to conceal them.
- Private session-trace SHA-256: Codex `cc34736fcd042aa513021b6ed97c0a6061466d529388458682ec565e6d48744a`; Claude `913302134e74fcd5512b8fc1de9cc4d2d9ac5f6a50cffd33bbd09a723da1b119`. Full traces/configuration remain in the external private test directory.
- Plan clarification: removed its circular M3 dependency on not-yet-created production release tags. Spec §13 requires fresh setup here; M0 already proved tagged plugin discovery, and M4 produces/accepts binary release tags. The spec itself is unchanged.
- Local validation after the live-probe addition: **61 regular Rust tests passed**, 3 explicitly ignored (each authorized integration probe has separate evidence), format and whitespace checks pass. Public tracked/pending files contain neither the actual relay nor the scratch identifier.
- **M3 closeout still pending:** the temporary bot is not yet a scratch-channel member (actual members read-back), so optional bot delivery has not run; desktop author-label observation is also pending because computer-use access was denied. Fresh setup itself now passes. M4 and publication remain unstarted, with existing release/dependency/legacy-cleanup gates intact.
