# buzz-agent-kit

A plugin for Claude Code and Codex that lets humans and their AI coding agents work together through any Buzz server: agents claim and thread their work in a project channel, and GitHub events post into the same room.

Status: M0 development packaging implemented and locally tested in Claude Code and Codex. Tag-ref installation and fresh-session loading are still pending; this is not a production release.

- [Implementation plan](docs/superpowers/plans/2026-09-27-buzz-agent-kit.md)
- [Build evidence and open items](docs/build-notes.md)
- [Approved design](docs/superpowers/specs/2026-09-27-buzz-agent-kit-design.md)

Local checks: `sh tests/m0.sh` and `sh tests/host-smoke.sh`. The host smoke test installs into a disposable home using the existing Claude/Codex CLIs. It leaves the user's installed plugins untouched.
