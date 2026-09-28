# Dependency decisions

## Policy (2026-09-27)

Apply the local supply-chain-hardening playbook and spec §11 before adding any crate, Action, or tool. Obtain Ron's explicit approval for each concrete version/revision before adding it. No dependencies have been added or approved during implementation.

For each proposal record: purpose and why a small local implementation is insufficient; official registry/source and exact version/hash; downloads; maintainer count and ownership-transfer evidence over the last 90 days; release/source activity; advisory and behavioral scan; build/install scripts; transitive count and noteworthy behavior; license; reviewer; approval and residual uncertainties. An unavailable signal stays unknown, never becomes a pass.

## M0 baseline

Uses already installed host CLIs and system utilities only; no third-party package download/install. Local shell tests use the existing shell/utilities. Python 3 may be used interactively to inspect JSON, without introducing it as a shipped runtime dependency.

`cargo-dist` and `shellcheck` are absent. Their installation is deferred until vetted and approved. Planned §11 crates, cargo-audit, cargo-deny, Actions and the Linux Buzz artifact are not yet vetted or approved.
