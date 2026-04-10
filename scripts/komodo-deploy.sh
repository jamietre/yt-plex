#!/usr/bin/env bash
# Trigger a Komodo stack redeploy via the Execute API.
# Requires KOMODO_API_KEY and KOMODO_API_SECRET to be set in the environment.
# KOMODO_HOST defaults to http://PRIVATE_IP_REDACTED:9120
set -euo pipefail

HOST="${KOMODO_HOST:-http://PRIVATE_IP_REDACTED:9120}"
KEY="${KOMODO_API_KEY:-}"
SECRET="${KOMODO_API_SECRET:-}"
STACK="${KOMODO_STACK:-yt-plex}"

if [ -z "$KEY" ] || [ -z "$SECRET" ]; then
    echo "Set KOMODO_API_KEY and KOMODO_API_SECRET in .env to trigger redeploy automatically."
    echo "Generate them in the Komodo UI: http://PRIVATE_IP_REDACTED:9120 → User Settings → API Keys"
    exit 0
fi

echo "Triggering Komodo redeploy for stack: $STACK"

RESPONSE=$(curl -sf -X POST "$HOST/execute/DeployStack" \
    -H "Content-Type: application/json" \
    -H "X-Api-Key: $KEY" \
    -H "X-Api-Secret: $SECRET" \
    -d "{\"stack\":\"$STACK\",\"services\":[]}")

STATUS=$(echo "$RESPONSE" | python3 -c "import sys,json; d=json.load(sys.stdin); print('ok' if d.get('success') else d.get('error','unknown'))" 2>/dev/null || echo "$RESPONSE")
echo "Deploy triggered: $STATUS"
