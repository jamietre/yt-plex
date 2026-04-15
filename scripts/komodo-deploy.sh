#!/usr/bin/env bash
# Trigger a Komodo stack redeploy via the Execute API.
# Requires KOMODO_HOST, KOMODO_API_KEY, and KOMODO_API_SECRET to be set in the environment.
set -euo pipefail

HOST="${KOMODO_HOST:-}"
KEY="${KOMODO_API_KEY:-}"
SECRET="${KOMODO_API_SECRET:-}"
STACK="${KOMODO_STACK:-yt-plex}"

if [ -z "$HOST" ] || [ -z "$KEY" ] || [ -z "$SECRET" ]; then
    echo "Set KOMODO_HOST, KOMODO_API_KEY, and KOMODO_API_SECRET in .env to trigger redeploy."
    exit 1
fi

echo "Triggering Komodo redeploy for stack: $STACK"

RESPONSE=$(curl -sf -X POST "$HOST/execute/DeployStack" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $KEY" \
    -H "X-Api-Secret: $SECRET" \
    -d "{\"stack\":\"$STACK\",\"services\":[]}")

STATUS=$(echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print('ok' if d.get('success') else d.get('error','unknown'))" 2>/dev/null || echo "$RESPONSE")
echo "Deploy triggered: $STATUS"
