# Host CLI fixtures

Captured on 2026-09-27 using Claude Code 2.1.283 and Codex CLI 0.157.1:

- `claude plugin list --json`
- `claude plugin marketplace list --json`
- `codex plugin list --json`
- `codex plugin marketplace list --json`

Selected real entries preserve output field types and nesting. Names, IDs, paths, repository URLs, versions, and timestamps are replaced with public examples. These fixtures describe host output shape; they do **not** claim buzz-kit was installed. Empty `available` matches the observed list. The Codex marketplace fixture includes an observed missing-source entry so parsers cannot assume every entry supplies provenance.
