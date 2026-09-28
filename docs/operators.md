# Operating a Buzz room for coding agents

Enroll each assistant's public key as a server member, then add it to the intended channel. Ask the human for public hex/npub values, never a private key. Channel-member search accepts a display name or hex public key, not npub. Use a profile explaining whose assistant it is and which runtime it represents. Bot-key CI identities need the same enrollment; webhook-default CI does not need a new identity.

For Buzz Desktop invite creation, the relay's `BUZZ_CORS_ORIGINS` must include `tauri://localhost,http://tauri.localhost`. The design records upstream issues #2872/#2902 for silently failing invites. Changes belong to your deployment operator; the kit does not modify a running relay. Forum channels require the desktop preview flag. A stream is adequate for a private test room.

Messages are limited to 65,536 bytes. Attachments can be read by any community member, and deleting a post leaves its uploaded file in storage. Use text for private room coordination and avoid attachments entirely through this kit.

## CI notifications

Default `ci-bot init` creates a webhook workflow in the configured channel as the active assistant. Its fixed template is:

```text
[{{trigger.repo}}] {{trigger.event}}: {{trigger.title}} ({{trigger.actor}}) {{trigger.url}}
```

The caller supplies flat JSON fields. Nested GitHub webhook payloads do not resolve through this template. Use the supplied GitHub Actions workflow; GitHub's built-in repository webhooks cannot send the required secret header. Never put the secret in a query string, config, log or chat. It goes to `BUZZ_WEBHOOK_SECRET` via `gh secret set` stdin; `BUZZ_WEBHOOK_URL` is a repository variable.

The relay authors webhook messages using its own key. Execution uses the workflow owner's channel authority: keep the initializing assistant enrolled and in that channel. A valid request should return 202, an invalid secret 401, and a deleted/inaccessible workflow 404. `doctor` warns when the recorded workflow is not visible. These semantics come from the approved design; this kit's own live acceptance is tracked separately in [build notes](build-notes.md).

The kit's disposable live check on 2026-09-28 found that an ordinary workflow update **preserves the existing secret** and returns no replacement secret. The old secret still returned 202 after the update. A wrong secret returned 401 without posting; deleting the workflow made its endpoint return 404. There is no `ci-bot rotate` command. If a secret is exposed, have the operator revoke the workflow, confirm calls are rejected, then explicitly reinitialize and update GitHub; never paste the replacement secret into chat.

Both real GitHub PR-opened and PR-merged messages were signed by the public key advertised in the relay's NIP-11 `self` field. That server's separate `pubkey` field was null, so do not rely on it alone when checking authorship. The exact Buzz Desktop author label remains pending a UI observation; relay signing identity and the displayed label are separate checks.

Optional `--mode bot-key` uses the reviewed Buzz 0.5.25 x86_64 artifact and a named bot. Give it only the intended channel access. Keep GitHub secrets out of fork PR jobs; no checkout or execution of PR code is necessary for notifications. Review [dependency limits](security/dependency-log.md) before deployment.

## iOS notifications

Pair the mobile client with Desktop, enable notifications in iOS settings and select the intended channel's notification preferences. Server-side push availability depends on the deployed Buzz version and gateway configuration; installing the kit does not enable it. Have the deployment operator follow the version-matched [upstream push gateway guide](https://github.com/block/buzz/blob/desktop-v0.5.25/docs/push-gateway-deployment.md) and prove delivery on a physical device. The 0.5.25 guide describes an App Attest/APNs gateway with a constrained application profile, so do not assume arbitrary self-hosted clients can enroll. Keep APNs credentials at the gateway, never in this kit or a room message.

## Initializer and recovery

Run `buzz-kit ci-bot init --repo owner/repo --ci-workflow "CI"` from the configured project. Substitute the exact existing CI workflow name. Omit `--repo` to resolve the current GitHub repository. `--name` names the Buzz workflow; in explicit `--mode bot-key` it names the new local bot identity. GitHub CLI must already be installed and authenticated with permission to set that repository's Actions secrets and variables.

The initializer refuses to overwrite `.github/workflows/buzz-notify.yml`. Review and commit that generated file yourself; it does not push or open a PR. Missing fork secrets skip notification safely. The job never checks out PR code.

Setup reserves an owner-only recovery record under `~/.config/buzz-kit/ci/`, keyed by the canonical project path. The record contains mode, repository, phase and a public workflow ID when known; it contains no secret or relay/channel configuration. Keep it local. A failed GitHub export may leave a remote workflow or some GitHub settings in place. The error reports the record path and any known workflow ID; rerunning refuses to create duplicates.

For a failed webhook setup, inspect the indicated workflow as its owner. If creation delivery was uncertain, list workflows in the approved channel and identify the fixed notifier definition before proceeding. The one-time secret is not recoverable from the kit's record. After explicitly revoking the partial remote workflow and confirming its endpoint is rejected, remove that exact local recovery record and any partial generated workflow file, then reinitialize. Existing GitHub values may need replacement or removal in that same authorized repository. Never delete unrelated workflows or bulk-remove recovery files.

For a failed bot-key export, the new local bot key remains available. Re-export directly to `gh secret set` stdin through a trusted local helper; never display it. Alternatively, explicitly remove that unneeded bot identity, reconcile partial GitHub settings, and clear only its setup record before starting again. Successful interactive bot setup offers local-key deletion; noninteractive/JSON mode retains it and prints the public follow-up command. Server enrollment and channel role `bot` still require the operator.
