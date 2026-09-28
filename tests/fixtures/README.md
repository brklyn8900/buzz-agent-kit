# Host CLI fixtures

Captured on 2026-09-27 using Claude Code 2.1.283 and Codex CLI 0.157.1:

- `claude plugin list --json`
- `claude plugin marketplace list --json`
- `codex plugin list --json`
- `codex plugin marketplace list --json`

Refreshed after the actual `v0.0.0-m0` tag installation into both hosts. Entries preserve the installed kit's output fields, versions and public GitHub source; home paths are anonymized. Claude's marketplace includes `ref`; Codex's installed plugin includes `marketplaceSource` but no `installPath`. Empty `available` matches the observed list. The Codex marketplace fixture also retains the earlier observed missing-source shape under a generic name so parsers cannot assume every entry supplies provenance.
