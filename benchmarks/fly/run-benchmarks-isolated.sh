#!/usr/bin/env bash
set -e

APP="${APP:-bench-mesh}"
REGION="${REGION:-ord}"
SERVER_IMAGE="${SERVER_IMAGE:-registry.fly.io/bench-mesh/servers:latest}"

CONNECTIONS=100
WARMUP_DURATION=30
BENCH_DURATION=30
RUNS=5
DISCARD_RUNS=1   # first N timed runs excluded from average (allow JIT/code-cache warmup)

echo "=== Mesh HTTP Isolated Benchmark Runner ==="
echo "App:         $APP"
echo "Region:      $REGION"
echo "Image:       $SERVER_IMAGE"
echo "Connections: $CONNECTIONS"
echo "Duration:    ${WARMUP_DURATION}s warmup + ${BENCH_DURATION}s x${RUNS} timed runs (first ${DISCARD_RUNS} discarded)"
echo ""

declare -A RESULT_RPS
declare -A RESULT_P50
declare -A RESULT_P99
declare -A RESULT_RSS

declare -A PORTS
PORTS[Mesh]=3000
PORTS[Go]=3001
PORTS[Rust]=3002
PORTS[Elixir]=3003

run_bench() {
  local url=$1
  local duration=$2
  bombardier -c "$CONNECTIONS" -d "${duration}s" --timeout=30s -l "$url" 2>/dev/null
}

for lang in Mesh Go Rust Elixir; do
  port="${PORTS[$lang]}"
  machine_name="bench-isolated-server"

  echo ""
  echo "=== Isolated benchmark: $lang ==="

  # Destroy any existing machine with this name from a previous (possibly failed) run
  existing_id=$(fly machine list --app "$APP" --json 2>/dev/null \
    | grep -o '"id":"[^"]*"' | grep -A1 "\"name\":\"$machine_name\"" | head -1 | cut -d'"' -f4 || true)
  if [ -z "$existing_id" ]; then
    # Try a more portable approach: list machines and find by name
    existing_id=$(fly machine list --app "$APP" 2>/dev/null \
      | awk -v name="$machine_name" '$0 ~ name {print $1}' | head -1 || true)
  fi
  if [ -n "$existing_id" ]; then
    echo "Found existing machine $existing_id ($machine_name) — destroying before reuse..."
    fly machine stop "$existing_id" --app "$APP" 2>/dev/null || true
    fly machine destroy "$existing_id" --app "$APP" --force 2>/dev/null || true
    sleep 3
  fi

  # Launch a fresh server machine for this language only.
  # start-server-isolated.sh is baked into the server image under /app/benchmarks/fly/.
  # Use /bin/bash as entrypoint to avoid relying on the file's execute bit.
  echo "Launching server machine for $lang..."
  if ! machine_output=$(fly machine run "$SERVER_IMAGE" \
    /app/benchmarks/fly/start-server-isolated.sh \
    --app "$APP" \
    --name "$machine_name" \
    --vm-size performance-2x \
    --region "$REGION" \
    --env "LANG=$lang" \
    --entrypoint /bin/bash \
    2>&1); then
    echo "ERROR: fly machine run failed for $lang:"
    echo "$machine_output"
    exit 1
  fi
  echo "$machine_output"

  # Extract machine ID from output — parse "Machine ID: <id>" line directly
  # (regex on raw output would match image IDs first, giving the wrong result)
  machine_id=$(echo "$machine_output" | grep "Machine ID:" | awk '{print $NF}' | head -1 || true)

  # Extract IPv6 address from fly machine run output ("connect via the following private ip")
  # Internal DNS (*.vm.app.internal) has propagation delays inside Fly machines — use direct IPv6 instead.
  server_ipv6=$(echo "$machine_output" | grep -A1 "private ip" | grep -oE 'fdaa:[0-9a-f:]+' | head -1 || true)
  if [ -n "$server_ipv6" ]; then
    SERVER_HOST="[$server_ipv6]"
  else
    # Fallback to internal DNS if IPv6 extraction failed
    SERVER_HOST="${machine_name}.vm.${APP}.internal"
  fi

  echo "Machine ID: ${machine_id:-unknown}"
  echo "Server host: $SERVER_HOST"

  # Wait for HTTP reachability
  # (Using direct IPv6 address to avoid internal DNS propagation delays inside Fly machines)
  echo "Polling $SERVER_HOST:$port for HTTP reachability..."
  http_ready=false
  for attempt in $(seq 1 90); do
    if curl -sf -6 "http://$SERVER_HOST:$port/text" > /dev/null 2>&1; then
      http_ready=true
      echo "$lang server is reachable at $SERVER_HOST:$port"
      break
    fi
    sleep 2
  done

  if [ "$http_ready" != "true" ]; then
    echo "WARNING: $lang server not reachable after 180s — skipping" >&2
    for ep in text json; do
      RESULT_RPS["${lang}_${ep}"]="N/A"
      RESULT_P50["${lang}_${ep}"]="N/A"
      RESULT_P99["${lang}_${ep}"]="N/A"
    done
    RESULT_RSS["$lang"]="N/A"
    if [ -n "$machine_id" ]; then
      fly machine stop "$machine_id" --app "$APP" 2>/dev/null || true
      fly machine destroy "$machine_id" --app "$APP" --force 2>/dev/null || true
    fi
    continue
  fi

  base_url="http://$SERVER_HOST:$port"

  for endpoint in text json; do
    url="$base_url/$endpoint"
    echo ""
    echo "  Endpoint: /$endpoint"

    # Warmup run (results discarded)
    run_bench "$url" "$WARMUP_DURATION" > /dev/null
    echo "  Warmup done. Running ${RUNS} timed runs..."

    rps_total=0
    counted=0
    last_p50="N/A"
    last_p99="N/A"

    for i in $(seq 1 $RUNS); do
      output=$(run_bench "$url" "$BENCH_DURATION")
      rps=$(echo "$output" | awk '/Reqs\/sec/{print $2}')
      p50=$(echo "$output" | awk '/^ *50%/{print $2}')
      p99=$(echo "$output" | awk '/^ *99%/{print $2}')
      if [ "$i" -le "$DISCARD_RUNS" ]; then
        echo "    Run $i: ${rps:-N/A} req/s  p50=${p50:-N/A}  p99=${p99:-N/A}  [warmup — excluded]"
      else
        rps_total=$(echo "$rps_total + ${rps:-0}" | bc -l)
        counted=$((counted + 1))
        last_p50="${p50:-N/A}"
        last_p99="${p99:-N/A}"
        echo "    Run $i: ${rps:-N/A} req/s  p50=${p50:-N/A}  p99=${p99:-N/A}"
      fi
    done

    avg_rps=$(echo "scale=0; $rps_total / $counted" | bc -l)
    RESULT_RPS["${lang}_${endpoint}"]="$avg_rps"
    RESULT_P50["${lang}_${endpoint}"]="$last_p50"
    RESULT_P99["${lang}_${endpoint}"]="$last_p99"
  done

  # Collect peak RSS from machine logs
  if [ -n "$machine_id" ]; then
    peak_rss=$(fly logs --machine "$machine_id" --app "$APP" --no-tail 2>/dev/null \
      | grep '^RSS,' \
      | awk -F, '{print $4}' \
      | sort -n \
      | tail -1 || true)
    if [ -n "$peak_rss" ] && [ "$peak_rss" -gt 0 ] 2>/dev/null; then
      peak_rss_mb=$(echo "scale=1; $peak_rss / 1024" | bc -l)
      RESULT_RSS["$lang"]="${peak_rss_mb} MB"
    else
      RESULT_RSS["$lang"]="N/A"
    fi
  else
    RESULT_RSS["$lang"]="N/A"
  fi

  echo ""
  echo "  $lang result: /text=${RESULT_RPS[${lang}_text]:-N/A} req/s  /json=${RESULT_RPS[${lang}_json]:-N/A} req/s  Peak RSS=${RESULT_RSS[$lang]:-N/A}"

  # Stop and destroy the server machine before moving to the next language
  echo "  Stopping and destroying machine ${machine_id:-$machine_name}..."
  if [ -n "$machine_id" ]; then
    fly machine stop "$machine_id" --app "$APP" 2>/dev/null || true
    fly machine destroy "$machine_id" --app "$APP" --force 2>/dev/null || true
  else
    stop_id=$(fly machine list --app "$APP" 2>/dev/null \
      | awk -v name="$machine_name" '$0 ~ name {print $1}' | head -1 || true)
    if [ -n "$stop_id" ]; then
      fly machine stop "$stop_id" --app "$APP" 2>/dev/null || true
      fly machine destroy "$stop_id" --app "$APP" --force 2>/dev/null || true
    fi
  fi
  echo "  Machine destroyed. Moving to next language."
  sleep 5
