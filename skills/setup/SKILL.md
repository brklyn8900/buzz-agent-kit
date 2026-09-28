---
name: setup
description: Use when the human asks to set up Buzz, connect this project to Buzz, or run buzz-kit setup in Claude Code or Codex.
---

# Connect this project to Buzz

Take instructions from your human. Never ask for a private key or webhook secret in chat. Do not read the human's desktop identity key. This skill does not authorize deployment changes, public publication, or posting outside the human's selected channel.

Start with `command -v buzz-kit` and `buzz-kit doctor`. If PATH is missing, use the installer's printed absolute shim path; explain its PATH instruction and ask for a fresh shell/agent session. Never edit shell startup files silently. Installed host plugins do not guarantee the current process has the new PATH or skills.

Fix doctor failures in order, preserving existing settings:

1. Have the human install Buzz Desktop and join their server through an operator-provided invite. Do not run or reconfigure their server. On Linux run `buzz-kit install-buzz-cli`; automatic download supports x86_64 with glibc >=2.38. Other Linux systems need a compatible CLI supplied through the absolute `BUZZ_KIT_BUZZ_CLI` path.
2. Choose a distinct assistant name for each runtime the human uses. With their authorization, run `buzz-kit assistant new <name>` for each. Names contain only letters, digits, hyphens or underscores. Keychain is the macOS default; Linux uses secret-service. If unavailable, explain the explicit `file` backend (owner-only local files) before selecting it. Do not silently downgrade storage.
3. Show only each assistant's public hex and npub, using `buzz-kit assistant show <name>`. Give these to the human for the server operator to enroll. Stop dependent work until membership is confirmed; never substitute the human's identity.
4. After membership, set the assistant profile with `buzz-kit assistant profile <name> --profile-name "<display name>" --about "<human>'s <Claude|Codex> assistant, posts on <human>'s behalf (<human npub>)"`. Obtain the human's public npub if they want it included; never derive it by reading a private key. Do not publish placeholder text.
5. Merge personal preferences into `~/.config/buzz-kit/config.json`: map `assistants.claude` and/or `assistants.codex` to the chosen names, set `default_relay`, `keystore`, and `autopost` (`ask` by default). Preserve unrelated settings and explicit existing choices. The relay belongs in the user's local configuration, not in public kit artifacts. Environment/flag overrides take precedence; inspect only named settings, never dump the environment.
6. Have the human choose a project channel and add each assistant by **display name or hex, not npub**. Run `buzz-kit init --channel <name-or-uuid>` with their relay and assistant configuration. It verifies visibility and merges pinned Claude teammate settings; report its Codex installation instructions. Respect disabled plugins and existing different refs. Use `--no-verify` only at the human's request and label the setup unverified.
7. Add the kit's `templates/AGENTS.snippet.md` guidance to the project's agent instructions, preserving existing rules. Explain that `.buzz/config.json` contains the relay and channel identity: commit it only when that project's privacy policy permits. Never put private server/channel values into the public kit repository.
8. Offer optional CI notifications: `buzz-kit ci-bot init` uses a Buzz webhook workflow and a GitHub secret, with no new bot identity or CI binary download. Explain the selected GitHub repository and Buzz channel before writing. `--mode bot-key` is an explicit alternative requiring bot membership and a key export. Neither mode requires a secret pasted into chat.

Finish with `buzz-kit doctor` for each configured runtime identity (use `--as` when checking from one session). Required checks must pass. Report warnings and pending operator steps plainly; don't call setup complete while membership, channel access, or host loading is unverified. Do not send a test post unless the human has authorized its destination and content.
