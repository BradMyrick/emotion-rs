#!/usr/bin/env bash
# sync.sh — sync a workspace example + its build driver into the server's
# ps2dev examples tree for ps2-build.sh. Generalizes sync-example.sh.
#
# Usage: sync.sh <example-name> [user@host:path]
#   (or export PS2_SYNC_DEST for the default destination)
#
# Handles both example shapes:
#   shim-based  (hello-rs):  examples/<name>/shim/* -> $DEST/
#   pure-Rust   (pure-boot): examples/<name>/Makefile -> $DEST/Makefile
#
# rust/target and rust/.cargo-home are server-side build caches:
# excluded here and protected from --delete.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
NAME="${1:?usage: sync.sh <example-name> [user@host:path]}"
DEST="${2:-${PS2_SYNC_DEST:-}}"
if [[ -z "$DEST" ]]; then
  echo "usage: $0 <example-name> [user@host:path] (or export PS2_SYNC_DEST)" >&2
  exit 1
fi
EX="$REPO/examples/$NAME"
[[ -d "$EX" ]] || { echo "no such example: $EX" >&2; exit 1; }

# rsync only creates the last path component; pre-create the full remote tree.
if [[ "$DEST" == *:* ]]; then
  ssh "${DEST%%:*}" "mkdir -p '${DEST#*:}/rust/examples'"
fi

rsync -av --delete \
  --exclude target/ --exclude .cargo-home/ \
  "$REPO/Cargo.toml" "$REPO/rust-toolchain.toml" "$REPO/targets" \
  "$REPO/crates" \
  "$DEST/rust/"

# Whole examples/ tree: the workspace manifest names every example, so
# every member manifest must be present at each build destination.
rsync -av --delete \
  --exclude target/ --exclude .cargo-home/ \
  "$REPO/examples/" "$DEST/rust/examples/"

if [[ -f "$EX/Makefile" ]]; then
  if [[ "$DEST" == *:* ]]; then
    scp -q "$EX/Makefile" "$DEST/Makefile"
  else
    cp "$EX/Makefile" "$DEST/Makefile"
  fi
elif [[ -d "$EX/shim" ]]; then
  rsync -av "$EX/shim/" "$DEST/"
else
  echo "no build driver found in $EX (Makefile or shim/)" >&2
  exit 1
fi

echo "synced $NAME -> $DEST (build: ps2-build.sh $DEST \"<Game Name>\")"
