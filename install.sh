#!/bin/sh
# Tag-pinned installer; --dev explicitly permits a local development checkout.
set -eu
die() { printf 'install: %s\n' "$*" >&2; exit 1; }
KIT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
DEV=0
DRY_RUN=0
for ARG in "$@"; do
    case "$ARG" in
        --dev) DEV=1 ;;
        --dry-run) DRY_RUN=1 ;;
        *) die 'usage: install.sh [--dev] [--dry-run]' ;;
    esac
done
TAG=$(git -C "$KIT_DIR" describe --tags --exact-match HEAD 2>/dev/null || true)
if [ "$DEV" -eq 0 ]; then
    [ -z "${BUZZ_KIT_BIN:-}" ] || die 'BUZZ_KIT_BIN is a development override; use --dev explicitly'
    printf '%s\n' "$TAG" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z]+([.-][0-9A-Za-z]+)*)?$' || die 'checkout is not at a plugin tag; use --dev only for local development'
    git -C "$KIT_DIR" diff --quiet HEAD -- || die 'release checkout has modified tracked files; restore it or use --dev'
    for MANIFEST in .claude-plugin/plugin.json plugin.json .claude-plugin/marketplace.json; do
        VERSION=$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$KIT_DIR/$MANIFEST")
        [ "$VERSION" = "${TAG#v}" ] || die 'plugin tag and manifest versions do not agree'
    done
    VERSION=$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$KIT_DIR/crates/buzz-kit/Cargo.toml")
    [ "$VERSION" = "${TAG#v}" ] || die 'plugin tag and binary package versions do not agree'
fi
HAS_CLAUDE=0
HAS_CODEX=0
if command -v claude >/dev/null 2>&1; then HAS_CLAUDE=1; fi
if command -v codex >/dev/null 2>&1; then HAS_CODEX=1; fi
[ "$HAS_CLAUDE$HAS_CODEX" != 00 ] || die 'neither Claude Code nor Codex is installed'
if [ "$DRY_RUN" -eq 1 ]; then
    printf 'Dry run: no changes.\n1. %s/bin/buzz-kit bootstrap\n' "$KIT_DIR"
    if [ "$DEV" -eq 1 ]; then
        printf '2. Add local marketplace %s and install buzz-kit@buzz-agent-kit in detected hosts.\n' "$KIT_DIR"
    else
        printf '2. Install brklyn8900/buzz-agent-kit at %s in detected hosts.\n' "$TAG"
    fi
    printf '3. %s/.local/bin/buzz-kit doctor\n' "$HOME"
    exit 0
fi

# Preflight all hosts before bootstrap or plugin-system changes. Conservative
# identity checks refuse collisions; real host commands own JSON persistence.
preflight() {
    HOST=$1
    PLUGINS=$("$HOST" plugin list --json) || die "$HOST plugin inventory unavailable"
    MARKETPLACES=$("$HOST" plugin marketplace list --json) || die "$HOST marketplace inventory unavailable"
    if printf '%s\n' "$PLUGINS" | grep -Eq '"(name|pluginId|id)"[[:space:]]*:[[:space:]]*"buzz-kit(@[^"]*)?"'; then
        die "$HOST already has buzz-kit; refusing replacement"
    fi
    if printf '%s\n' "$MARKETPLACES" | grep -Eq '"name"[[:space:]]*:[[:space:]]*"buzz-agent-kit"'; then
        die "$HOST already has buzz-agent-kit marketplace; refusing replacement"
    fi
}
[ "$HAS_CLAUDE" -eq 0 ] || preflight claude
[ "$HAS_CODEX" -eq 0 ] || preflight codex
for SKILLS_ROOT in "${CLAUDE_CONFIG_DIR:-$HOME/.claude}/skills" "${CODEX_HOME:-$HOME/.codex}/skills" "$HOME/.agents/skills"; do
    for SKILL in buzz-kit setup room; do
        SKILL_DIR="$SKILLS_ROOT/$SKILL"
        [ ! -e "$SKILL_DIR" ] && [ ! -L "$SKILL_DIR" ] || die "existing skill would conflict: $SKILL_DIR"
    done
done
CLAUDE_KNOWN="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/plugins/known_marketplaces.json"
CODEX_KNOWN="${CODEX_HOME:-$HOME/.codex}/config.toml"
for KNOWN in "$CLAUDE_KNOWN" "$CODEX_KNOWN"; do
    [ ! -L "$KNOWN" ] || die 'refusing a symlinked marketplace configuration'
    [ ! -e "$KNOWN" ] || [ -f "$KNOWN" ] || die 'marketplace configuration is not a regular file'
done
backup() {
    if [ -f "$1" ]; then
        BACKUP=$(mktemp "$1.buzz-kit-backup.XXXXXX")
        cp -p "$1" "$BACKUP"
        printf 'Marketplace backup: %s\n' "$BACKUP"
    fi
}

"$KIT_DIR/bin/buzz-kit" bootstrap
if [ "$HAS_CLAUDE" -eq 1 ]; then
    backup "$CLAUDE_KNOWN"
    if [ "$DEV" -eq 1 ]; then
        claude plugin marketplace add "$KIT_DIR"
    else
        claude plugin marketplace add "brklyn8900/buzz-agent-kit#$TAG"
    fi
    claude plugin install buzz-kit@buzz-agent-kit
fi
if [ "$HAS_CODEX" -eq 1 ]; then
    backup "$CODEX_KNOWN"
    if [ "$DEV" -eq 1 ]; then
        codex plugin marketplace add "$KIT_DIR"
    else
        codex plugin marketplace add brklyn8900/buzz-agent-kit --ref "$TAG"
    fi
    codex plugin add buzz-kit@buzz-agent-kit
fi
if "$HOME/.local/bin/buzz-kit" doctor; then
    DOCTOR_OK=1
else
    DOCTOR_STATUS=$?
    [ "$DOCTOR_STATUS" -eq 1 ] || die "installed doctor could not run normally (exit $DOCTOR_STATUS)"
    DOCTOR_OK=0
fi
if [ "$DEV" -eq 1 ]; then
    printf 'Installed ✓ (explicit development checkout; not a verified release)\n'
else
    printf 'Installed ✓ (verified binary, shim and detected host plugins)\n'
fi
[ "$DOCTOR_OK" -eq 1 ] || printf 'Next steps: doctor found setup steps still needed; run the setup skill to configure identity, server and channel.\n'
case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) printf 'Next steps: export PATH="$HOME/.local/bin:$PATH"\n' ;;
esac
printf 'Next steps: start a fresh Claude/Codex session.\n'
