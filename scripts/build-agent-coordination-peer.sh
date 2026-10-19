#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
fixture_dir="$PWD/.cache/agent-coordination-linux"
mkdir -p "$fixture_dir/cargo" "$fixture_dir/target"
# This compiler container and caches belong only to the synthetic A2A fixture.
# Source is read-only; the resulting binary runs inside the Ubuntu peer image.
./scripts/docker.sh run --rm --init \
  --label io.recollect.owner=agent-coordination-proof \
  -v "$PWD/Cargo.toml:/workspace/Cargo.toml:ro" \
  -v "$PWD/Cargo.lock:/workspace/Cargo.lock:ro" \
  -v "$PWD/crates:/workspace/crates:ro" \
  -v "$fixture_dir:/fixture" \
  -w /workspace \
  -e CARGO_HOME=/fixture/cargo -e CARGO_TARGET_DIR=/fixture/target \
  -e CARGO_BUILD_JOBS=3 -e CARGO_INCREMENTAL=0 -e CARGO_PROFILE_DEV_DEBUG=0 \
  rust:1.94-slim-bookworm@sha256:cf9dd0ec73e75f827fe59123fff9dc65af1a1c8363c3c31ee8d7f8ad0b6a5fb2 \
  sh -c 'cargo build --locked -p recollect-agent --bin recollect-plugin && cp /fixture/target/debug/recollect-plugin /fixture/recollect-plugin'
