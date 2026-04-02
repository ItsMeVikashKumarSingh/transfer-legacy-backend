#!/usr/bin/env sh
set -eu

if [ -z "${BAO_ADDR:-}" ] || [ -z "${BAO_TOKEN:-}" ]; then
  echo "BAO_ADDR and BAO_TOKEN must be set"
  exit 1
fi

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)

bao secrets enable transit
bao secrets enable -path=kv kv-v2
bao write transit/keys/tl-signing type=ed25519
bao policy write tl-api "${SCRIPT_DIR}/policy-api.hcl"
bao auth enable approle
bao write auth/approle/role/tl-api policies=tl-api token_ttl=24h token_max_ttl=72h
