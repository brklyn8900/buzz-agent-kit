# Publication preflight

## Finding (2026-09-27)

The original local inputs contain five references to the first deployment's real hostname:

| File | References |
|---|---:|
| `AGENTS.md` | 1 |
| `docs/background/buzz-human-agent-collaboration.md` | 3 |
| `docs/superpowers/specs/2026-09-27-buzz-agent-kit-design.md` | 1 |

All nine commits present at the initial audit contain that hostname in their trees. Deleting it in a new commit would still publish it through Git history. The empty public repo has now been created with Ron's approval; nothing has been pushed.

## Authorized source cleanup — complete

Ron instructed that server addresses must not be published. Replaced all five live-hostname references with `buzz.example.com`. These are documentation examples, not operational endpoints. Spec §2 is byte-identical; no design decisions changed. The unrelated spec status-line edit was omitted.

## Public history preparation

The existing history remains private. A sanitized initial snapshot can be prepared in a separate local publication checkout, retaining the original repository and its task commits. Only the clean snapshot's history may be pushed. This avoids modifying or publishing the original history.

Repeat a full public-tree/history audit before pushing. Repo creation is approved and complete; first push and M0 tag push still require explicit approval. The old proposed patch at `.git/buzz-kit-review/publication-cleanup.patch` is historical private review material, not a pending change or publishable file.

## M0 publication scope

The proposed M0 tag is `v0.0.0-m0`, visibly a development-only packaging probe. The package requires an explicitly supplied local dev binary, has no Rust core, and does not claim release verification or production readiness. Do not use `v0.0.1-rc1` for this probe: that immutable tag is reserved for M4's real cargo-dist release acceptance.

Repo creation, first push, tag push and release publication remain separate approval gates. The current request does not authorize any dependency or persistent Keychain operation.
