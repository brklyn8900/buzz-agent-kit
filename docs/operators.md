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

Secret rotation through workflow updates and the exact desktop author display are still pending the kit's disposable live workflow check. Do not assume an ordinary workflow update rotates the secret. There is no `ci-bot rotate` command. If a secret is exposed, have the operator revoke the workflow, confirm calls are rejected, then explicitly reinitialize and update GitHub; never paste the replacement secret into chat.

Optional `--mode bot-key` uses the reviewed Buzz 0.5.25 x86_64 artifact and a named bot. Give it only the intended channel access. Keep GitHub secrets out of fork PR jobs; no checkout or execution of PR code is necessary for notifications. Review [dependency limits](security/dependency-log.md) before deployment.

## iOS notifications

Pair the mobile client with Desktop, enable notifications in iOS settings and select the intended channel's notification preferences. Server-side push availability depends on the deployed Buzz version and gateway configuration; installing the kit does not enable it. Have the deployment operator follow the version-matched [upstream push gateway guide](https://github.com/block/buzz/blob/desktop-v0.5.25/docs/push-gateway-deployment.md) and prove delivery on a physical device. The 0.5.25 guide describes an App Attest/APNs gateway with a constrained application profile, so do not assume arbitrary self-hosted clients can enroll. Keep APNs credentials at the gateway, never in this kit or a room message.
