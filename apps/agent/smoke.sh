#!/usr/bin/env bash
# End-to-end smoke test: real agent binary against the real local Podman.
set -u

TOKEN="smoke-secret-0123456789abcdef"
BASE="http://127.0.0.1:9091"
ID="smoke-agent"
AUTH=(-H "Authorization: Bearer $TOKEN")

cd "$(dirname "$0")"
cargo build --quiet || exit 1

AGENT_TOKEN="$TOKEN" AGENT_BIND=127.0.0.1:9091 MINECRAFT_IMAGE=nginx:alpine \
  ./target/debug/agent >/tmp/opencode/agent-smoke.log 2>&1 &
AGENT_PID=$!

cleanup() {
  kill "$AGENT_PID" 2>/dev/null
  wait "$AGENT_PID" 2>/dev/null
  podman rm -f "pumapanel-$ID" >/dev/null 2>&1
  podman volume rm "pumapanel-$ID" >/dev/null 2>&1
}
trap cleanup EXIT

for _ in $(seq 1 40); do
  curl -sf "$BASE/health" >/dev/null && break
  sleep 0.25
done

echo "== startup log =="
cat /tmp/opencode/agent-smoke.log

echo "== unauthenticated /runtime =="
curl -s -o /dev/null -w 'status=%{http_code}\n' "$BASE/runtime"

echo "== /runtime =="
curl -s "${AUTH[@]}" "$BASE/runtime" | jq -c .

echo "== POST /servers =="
curl -s "${AUTH[@]}" -H 'content-type: application/json' \
  -d "{\"id\":\"$ID\",\"version\":\"1.21.8\",\"memoryMb\":1024,\"hostPort\":25599,\"javaMin\":21}" \
  -w '\nstatus=%{http_code}\n' "$BASE/servers"

echo "== container labels as the runtime sees them =="
podman inspect "pumapanel-$ID" | jq -c '.[0].Config.Labels'
podman inspect "pumapanel-$ID" | jq -c '.[0].Config.Env | map(select(test("^(EULA|TYPE|VERSION|MEMORY|INIT_MEMORY)=")))'

echo "== GET /servers =="
curl -s "${AUTH[@]}" "$BASE/servers" | jq -c .

echo "== POST /servers/$ID/stop =="
curl -s "${AUTH[@]}" -X POST "$BASE/servers/$ID/stop" | jq -c .

echo "== POST /servers/$ID/start =="
curl -s "${AUTH[@]}" -X POST "$BASE/servers/$ID/start" | jq -c .

echo "== DELETE /servers/$ID?removeData=true =="
curl -s "${AUTH[@]}" -X DELETE -o /dev/null -w 'status=%{http_code}\n' "$BASE/servers/$ID?removeData=true"

echo "== leftovers =="
podman ps -a --filter "label=pumapanel.managed=true" --format '{{.Names}}' || true
podman volume ls --format '{{.Name}}' | grep "pumapanel-$ID" || echo "(no volume left)"

echo "== repeated delete (idempotency) =="
curl -s "${AUTH[@]}" -X DELETE -o /dev/null -w 'status=%{http_code}\n' "$BASE/servers/$ID?removeData=true"
