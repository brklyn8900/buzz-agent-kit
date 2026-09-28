# Prepared live acceptance scope

Status: authorized and in progress. Default webhook PR-opened/merged delivery, disposable workflow lifecycle, and fresh Claude/Codex development setup checks passed on 2026-09-28. Optional bot membership/delivery and desktop presentation remain pending; see build notes. Release-tag installation remains M4.

## Isolated GitHub destination

Authorized repository: **brklyn8900/buzz-agent-kit-acceptance**, **private**, default branch `master`. Seed it only with `tests/fixtures/acceptance-repo/` and the initializer-generated notifier. The first push and later synthetic branches/PRs exercise notification events; no release tags, deployment, external Actions or production code.

Use the already approved private scratch channel, configured only in an external temporary checkout and private repository variables. Do not copy its name/UUID or relay address into this public kit or build notes. Secrets go straight from process memory to GitHub via stdin. Keep the acceptance repo private; public evidence records outcomes and hashes, not operational values.

1. Run default webhook `ci-bot init --repo brklyn8900/buzz-agent-kit-acceptance --ci-workflow CI` as the existing enrolled Codex assistant.
2. Review and push the generated workflow; open and merge a synthetic PR. Read back channel events and verify fixed content and relay authorship. A wrong-secret request must fail without a new message.
3. In a separate disposable scratch workflow, hold its one-time secret in memory, test create/update behavior and old/new secret validity, then delete it and confirm 404. Preserve only its public workflow ID in private recovery state if interrupted. Never print or persist the secret. Document observed rotation and desktop author presentation.
4. Exercise optional bot-key mode with a separately approved temporary identity, after the operator enrolls it and adds it to the scratch channel with role bot. Verify one synthetic PR-opened event; remove only that test identity/settings after agreed cleanup. No legacy key cleanup.

## Identities and host walkthroughs

- Codex: existing approved R2D2 kit copy; legacy remains untouched.
- Claude: authorized copy-only `assistant import --legacy koinosbuzz-assistant/c3po --as c3po` passed, including repeated read-back verification. The legacy source remains untouched. This does not edit deployment callers or authorize cleanup-legacy.
- Optional CI bot: authorized creation of `buzz-kit-ci-test` in the kit Keychain service and stdin-only export to the private test repo passed. Operator enrollment is pending. Deletion of this temporary local test key after its test is authorized. Show only the public hex/npub for operator enrollment. Do not reuse a human or runtime assistant key as the CI bot.
- Use isolated host configuration and a temporary project for fresh Claude/Codex setup walkthroughs. Actual testing found that replacing HOME hides the enrolled macOS Keychain entries (and native Claude sign-in). Keep the real account HOME for Keychain/authentication, isolate Codex's plugin directory and Claude's setting sources/inline plugin, and use private temporary projects. The current implementation can be exercised as explicit local development; completed release-tag installation remains the M4 RC/release gate. Never present a development walkthrough as published-release proof.

## Gates still outside this authorization

New dependencies/Actions/release tools need their own vetted proposal. Every new binary/plugin tag and release remains approval-gated. No edits to the sibling deployment repository, no production server changes, no broadcasts, no unrelated-channel posts, and no cleanup of either legacy assistant item.
