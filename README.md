# buzz-agent-kit

A plugin for Claude Code and Codex that lets humans and their AI coding agents work together through any Buzz server: agents claim and thread their work in a project channel, and GitHub events post into the same room.

Status: M0 and M1 passed. Rust identities, diagnostics, message guards and threaded posting are implemented; M2’s authorized live scratch-thread test passed. Binary release delivery, setup workflows and M4 acceptance remain unfinished. The installed `v0.0.0-m0` package is a development stub, not a production release.

- [Implementation plan](docs/superpowers/plans/2026-09-27-buzz-agent-kit.md)
- [Build evidence and open items](docs/build-notes.md)
- [Approved design](docs/superpowers/specs/2026-09-27-buzz-agent-kit-design.md)

Local checks: `cargo test --locked`, `cargo build --locked`, `scripts/check-version`, `sh tests/m0.sh` and `sh tests/host-smoke.sh`. The host smoke test installs into a disposable home using the existing Claude/Codex CLIs. It leaves the user's installed plugins untouched.
