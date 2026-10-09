#!/usr/bin/env bash
set -euo pipefail
umask 077

RECOLLECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RECOLLECT_RUST_TOOLS="$RECOLLECT_ROOT/.codex/tools/rust-analyzer"
RECOLLECT_GRAFT_TOOLS="$RECOLLECT_ROOT/.codex/tools/graft"
RECOLLECT_RUST_CACHE="$RECOLLECT_ROOT/.cache/rust-navigation"
if [[ "$(uname -s)" != Darwin || "$(uname -m)" != arm64 ]]; then
  printf '%s\n' 'This setup currently verifies Apple Silicon macOS only.' >&2
  exit 1
fi
for RECOLLECT_TOOL in cargo rustc node npm curl gzip shasum; do
  if ! command -v "$RECOLLECT_TOOL" >/dev/null; then
    printf '%s\n' "Rust navigation setup requires $RECOLLECT_TOOL on PATH." >&2; exit 1
  fi
done
export TMPDIR="$RECOLLECT_RUST_CACHE/tmp"
export CARGO_HOME="$RECOLLECT_RUST_CACHE/cargo"
export CARGO_TARGET_DIR="$RECOLLECT_RUST_CACHE/bridge-target"
export DO_NOT_TRACK=1 CI=1
export npm_config_cache="$RECOLLECT_RUST_CACHE/npm"
export npm_config_devdir="$RECOLLECT_RUST_CACHE/node-gyp"
export npm_config_userconfig=/dev/null
mkdir -p "$TMPDIR" "$CARGO_HOME" "$RECOLLECT_RUST_TOOLS/installed/bin"
cd "$RECOLLECT_ROOT"

if [[ ! -x "$RECOLLECT_RUST_TOOLS/installed/bin/rust-analyzer" ]]; then
  curl --fail --location --retry 2 \
    'https://github.com/rust-lang/rust-analyzer/releases/download/2026-10-05/rust-analyzer-aarch64-apple-darwin.gz' \
    --output "$RECOLLECT_RUST_CACHE/rust-analyzer.gz"
  gzip -dc "$RECOLLECT_RUST_CACHE/rust-analyzer.gz" > "$RECOLLECT_RUST_TOOLS/installed/bin/rust-analyzer.pending"
  chmod 700 "$RECOLLECT_RUST_TOOLS/installed/bin/rust-analyzer.pending"
  "$RECOLLECT_RUST_TOOLS/installed/bin/rust-analyzer.pending" --version
  mv "$RECOLLECT_RUST_TOOLS/installed/bin/rust-analyzer.pending" "$RECOLLECT_RUST_TOOLS/installed/bin/rust-analyzer"
fi
cargo install rust-analyzer-mcp --version 0.4.0 --locked --jobs 2 \
  --root "$RECOLLECT_RUST_TOOLS/installed"
# Cache trusted workspace dependencies locally before runtime offline analysis.
cargo fetch --locked
# rust-analyzer queries Cargo metadata for the standard library too. Use an owned
# source copy: metadata/lockfiles must not be generated under the rustup home.
RECOLLECT_SYSROOT="$(rustc --print sysroot)"
if [[ ! -f "$RECOLLECT_SYSROOT/lib/rustlib/src/rust/library/core/src/lib.rs" ]]; then
  printf '%s\n' 'The active toolchain needs rust-src; this setup does not modify rustup at home.' >&2
  exit 1
fi
RECOLLECT_TOOLCHAIN_ID="$(rustc --version --verbose | shasum -a 256 | awk '{print $1}')"
RECOLLECT_SYSROOT_SOURCE="$RECOLLECT_RUST_CACHE/sysroot/$RECOLLECT_TOOLCHAIN_ID/library"
if [[ ! -d "$RECOLLECT_SYSROOT_SOURCE" ]]; then
  mkdir -p "$(dirname "$RECOLLECT_SYSROOT_SOURCE")"
  cp -R "$RECOLLECT_SYSROOT/lib/rustlib/src/rust/library" "$RECOLLECT_SYSROOT_SOURCE.pending.$$"
  mv "$RECOLLECT_SYSROOT_SOURCE.pending.$$" "$RECOLLECT_SYSROOT_SOURCE"
fi
RUSTC_BOOTSTRAP=1 cargo fetch -Z unstable-options --locked \
  --manifest-path "$RECOLLECT_SYSROOT_SOURCE/Cargo.toml"
if [[ ! -f "$RECOLLECT_GRAFT_TOOLS/package-lock.json" ]]; then
  npm install --prefix "$RECOLLECT_GRAFT_TOOLS" --package-lock-only --ignore-scripts --no-audit --no-fund
fi
npm ci --prefix "$RECOLLECT_GRAFT_TOOLS" --ignore-scripts --no-audit --no-fund
# Native parsers need their own build scripts; do not run Graft's postinstall.
npm rebuild --prefix "$RECOLLECT_GRAFT_TOOLS" --ignore-scripts=false \
  tree-sitter tree-sitter-typescript tree-sitter-javascript tree-sitter-python \
  tree-sitter-go tree-sitter-java tree-sitter-kotlin tree-sitter-php \
  tree-sitter-r tree-sitter-swift
"$RECOLLECT_RUST_TOOLS/installed/bin/rust-analyzer-mcp" --version
"$RECOLLECT_GRAFT_TOOLS/graft" --version
"$RECOLLECT_GRAFT_TOOLS/graft" build --lsp --no-follow-nested-repos --no-follow-submodules
"$RECOLLECT_GRAFT_TOOLS/graft" check
