#!/bin/sh
# Synthetic archives and fake download/attestation tools; never hits the network.
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
TASK=$(mktemp -d "${TMPDIR:-/tmp}/buzz-kit-launcher.XXXXXX")
trap 'rm -rf "$TASK"' EXIT HUP INT TERM
fail() { printf 'FAIL: %s\n' "$*" >&2; exit 1; }
KIT="$TASK/kit with spaces"
mkdir -p "$KIT/bin" "$KIT/release" "$TASK/tools" "$TASK/assets" "$TASK/source"
cp "$ROOT/bin/buzz-kit" "$KIT/bin/buzz-kit"
cat > "$TASK/source/buzz-kit" <<'EOF'
#!/bin/sh
printf 'synthetic release binary: %s\n' "$*"
EOF
chmod +x "$TASK/source/buzz-kit"
for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
    tar -czf "$TASK/assets/buzz-kit-$target.tar.gz" -C "$TASK/source" buzz-kit
done
cat > "$TASK/tools/curl" <<'EOF'
#!/bin/sh
set -eu
out=
while [ "$#" -gt 0 ]; do
    case "$1" in -o|--output) out=$2; shift 2 ;; *) url=$1; shift ;; esac
done
printf '%s\n' "$url" >> "$TEST_DOWNLOAD_LOG"
cp "$TEST_ASSETS/${url##*/}" "$out"
EOF
cat > "$TASK/tools/uname" <<'EOF'
#!/bin/sh
case "$1" in -s) printf '%s\n' "$TEST_OS" ;; -m) printf '%s\n' "$TEST_ARCH" ;; esac
EOF
# Emulated targets still run on the current machine's real mv implementation.
REAL_OS=$(uname -s)
cat > "$TASK/tools/mv" <<'EOF'
#!/bin/sh
if [ "$1" = -fT ] && [ "$TEST_REAL_OS" = Darwin ]; then shift; exec /bin/mv -fh "$@"; fi
if [ "$1" = -fh ] && [ "$TEST_REAL_OS" = Linux ]; then shift; exec /bin/mv -fT "$@"; fi
exec /bin/mv "$@"
EOF
chmod +x "$TASK/tools/"*
PATH="$TASK/tools:/usr/bin:/bin:/usr/sbin:/sbin"
TEST_REAL_OS=$REAL_OS TEST_ASSETS="$TASK/assets" TEST_DOWNLOAD_LOG="$TASK/downloads"
export PATH TEST_REAL_OS TEST_ASSETS TEST_DOWNLOAD_LOG
unset BUZZ_KIT_BIN
write_checksums() {
    printf 'bin_tag: bin-v0.1.0\n' > "$KIT/release/checksums.txt"
    for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
        digest=$(shasum -a 256 "$TASK/assets/buzz-kit-$target.tar.gz" | awk '{print $1}')
        printf '%s  buzz-kit-%s.tar.gz\n' "$digest" "$target" >> "$KIT/release/checksums.txt"
    done
}
write_checksums
TEST_OS=Darwin TEST_ARCH=arm64;export TEST_OS TEST_ARCH
HOME="$TASK/implicit-home";XDG_DATA_HOME="$HOME/data";export HOME XDG_DATA_HOME
mkdir -p "$HOME"
"$KIT/bin/buzz-kit" probe > "$TASK/implicit-output"
[ "$(cat "$TASK/implicit-output")" = 'synthetic release binary: probe' ] || fail 'implicit bootstrap polluted command stdout'
[ ! -e "$XDG_DATA_HOME/buzz-kit/.bootstrap-lock" ] || fail 'implicit bootstrap left its lock'
for pair in Darwin:arm64 Darwin:x86_64 Linux:x86_64 Linux:aarch64; do
    TEST_OS=${pair%:*} TEST_ARCH=${pair#*:};export TEST_OS TEST_ARCH
    HOME="$TASK/home $TEST_OS $TEST_ARCH ' quoted";XDG_DATA_HOME="$HOME/data";export HOME XDG_DATA_HOME
    mkdir -p "$HOME"
    (cd /; "$KIT/bin/buzz-kit" bootstrap) > "$TASK/bootstrap"
    [ "$(wc -l < "$HOME/.local/bin/buzz-kit" | tr -d ' ')" = 3 ] || fail 'shim line count'
    "$HOME/.local/bin/buzz-kit" 'an argument' > "$TASK/output"
    grep -q 'synthetic release binary: an argument' "$TASK/output" || fail 'shim argument preservation'
    before=$(wc -l < "$TEST_DOWNLOAD_LOG")
    "$KIT/bin/buzz-kit" bootstrap > /dev/null
    [ "$(wc -l < "$TEST_DOWNLOAD_LOG")" = "$before" ] || fail 'cache hit downloaded'
done
printf 'PASS: all four targets, cache hit, spaces, shim, different cwd\n'
TEST_OS=Darwin TEST_ARCH=arm64;export TEST_OS TEST_ARCH
HOME="$TASK/security-home";XDG_DATA_HOME="$HOME/data";export HOME XDG_DATA_HOME
mkdir -p "$HOME/.local/bin"
printf '#!/bin/sh\n# foreign\nexit 42\n' > "$HOME/.local/bin/buzz-kit"
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1; then fail 'foreign shim overwritten';fi
[ ! -e "$XDG_DATA_HOME" ] || fail 'foreign refusal mutated cache'
rm "$HOME/.local/bin/buzz-kit"
cp "$KIT/release/checksums.txt" "$TASK/good-checksums"
sed 's/^[0-9a-f][0-9a-f]*/0000000000000000000000000000000000000000000000000000000000000000/' "$TASK/good-checksums" > "$KIT/release/checksums.txt"
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1;then fail 'checksum mismatch accepted';fi
[ ! -e "$XDG_DATA_HOME/buzz-kit/current" ] || fail 'mismatch changed current'
cp "$TASK/good-checksums" "$KIT/release/checksums.txt"
cat > "$TASK/tools/gh" <<'EOF'
#!/bin/sh
exit 1
EOF
chmod +x "$TASK/tools/gh"
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1;then fail 'attestation failure accepted';fi
[ ! -e "$XDG_DATA_HOME/buzz-kit/current" ] || fail 'failed attestation changed current'
rm "$TASK/tools/gh"
"$KIT/bin/buzz-kit" bootstrap > /dev/null
cp "$HOME/.local/bin/buzz-kit" "$TASK/shim-before"
printf 'corrupted\n' > "$XDG_DATA_HOME/buzz-kit/current/buzz-kit"
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1;then fail 'corrupt cached executable accepted';fi
cmp "$HOME/.local/bin/buzz-kit" "$TASK/shim-before" || fail 'failed verification changed shim'
printf 'PASS: foreign shim, checksum, attestation and cached-binary integrity refusals\n'
TEST_OS=Unknown;export TEST_OS
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1;then fail 'unsupported OS accepted';fi
TEST_OS=Darwin;export TEST_OS
HOME="$TASK/unsafe-tar-home";XDG_DATA_HOME="$HOME/data";export HOME XDG_DATA_HOME
mkdir -p "$HOME"
rm "$TASK/source/buzz-kit";ln -s /bin/sh "$TASK/source/buzz-kit"
tar -czf "$TASK/assets/buzz-kit-aarch64-apple-darwin.tar.gz" -C "$TASK/source" buzz-kit
write_checksums
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1;then fail 'archive symlink accepted';fi
[ ! -e "$XDG_DATA_HOME/buzz-kit/current" ] || fail 'unsafe archive changed current'
rm "$TASK/source/buzz-kit"
printf 'escape' > "$TASK/escape"
tar -czPf "$TASK/assets/buzz-kit-aarch64-apple-darwin.tar.gz" -C "$TASK/source" ../escape
write_checksums
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1;then fail 'archive traversal accepted';fi
[ ! -e "$XDG_DATA_HOME/buzz-kit/current" ] || fail 'unsafe path changed current'
printf 'not a gzip archive' > "$TASK/assets/buzz-kit-aarch64-apple-darwin.tar.gz"
write_checksums
if "$KIT/bin/buzz-kit" bootstrap > /dev/null 2>&1;then fail 'invalid archive accepted';fi
[ ! -e "$XDG_DATA_HOME/buzz-kit/current" ] || fail 'invalid archive changed current'
printf 'PASS: unsupported target and unsafe tar refusal\n'
