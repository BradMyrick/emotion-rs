#!/usr/bin/env bash
# sync-example.sh — sync the Rust example + shim into the server's ps2dev
# examples tree for ps2-build.sh.
#
# Server layout after sync:
#   <dest>/Makefile, <dest>/main.c   <- examples/hello-rs/shim/
#   <dest>/rust/                     <- workspace subset: root manifests,
#                                       targets/ spec, the crate itself
#
# rust/target and rust/.cargo-home are server-side build caches:
# excluded here and protected from --delete.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${1:-${PS2_SYNC_DEST:-}}"
if [[ -z "$DEST" ]]; then
  echo "usage: $0 [user@host:path] (or export PS2_SYNC_DEST)" >&2
  exit 1
fi

rsync -av --delete \
  --exclude target/ --exclude .cargo-home/ \
  "$REPO/Cargo.toml" "$REPO/rust-toolchain.toml" "$REPO/targets" \
  "$DEST/rust/"

rsync -av --delete --exclude shim/ \
  "$REPO/examples/hello-rs/" "$DEST/rust/examples/hello-rs/"

rsync -av "$REPO/examples/hello-rs/shim/" "$DEST/"

echo "synced -> $DEST (build: ps2-build.sh ~/homelab/ps2dev/examples/hello-rs \"Hello Rust (Homebrew)\")"
