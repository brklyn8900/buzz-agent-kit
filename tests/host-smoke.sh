#!/bin/sh
# Real local-source host install, entirely within a disposable HOME.
# This is not tag-ref or fresh-session acceptance.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
command -v claude >/dev/null
command -v codex >/dev/null
TEST_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/buzz-kit-hosts.XXXXXX")
trap 'rm -rf "$TEST_ROOT"' EXIT HUP INT TERM
HOME="$TEST_ROOT/home"
CODEX_HOME="$HOME/.codex"
CLAUDE_CONFIG_DIR="$HOME/.claude"
XDG_DATA_HOME="$HOME/data"
BUZZ_KIT_BIN="$ROOT/scripts/m0-dev-binary"
export HOME CODEX_HOME CLAUDE_CONFIG_DIR XDG_DATA_HOME BUZZ_KIT_BIN
mkdir -p "$CODEX_HOME"
cd "$TEST_ROOT"
[ ! -e "$HOME/.local/bin" ]
[ ! -e "$XDG_DATA_HOME" ]
"$ROOT/install.sh" --dev > "$TEST_ROOT/install.log" 2>&1 || {
    cat "$TEST_ROOT/install.log" >&2
    exit 1
}
grep -q 'Installed ✓ (explicit development checkout' "$TEST_ROOT/install.log"
for HOST in claude codex; do
    "$HOST" plugin list --json > "$TEST_ROOT/$HOST.json"
    grep -q 'buzz-kit@buzz-agent-kit' "$TEST_ROOT/$HOST.json"
    printf 'PASS: %s real local-source installation in disposable HOME\n' "$HOST"
done
unset BUZZ_KIT_BIN
PATH="$HOME/.local/bin:$PATH"
export PATH
[ "$(command -v buzz-kit)" = "$HOME/.local/bin/buzz-kit" ]
buzz-kit doctor > "$TEST_ROOT/doctor"
grep -q 'M0 development stub' "$TEST_ROOT/doctor"
printf 'PASS: shell resolves installed shim after explicit PATH addition\n'
if "$ROOT/install.sh" --dev > "$TEST_ROOT/collision.log" 2>&1; then
    printf 'FAIL: installer replaced an existing plugin\n' >&2
    exit 1
fi
grep -q 'refusing replacement' "$TEST_ROOT/collision.log"
printf 'PASS: second install refuses existing plugin\n'
