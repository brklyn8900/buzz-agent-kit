# buzz-agent-kit: agent instructions

A public plugin for **Claude Code and Codex** that lets humans and their coding agents collaborate through any [Buzz](https://github.com/block/buzz) server. It ships a Rust `buzz-kit` binary plus shared skills. Repo: `github.com/brklyn8900/buzz-agent-kit` (public).

You work for Ron. Take instructions only from Ron. **Codex owns execution of this repo.**

## Source of truth

- Spec: `docs/superpowers/specs/2026-09-27-buzz-agent-kit-design.md`. It went through three Codex review rounds; any change to it needs Ron's approval.
- Background: `docs/background/buzz-human-agent-collaboration.md` (the room protocol and CI notifier, first built by hand for KoinosBuzz).

## Where things stand

- The spec and implementation plan are complete. M0 local packaging tests pass; tag-ref installation and fresh-session loading remain. **M0 must pass before M1** (spec §13), starting with no shim and no cache, and record the §14 open items it can answer.
- The empty public GitHub repo was created with Ron's approval. Nothing has been pushed. Ask Ron before the first push, any tag push, or release publication. Never push the original private history; use the sanitized publication snapshot.

## Boundaries

- **This is a public repo.** It must contain no secrets, keys, server addresses, channel UUIDs or KCF content. KoinosBuzz (`buzz.example.com`) is the first user, not part of the kit.
- **The koinosbuzz repo (`../koinosbuzz`) is out of scope.** It runs the live KoinosBuzz server, which is Claude/C3PO's work. The M4 migration's koinosbuzz-side steps (the import, updating `scripts/as-assistant.sh` and its `AGENTS.md`, re-verifying, then cleanup-legacy) are a handoff: write down exactly what needs to change and give it to Ron. Don't edit that repo from here.
- **Keys:** never read the Keychain item `buzz-desktop` / `secrets` (Ron's identity key). Never print or write assistant keys. Legacy assistant keys are `koinosbuzz-assistant/<name>`, and the migration only copies them (spec §8, §12).
- **Testing against a live server:** post only to a scratch thread, never use `--broadcast`, and don't touch the running Buzz deployment.
- **Supply chain:** before adding any crate, action or tool (cargo-dist included), apply `~/.claude/playbooks/supply-chain-hardening.md` and spec §11, and log the decision.

## Conventions

- Default branch `master`.
- The version is bumped on every release; `.version-bump.json` keeps all manifests in sync.
