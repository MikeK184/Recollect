#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
set -a
source .env
set +a
export RECOLLECT_OIDC_ISSUER=http://127.0.0.1:5556/dex
export RECOLLECT_OIDC_CLIENT_ID=recollect-test
export RECOLLECT_OIDC_CLIENT_SECRET="$(python3 -c 'import secrets; print(secrets.token_urlsafe(32))')"
export DEX_CLIENT_SECRET="$RECOLLECT_OIDC_CLIENT_SECRET"
export RECOLLECT_OIDC_SCOPES='profile groups'
export RECOLLECT_TEST_DEX=1
RECOLLECT_DEX_NAME="recollect-oidc-test-$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
RECOLLECT_DEX_ID=''
cleanup() {
  if [[ -n "$RECOLLECT_DEX_ID" ]]; then ./scripts/docker.sh rm -f "$RECOLLECT_DEX_ID" >/dev/null; fi
}
trap cleanup EXIT
RECOLLECT_DEX_ID="$(./scripts/docker.sh run -d --rm --name "$RECOLLECT_DEX_NAME" --label recollect.test=oidc --memory=256m -p 127.0.0.1:5556:5556 -e DEX_CLIENT_SECRET -v "$PWD/infra/dex-test.yaml:/etc/dex/config.yaml:ro" ghcr.io/dexidp/dex:v2.45.1 dex serve /etc/dex/config.yaml)"
for attempt in {1..100}; do
  if curl -fsS "$RECOLLECT_OIDC_ISSUER/.well-known/openid-configuration" >/dev/null 2>&1; then break; fi
  sleep 0.1
done
curl -fsS "$RECOLLECT_OIDC_ISSUER/.well-known/openid-configuration" >/dev/null
if [[ "${1:-}" == ui ]]; then
  ./scripts/test-ui.sh
else
  cargo test -p recollect-server --test platform oidc_live -- --ignored --test-threads=1
fi
