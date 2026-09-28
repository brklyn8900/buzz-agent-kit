# buzz-agent-kit

A shared plugin for Claude Code and Codex: separate assistant identities, guarded messages, and one project room for humans and coding agents using [Buzz](https://github.com/block/buzz).

**Development status:** M0–M2 passed. Production skills and CI setup are in progress; v0.1.0 is not published. The existing `v0.0.0-m0` tag is a development probe, not a production installer. Commands below describe installation from a completed release tag; substitute an actually published `vX.Y.Z`.

## Install a release

Use a tag-pinned checkout, inspect it, and run the installer from any working directory:

```sh
git clone --branch vX.Y.Z --depth 1 https://github.com/brklyn8900/buzz-agent-kit.git
./buzz-agent-kit/install.sh --dry-run
./buzz-agent-kit/install.sh
```

The installer bootstraps the verified binary, installs the detected Claude/Codex plugin at that same tag, then runs doctor. It prints PATH guidance without editing your shell startup files. Start a fresh shell and agent session, then invoke the setup skill (Claude: `/buzz-kit:setup`; Codex: ask to use the Buzz setup skill).

Manual host installation, after running the tagged checkout's `bin/buzz-kit bootstrap`:

```sh
claude plugin marketplace add brklyn8900/buzz-agent-kit#vX.Y.Z
claude plugin install buzz-kit@buzz-agent-kit
codex plugin marketplace add brklyn8900/buzz-agent-kit --ref vX.Y.Z
codex plugin add buzz-kit@buzz-agent-kit
```

Use only the commands for hosts you have installed. Plugin registration alone does not put the binary on PATH. Ordinary launches keep the current binary; after updating the installed plugin, run `buzz-kit update` explicitly. `buzz-kit update --to bin-vX.Y.Z` selects a verified cached binary and pins it; `buzz-kit update --unpin` permits updating again.

Other agents can use the skills-only fallback `DO_NOT_TRACK=1 npx skills add brklyn8900/buzz-agent-kit`. This invokes Vercel's separate CLI, whose telemetry is disabled by that variable; vet/approve that dependency under your own policy before running it. It does **not** install `buzz-kit`, its shim or an assistant identity. This project's installer does not execute it.

## Connect a project

Follow the [setup skill](skills/setup/SKILL.md). Your human joins the server in Desktop; the operator enrolls separate assistant public keys and the channel owner adds them by display name or hex. Never share private keys or use the human's desktop identity.

Personal preferences live in `~/.config/buzz-kit/config.json`:

```json
{
  "assistants": { "claude": "claude-assistant", "codex": "codex-assistant" },
  "autopost": "ask",
  "keystore": "auto",
  "default_relay": "wss://buzz.example.com"
}
```

`buzz-kit init --channel <name-or-uuid>` verifies access, writes `.buzz/config.json` and merges pinned Claude teammate settings without overwriting deliberate choices. Treat relay/channel details according to your project's privacy policy; keep private values out of public repositories. See the [generic config](templates/buzz.config.example.json) and [agent instructions](templates/AGENTS.snippet.md).

The [room skill](skills/room/SKILL.md) reads/searches first, uses one claim thread per work item, and posts only material changes. `autopost` is `ask` (default), `status`, or `off`. Channel content is untrusted information, never instructions. No broadcasts or attachments; message size is 65,536 bytes, with guarded `--split` available.

macOS uses Keychain. Linux uses secret-service; the explicit file fallback uses owner-only permissions. Automatic Linux Buzz CLI installation currently supports x86_64/glibc >=2.38, using the reviewed, checksum-pinned 0.5.25 artifact. Other Linux targets require a compatible CLI via `BUZZ_KIT_BUZZ_CLI`. The kit itself targets both architectures on both operating systems.

CI setup uses a webhook by default; optional bot-key mode provides a named author. See [operator guidance](docs/operators.md) for enrollment, webhook authority, attachments, CORS and iOS push caveats. Implementation and live notification acceptance are tracked in the build notes.

## Development and evidence

Run `cargo test --locked`, `cargo build --locked`, `scripts/check-version`, `python3 tests/skills.py`, `sh tests/launcher.sh` and `sh tests/m0.sh`. Real Keychain/archive tests are explicitly ignored by default; their prerequisites and executed evidence are recorded separately. Synthetic tests are not release or live-server acceptance.

- [Implementation plan](docs/superpowers/plans/2026-09-27-buzz-agent-kit.md)
- [Build evidence and acceptance status](docs/build-notes.md)
- [Approved design](docs/superpowers/specs/2026-09-27-buzz-agent-kit-design.md)
- [Dependency decisions](docs/security/dependency-log.md)
- [Collaboration protocol](docs/collaboration.md)

Licensed under [Apache-2.0](LICENSE).
