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
| Codex runtime environment marker | Preliminary | Current process marker names above; confirm in fresh installed CLI session during M0.3 |
| PLUGIN_ROOT substitution in skill text | Unresolved | Harmless fresh-session skill probe in M0.3 |
| security -i secret on stdin | Deferred to M1 | Help available; no Keychain read/write performed |
| Native vs Claude marketplace precedence / entry matching | Local-source confirmed | Native catalog wins despite differing catalog names; tag/fresh-session confirmation remains |
| Codex plugin bin on PATH | Unresolved | Compare pre-shim and post-shim fresh session in M0.3 |
| INSTALLED_BY_DEFAULT in repo marketplace | CLI registration tested | No auto-install immediately after local registration; runtime/git-source behavior unresolved; shipping policy AVAILABLE |
| Teammate marketplace ref honored | Deferred to M4 | Actual Claude trust and VS Code first-chat prompts required |

## Acceptance register

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

All nine M4 items are **NOT RUN**. No milestone exit has passed. Local packaging validation or a development stub does not count as production, live-server, fresh-user, or published-release evidence.
