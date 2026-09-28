#!/bin/sh
# M0 smoke tests: installed system tools only; no network or host mutations.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
for file in .claude-plugin/plugin.json .claude-plugin/marketplace.json \
    .agents/plugins/marketplace.json plugin.json .version-bump.json skills/setup/SKILL.md; do
    [ -f "$ROOT/$file" ] || fail "missing $file"
done
for file in .claude-plugin/plugin.json plugin.json; do
    grep -q '"name": "buzz-kit"' "$ROOT/$file" || fail "plugin identity: $file"
    grep -q '"version":' "$ROOT/$file" || fail "missing version: $file"
done
for file in .claude-plugin/marketplace.json .agents/plugins/marketplace.json; do
    grep -q '"name": "buzz-agent-kit"' "$ROOT/$file" || fail "marketplace identity: $file"
done
if grep -E '"(hooks|mcpServers)"' "$ROOT/plugin.json" "$ROOT/.claude-plugin/plugin.json"; then
    fail 'forbidden plugin integration'
fi
printf 'PASS: M0 package files and identity\n'
[ "${1:-}" = '--packaging-only' ] && exit 0
for file in bin/buzz-kit install.sh scripts/m0-dev-binary; do
    [ -x "$ROOT/$file" ] || fail "missing executable $file"
done
TEST_ROOT=$(mktemp -d "${TMPDIR:-/tmp}/buzz-kit-m0.XXXXXX")
trap 'rm -rf "$TEST_ROOT"' EXIT HUP INT TERM
HOME="$TEST_ROOT/home with spaces and ' quote"
XDG_DATA_HOME="$HOME/data with spaces"
export HOME XDG_DATA_HOME
mkdir -p "$HOME"
unset BUZZ_KIT_BIN
if "$ROOT/bin/buzz-kit" bootstrap >"$TEST_ROOT/no-dev" 2>&1; then
    fail 'M0 bootstrap must require explicit development binary'
fi
[ ! -e "$XDG_DATA_HOME/buzz-kit/current" ] || fail 'failed bootstrap mutated current'
BUZZ_KIT_BIN="$ROOT/scripts/m0-dev-binary"
export BUZZ_KIT_BIN
cd "$TEST_ROOT"
"$ROOT/bin/buzz-kit" bootstrap >"$TEST_ROOT/bootstrap"
"$ROOT/bin/buzz-kit" bootstrap >"$TEST_ROOT/bootstrap-again"
[ -x "$HOME/.local/bin/buzz-kit" ] || fail 'missing shim'
[ -L "$XDG_DATA_HOME/buzz-kit/current" ] || fail 'current is not symlink'
[ "$(wc -l < "$HOME/.local/bin/buzz-kit" | tr -d ' ')" = 3 ] || fail 'shim is not three lines'
grep -q 'PATH' "$TEST_ROOT/bootstrap" || fail 'missing PATH guidance'
unset BUZZ_KIT_BIN
"$HOME/.local/bin/buzz-kit" doctor >"$TEST_ROOT/doctor"
grep -q 'M0 development stub' "$TEST_ROOT/doctor" || fail 'doctor must disclose stub'
"$ROOT/bin/buzz-kit" probe 'argument with spaces' >"$TEST_ROOT/arguments"
grep -q 'argument with spaces' "$TEST_ROOT/arguments" || fail 'argument was lost'
mkdir -p "$TEST_ROOT/links"
ln -s "$ROOT/bin/buzz-kit" "$TEST_ROOT/links/buzz-kit"
"$TEST_ROOT/links/buzz-kit" doctor >"$TEST_ROOT/symlink"
grep -q 'M0 development stub' "$TEST_ROOT/symlink" || fail 'symlink invocation failed'
printf 'PASS: explicit bootstrap, clean HOME, spaces, shim, current, arguments, symlink\n'
printf '#!/bin/sh\n# foreign shim\nexit 42\n' > "$HOME/.local/bin/buzz-kit"
cp "$HOME/.local/bin/buzz-kit" "$TEST_ROOT/foreign-before"
readlink "$XDG_DATA_HOME/buzz-kit/current" > "$TEST_ROOT/current-before"
BUZZ_KIT_BIN="$ROOT/scripts/m0-dev-binary"
export BUZZ_KIT_BIN
if "$ROOT/bin/buzz-kit" bootstrap >"$TEST_ROOT/foreign" 2>&1; then fail 'overwrote foreign shim'; fi
cmp "$HOME/.local/bin/buzz-kit" "$TEST_ROOT/foreign-before" || fail 'foreign shim changed'
readlink "$XDG_DATA_HOME/buzz-kit/current" > "$TEST_ROOT/current-after"
cmp "$TEST_ROOT/current-before" "$TEST_ROOT/current-after" || fail 'foreign refusal changed current'
printf 'PASS: foreign shim refusal preserves existing files\n'
unset BUZZ_KIT_BIN
UNTAGGED="$TEST_ROOT/untagged-checkout"
mkdir "$UNTAGGED"
cp "$ROOT/install.sh" "$UNTAGGED/install.sh"
git -C "$UNTAGGED" init -q -b master
git -C "$UNTAGGED" add install.sh
git -C "$UNTAGGED" -c user.name='M0 test' -c user.email='test@example.com' \
    -c commit.gpgsign=false commit -qm 'test: untagged installer fixture'
if "$UNTAGGED/install.sh" --dry-run >"$TEST_ROOT/untagged" 2>&1; then
    fail 'untagged checkout accepted without --dev'
fi
mkdir -p "$UNTAGGED/.claude-plugin" "$UNTAGGED/crates/buzz-kit"
for manifest in plugin.json .claude-plugin/plugin.json .claude-plugin/marketplace.json crates/buzz-kit/Cargo.toml; do
    cp "$ROOT/$manifest" "$UNTAGGED/$manifest"
done
git -C "$UNTAGGED" add .
git -C "$UNTAGGED" -c user.name='M0 test' -c user.email='test@example.com' -c commit.gpgsign=false commit -qm 'test: tagged manifest fixture'
VERSION=$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$ROOT/plugin.json")
git -C "$UNTAGGED" tag "v$VERSION"
"$UNTAGGED/install.sh" --dry-run >"$TEST_ROOT/tagged"
grep -q "v$VERSION" "$TEST_ROOT/tagged" || fail 'tagged dry-run omitted ref'
HOME="$TEST_ROOT/dry-home"
XDG_DATA_HOME="$HOME/data"
export HOME XDG_DATA_HOME
"$ROOT/install.sh" --dev --dry-run >"$TEST_ROOT/dry-run"
[ ! -e "$HOME" ] || fail 'dry-run mutated HOME'
grep -q 'bootstrap' "$TEST_ROOT/dry-run" || fail 'dry-run omitted bootstrap'
printf 'PASS: untagged refusal and mutation-free dry-run\n'
