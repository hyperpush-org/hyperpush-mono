#!/usr/bin/env bash
set -euo pipefail

DEFAULT_PORT="18080"
PORT_VALUE="${PORT:-$DEFAULT_PORT}"
BASE_URL="${BASE_URL:-http://127.0.0.1:${PORT_VALUE}}"
LAST_RESPONSE=""
LAST_HEALTH_RESPONSE=""

usage() {
  echo "usage: bash deploy-smoke.sh" >&2
}

fail() {
  echo "[deploy-smoke] $1" >&2
  exit 1
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    fail "required command missing from PATH: $command_name"
  fi
}

json_field() {
  local field="$1"
  python3 -c '
import json
import sys

field = sys.argv[1]
data = json.load(sys.stdin)
value = data
for key in field.split("."):
    if not isinstance(value, dict):
        sys.exit(1)
    value = value.get(key)
    if value is None:
        sys.exit(1)
if isinstance(value, bool):
    print("true" if value else "false")
elif isinstance(value, (dict, list)):
    print(json.dumps(value, separators=(",", ":")))
else:
    print(value)
' "$field"
}

read_processed_at() {
  python3 -c '
import json
import sys

value = json.load(sys.stdin).get("processed_at")
print("" if value is None else value)
'
}

if [[ $# -ne 0 ]]; then
  usage
  exit 1
fi

for required_command in curl python3; do
  require_command "$required_command"
done

if [[ ! "$PORT_VALUE" =~ ^[1-9][0-9]*$ ]]; then
  fail "PORT must be a positive integer, got: $PORT_VALUE"
fi

case "$BASE_URL" in
  http://*|https://*) ;;
  *) fail "BASE_URL must start with http:// or https://, got: $BASE_URL" ;;
esac

printf '[deploy-smoke] waiting for health base_url=%s\n' "$BASE_URL"
for attempt in $(seq 1 80); do
  if health_response="$(curl -fsS "$BASE_URL/health" 2>/dev/null)"; then
    LAST_HEALTH_RESPONSE="$health_response"
    health_status="$(printf '%s' "$health_response" | json_field status || true)"
    worker_liveness="$(printf '%s' "$health_response" | json_field worker.liveness || true)"
    recovery_active="$(printf '%s' "$health_response" | json_field worker.recovery_active || true)"
    printf '[deploy-smoke] health poll=%s status=%s liveness=%s recovery_active=%s\n' \
      "$attempt" "${health_status:-missing}" "${worker_liveness:-missing}" "${recovery_active:-missing}"
    if [[ "$health_status" == "ok" && "$worker_liveness" == "healthy" && "$recovery_active" == "false" ]]; then
      printf '[deploy-smoke] health ready body=%s\n' "$health_response"
      break
    fi
  fi
  sleep 0.25
  if [[ "$attempt" == "80" ]]; then
    fail "/health never became ready at $BASE_URL; last_body=${LAST_HEALTH_RESPONSE:-unavailable}"
  fi
done

payload='{"kind":"deploy-smoke","attempt":1,"source":"deploy-smoke.sh"}'
printf '[deploy-smoke] creating job via POST %s/jobs\n' "$BASE_URL"
create_response="$(curl -fsS -X POST "$BASE_URL/jobs" -H 'content-type: application/json' -d "$payload")"
printf '[deploy-smoke] created job body=%s\n' "$create_response"
JOB_ID="$(printf '%s' "$create_response" | json_field id)"

if [[ -z "$JOB_ID" ]]; then
  fail "created job response did not include id"
fi

printf '[deploy-smoke] polling job id=%s\n' "$JOB_ID"
for attempt in $(seq 1 80); do
  LAST_RESPONSE="$(curl -fsS "$BASE_URL/jobs/$JOB_ID")"
  job_status="$(printf '%s' "$LAST_RESPONSE" | json_field status)"
  processed_at="$(printf '%s' "$LAST_RESPONSE" | read_processed_at)"
  printf '[deploy-smoke] poll=%s status=%s processed_at=%s\n' "$attempt" "$job_status" "${processed_at:-null}"
  if [[ "$job_status" == "processed" && -n "$processed_at" ]]; then
    attempts="$(printf '%s' "$LAST_RESPONSE" | json_field attempts)"
    printf '[deploy-smoke] processed job id=%s attempts=%s\n' "$JOB_ID" "$attempts"
    printf '%s\n' "$LAST_RESPONSE"
    exit 0
  fi
  sleep 0.25
done

fail "job $JOB_ID never reached processed state"
