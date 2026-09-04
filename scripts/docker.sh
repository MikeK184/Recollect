#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Public development images need no registry login. Keep desktop credentials untouched.
mkdir -p .cache/docker
if [[ ! -f .cache/docker/config.json ]]; then
  printf '{}\n' > .cache/docker/config.json
fi
RECOLLECT_DOCKER_HOST="$(docker context inspect --format '{{.Endpoints.docker.Host}}')"
if [[ "${1:-}" == compose ]] && command -v docker-compose >/dev/null; then
  shift
  exec env DOCKER_CONFIG="$PWD/.cache/docker" DOCKER_HOST="$RECOLLECT_DOCKER_HOST" docker-compose "$@"
fi
exec docker --config "$PWD/.cache/docker" --host "$RECOLLECT_DOCKER_HOST" "$@"