done

echo ""
echo "============================================================"
echo "== ISOLATED RESULTS (${CONNECTIONS} connections, ${BENCH_DURATION}s x$((RUNS - DISCARD_RUNS)) averaged, run 1 excluded) =="
echo "============================================================"
echo ""
printf "%-10s  %-12s  %-10s  %-10s  %-12s\n" "Language" "Req/s" "p50" "p99" "Peak RSS"
printf "%-10s  %-12s  %-10s  %-10s  %-12s\n" "--------" "-----" "---" "---" "--------"
echo ""
echo "/text endpoint:"
for lang in Mesh Go Rust Elixir; do
  printf "  %-10s  %-12s  %-10s  %-10s  %-12s\n" \
    "$lang" \
    "${RESULT_RPS[${lang}_text]:-N/A}" \
    "${RESULT_P50[${lang}_text]:-N/A}" \
    "${RESULT_P99[${lang}_text]:-N/A}" \
    "${RESULT_RSS[$lang]:-N/A}"
done

echo ""
echo "/json endpoint:"
for lang in Mesh Go Rust Elixir; do
  printf "  %-10s  %-12s  %-10s  %-10s  %-12s\n" \
    "$lang" \
    "${RESULT_RPS[${lang}_json]:-N/A}" \
    "${RESULT_P50[${lang}_json]:-N/A}" \
    "${RESULT_P99[${lang}_json]:-N/A}" \
    "${RESULT_RSS[$lang]:-N/A}"
done

echo ""
echo "============================================================"
