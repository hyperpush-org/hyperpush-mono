#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PORT="${PORT:-18080}"
JOB_POLL_MS="${JOB_POLL_MS:-500}"
BASE_URL="http://127.0.0.1:${PORT}"
LOG_FILE="$(mktemp -t reference-backend-smoke.XXXXXX.log)"
SERVER_PID=""
LAST_RESPONSE=""

: "${DATABASE_URL:?set DATABASE_URL}"

cleanup() {
  local status=$?
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  if [[ $status -ne 0 ]]; then
    echo "[smoke] failure; tailing server log from $LOG_FILE" >&2
    tail -n 200 "$LOG_FILE" >&2 || true
  else
    rm -f "$LOG_FILE"
  fi
}
trap cleanup EXIT

json_field() {
  local field="$1"
  python3 -c '
import json
import sys

field = sys.argv[1]
data = json.load(sys.stdin)
value = data.get(field)
if value is None:
    sys.exit(1)
if isinstance(value, (dict, list)):
    print(json.dumps(value, separators=(",", ":")))
else:
    print(value)
' "$field"
}

if [[ "$(psql "$DATABASE_URL" -Atqc "SELECT to_regclass('public.jobs') IS NOT NULL")" != "t" ]]; then
  echo "[smoke] jobs table is missing; run: cargo run -p meshc -- migrate reference-backend up" >&2
  exit 1
fi

echo "[smoke] building reference-backend"
(
  cd "$ROOT"
  cargo run -p meshc -- build reference-backend
)

echo "[smoke] starting reference-backend on :$PORT"
(
  cd "$ROOT"
  PORT="$PORT" JOB_POLL_MS="$JOB_POLL_MS" DATABASE_URL="$DATABASE_URL" ./reference-backend/reference-backend >"$LOG_FILE" 2>&1
) &
SERVER_PID=$!

for attempt in $(seq 1 80); do
  if health_response="$(curl -fsS "$BASE_URL/health" 2>/dev/null)"; then
    echo "[smoke] health ready: $health_response"
    break
  fi
  sleep 0.25
  if [[ "$attempt" == "80" ]]; then
    echo "[smoke] reference-backend never became healthy on $BASE_URL" >&2
    exit 1
  fi
done

create_response="$(curl -fsS -X POST "$BASE_URL/jobs" -H 'content-type: application/json' -d '{"kind":"demo","attempt":1,"source":"smoke"}')"
echo "[smoke] created job: $create_response"
JOB_ID="$(printf '%s' "$create_response" | json_field id)"

for attempt in $(seq 1 80); do
  LAST_RESPONSE="$(curl -fsS "$BASE_URL/jobs/$JOB_ID")"
  job_status="$(printf '%s' "$LAST_RESPONSE" | json_field status)"
  processed_at="$(printf '%s' "$LAST_RESPONSE" | python3 -c '
import json
import sys

value = json.load(sys.stdin).get("processed_at")
print("" if value is None else value)
')"
  echo "[smoke] poll $attempt status=$job_status processed_at=${processed_at:-null}"
  if [[ "$job_status" == "processed" && -n "$processed_at" ]]; then
    attempts="$(printf '%s' "$LAST_RESPONSE" | json_field attempts)"
    echo "[smoke] processed job after attempts=$attempts"
    echo "$LAST_RESPONSE"
    exit 0
  fi
  sleep 0.25
done

echo "[smoke] job $JOB_ID never reached processed state" >&2
echo "[smoke] final response: ${LAST_RESPONSE:-<none>}" >&2
exit 1
