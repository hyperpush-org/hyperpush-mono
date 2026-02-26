#!/usr/bin/env bash
# Benchmark runner: Mesh vs Go vs Rust vs Elixir HTTP throughput
# Usage: bash benchmarks/run_benchmarks.sh
# Requirements: wrk, go, cargo, mix, meshc (or meshc skipped with warning)
#
# Port assignments:
#   Mesh:   3000
#   Go:     3001
#   Rust:   3002
#   Elixir: 3003

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Track all server PIDs for cleanup
SERVER_PIDS=()

cleanup() {
  echo ""
  echo "Cleaning up server processes..."
  for pid in "${SERVER_PIDS[@]:-}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
}

trap cleanup EXIT

# -------------------------------------------------------------------
# wait_for_server: poll until HTTP 200 or timeout
# -------------------------------------------------------------------
wait_for_server() {
  local port=$1
  local max_wait=30
  local elapsed=0
  until curl -sf "http://localhost:$port/text" > /dev/null 2>&1; do
    sleep 1
    elapsed=$((elapsed + 1))
    if [ "$elapsed" -ge "$max_wait" ]; then
      echo "Server on port $port failed to start within ${max_wait}s" >&2
      return 1
    fi
  done
}

# -------------------------------------------------------------------
# measure_peak_rss: sample peak RSS of a PID in KB; print maximum
# -------------------------------------------------------------------
measure_peak_rss() {
  local pid=$1
  local peak=0
  while kill -0 "$pid" 2>/dev/null; do
    local rss
    # macOS / BSD: ps -o rss=
    rss=$(ps -o rss= -p "$pid" 2>/dev/null | tr -d ' ') || rss=""
    # Linux fallback: /proc/$pid/status VmRSS
    if [ -z "$rss" ] && [ -r "/proc/$pid/status" ]; then
      rss=$(grep -m1 'VmRSS:' "/proc/$pid/status" 2>/dev/null | awk '{print $2}') || rss=""
    fi
    if [ -n "$rss" ] && [ "$rss" -gt "$peak" ] 2>/dev/null; then
      peak=$rss
    fi
    sleep 0.5
  done
  echo "$peak"
}

# -------------------------------------------------------------------
# awk_parse_rps: extract Requests/sec value from wrk output
# -------------------------------------------------------------------
awk_parse_rps() {
  grep 'Requests/sec' "$1" | awk '{print $2}' | tr -d ','
}

# -------------------------------------------------------------------
# awk_parse_latency: extract latency percentile (50th or 99th)
# Input: file, "50.000%" or "99.000%"
# -------------------------------------------------------------------
awk_parse_latency() {
  local file=$1
  local pct=$2
  # wrk --latency output: "     50.000%    1.23ms"
  grep -m1 "${pct}" "$file" | awk '{print $2}' 2>/dev/null || echo "N/A"
}

# -------------------------------------------------------------------
# kb_to_mb: convert KB integer to MB with 1 decimal place
# -------------------------------------------------------------------
kb_to_mb() {
  local kb=$1
  if [ -z "$kb" ] || [ "$kb" -eq 0 ] 2>/dev/null; then
    echo "N/A"
    return
  fi
  awk "BEGIN { printf \"%.1f MB\", $kb/1024 }"
}

# -------------------------------------------------------------------
# run_wrk: 10s warmup + 3 x 30s timed runs; record avg rps, p50, p99
# Also samples peak RSS during each timed run
# Args: port endpoint server_pid result_prefix
# -------------------------------------------------------------------
run_wrk() {
  local port=$1
  local endpoint=$2
  local server_pid=$3
  local prefix=$4
  local threads
  threads=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 4)
  local url="http://localhost:${port}/${endpoint}"

  echo "  Warmup (10s): ${url}"
  wrk -t"${threads}" -c100 -d10s "${url}" > /dev/null 2>&1 || true

  local sum_rps=0
  local sum_p50_ms=0
  local sum_p99_ms=0
  local peak_rss=0
  local count=0

  for i in 1 2 3; do
    local tmp_out
    tmp_out=$(mktemp /tmp/wrk_out_XXXXXX.txt)

    # Start peak RSS sampler in background
    measure_peak_rss "$server_pid" > "${tmp_out}.rss" &
    local rss_pid=$!

    echo "  Run ${i}/3 (30s): ${url}"
    wrk -t"${threads}" -c100 -d30s --latency "${url}" > "${tmp_out}" 2>&1 || true

    # Signal RSS sampler to stop by waiting naturally (server still running)
    # We stop by killing the sampler after wrk finishes
    kill "$rss_pid" 2>/dev/null || true
    wait "$rss_pid" 2>/dev/null || true

    # Parse results
    local rps p50 p99
    rps=$(awk_parse_rps "${tmp_out}")
    p50=$(awk_parse_latency "${tmp_out}" "50.000%")
    p99=$(awk_parse_latency "${tmp_out}" "99.000%")

    # Parse RSS (KB)
    local rss_kb
    rss_kb=$(cat "${tmp_out}.rss" 2>/dev/null | tr -d ' \n') || rss_kb=0
    rss_kb=${rss_kb:-0}
    if [ -n "$rss_kb" ] && [ "$rss_kb" -gt "$peak_rss" ] 2>/dev/null; then
      peak_rss=$rss_kb
    fi

    # Accumulate for averaging (convert latency to numeric ms)
    if [ -n "$rps" ]; then
      sum_rps=$(awk "BEGIN { printf \"%.2f\", $sum_rps + $rps }")
      count=$((count + 1))
    fi

    rm -f "${tmp_out}" "${tmp_out}.rss"
  done

  # Average
  local avg_rps avg_p50 avg_p99
  if [ "$count" -gt 0 ]; then
    avg_rps=$(awk "BEGIN { printf \"%.0f\", $sum_rps / $count }")
  else
    avg_rps="N/A"
  fi

  # Store results in files (simple key=value for later parsing)
  echo "$avg_rps"       > "${prefix}_rps.txt"
  echo "$peak_rss"      > "${prefix}_rss_kb.txt"
  # Capture p50/p99 from final run (representative; averaging latency strings is complex)
  echo "${p50:-N/A}"    > "${prefix}_p50.txt"
  echo "${p99:-N/A}"    > "${prefix}_p99.txt"
}

# -------------------------------------------------------------------
# read_result: read a value from result file
# -------------------------------------------------------------------
read_result() {
  local file=$1
  cat "$file" 2>/dev/null | tr -d ' \n' || echo "N/A"
}

# -------------------------------------------------------------------
# format_rps: add thousands separator
# -------------------------------------------------------------------
format_rps() {
  local rps=$1
  if [ "$rps" = "N/A" ]; then echo "N/A"; return; fi
  printf "%'.0f" "$rps" 2>/dev/null || echo "$rps"
}

# -------------------------------------------------------------------
# RESULTS directory for temp files
# -------------------------------------------------------------------
TMP_DIR=$(mktemp -d /tmp/bench_XXXXXX)
trap "rm -rf ${TMP_DIR}; cleanup" EXIT

# -------------------------------------------------------------------
# Track which languages are available
# -------------------------------------------------------------------
MESH_AVAILABLE=false
GO_AVAILABLE=false
RUST_AVAILABLE=false
ELIXIR_AVAILABLE=false

MESH_PID=""
GO_PID=""
RUST_PID=""
ELIXIR_PID=""

echo "=== Mesh HTTP Benchmark Runner ==="
echo "Starting servers..."
echo ""

# ---
# Mesh: try meshc
# ---
if command -v meshc > /dev/null 2>&1; then
  echo "[Mesh] Starting on port 3000..."
  meshc run "${SCRIPT_DIR}/mesh/bench.mpl" > "${TMP_DIR}/mesh.log" 2>&1 &
  MESH_PID=$!
  SERVER_PIDS+=("$MESH_PID")
  if wait_for_server 3000; then
    echo "[Mesh] Ready on port 3000 (PID ${MESH_PID})"
    MESH_AVAILABLE=true
  else
    echo "[Mesh] WARNING: server did not start within 30s — skipping"
    MESH_AVAILABLE=false
  fi
else
  echo "[Mesh] WARNING: meshc not in PATH — skipping Mesh benchmark"
fi

# ---
# Go
# ---
if command -v go > /dev/null 2>&1; then
  echo "[Go] Building and starting on port 3001..."
  (cd "${SCRIPT_DIR}/go" && go run . > "${TMP_DIR}/go.log" 2>&1) &
  GO_PID=$!
  SERVER_PIDS+=("$GO_PID")
  if wait_for_server 3001; then
    echo "[Go] Ready on port 3001 (PID ${GO_PID})"
    GO_AVAILABLE=true
  else
    echo "[Go] WARNING: server did not start within 30s — skipping"
    GO_AVAILABLE=false
  fi
else
  echo "[Go] WARNING: go not in PATH — skipping Go benchmark"
fi

# ---
# Rust
# ---
if command -v cargo > /dev/null 2>&1; then
  echo "[Rust] Building (cargo build --release) and starting on port 3002..."
  (cd "${SCRIPT_DIR}/rust" && cargo build --release -q && ./target/release/bench > "${TMP_DIR}/rust.log" 2>&1) &
  RUST_PID=$!
  SERVER_PIDS+=("$RUST_PID")
  if wait_for_server 3002; then
    echo "[Rust] Ready on port 3002 (PID ${RUST_PID})"
    RUST_AVAILABLE=true
  else
    echo "[Rust] WARNING: server did not start within 30s — skipping"
    RUST_AVAILABLE=false
  fi
else
  echo "[Rust] WARNING: cargo not in PATH — skipping Rust benchmark"
fi

# ---
# Elixir
# ---
if command -v mix > /dev/null 2>&1; then
  echo "[Elixir] Fetching deps and starting on port 3003..."
  (cd "${SCRIPT_DIR}/elixir" && mix deps.get -q 2>/dev/null && mix run --no-halt > "${TMP_DIR}/elixir.log" 2>&1) &
  ELIXIR_PID=$!
  SERVER_PIDS+=("$ELIXIR_PID")
  if wait_for_server 3003; then
    echo "[Elixir] Ready on port 3003 (PID ${ELIXIR_PID})"
    ELIXIR_AVAILABLE=true
  else
    echo "[Elixir] WARNING: server did not start within 30s — skipping"
    ELIXIR_AVAILABLE=false
  fi
else
  echo "[Elixir] WARNING: mix not in PATH — skipping Elixir benchmark"
fi

# Check that wrk is available
if ! command -v wrk > /dev/null 2>&1; then
  echo ""
  echo "ERROR: wrk is not installed. Install it with: brew install wrk (macOS) or apt install wrk (Ubuntu)"
  exit 1
fi

echo ""
echo "=== Running benchmarks (100 connections, 10s warmup + 30s x3) ==="
echo ""

# -------------------------------------------------------------------
# Run benchmarks for each available server x each endpoint
# -------------------------------------------------------------------
for endpoint in text json; do
  echo "-- Endpoint: /${endpoint} --"

  if [ "$MESH_AVAILABLE" = "true" ] && [ -n "$MESH_PID" ]; then
    echo "[Mesh /${endpoint}]"
    run_wrk 3000 "$endpoint" "$MESH_PID" "${TMP_DIR}/mesh_${endpoint}"
  fi

  if [ "$GO_AVAILABLE" = "true" ] && [ -n "$GO_PID" ]; then
    echo "[Go /${endpoint}]"
    run_wrk 3001 "$endpoint" "$GO_PID" "${TMP_DIR}/go_${endpoint}"
  fi

  if [ "$RUST_AVAILABLE" = "true" ] && [ -n "$RUST_PID" ]; then
    echo "[Rust /${endpoint}]"
    run_wrk 3002 "$endpoint" "$RUST_PID" "${TMP_DIR}/rust_${endpoint}"
  fi

  if [ "$ELIXIR_AVAILABLE" = "true" ] && [ -n "$ELIXIR_PID" ]; then
    echo "[Elixir /${endpoint}]"
    run_wrk 3003 "$endpoint" "$ELIXIR_PID" "${TMP_DIR}/elixir_${endpoint}"
  fi

  echo ""
done

# -------------------------------------------------------------------
# Print summary table
# -------------------------------------------------------------------
echo "== Benchmark Results (100 connections, 30s x3 averaged) =="
echo ""

for endpoint in text json; do
  echo "Endpoint: /${endpoint}"
  printf "%-12s %-12s %-10s %-10s %-12s\n" "Language" "Req/s" "p50" "p99" "Peak RSS"
  printf '%0.s-' {1..58}; echo ""

  for lang in Mesh Go Rust Elixir; do
    lower=$(echo "$lang" | tr '[:upper:]' '[:lower:]')
    available_var="${lang^^}_AVAILABLE"
    available="${!available_var}"

    if [ "$available" = "true" ]; then
      rps=$(read_result "${TMP_DIR}/${lower}_${endpoint}_rps.txt")
      p50=$(read_result "${TMP_DIR}/${lower}_${endpoint}_p50.txt")
      p99=$(read_result "${TMP_DIR}/${lower}_${endpoint}_p99.txt")
      rss_kb=$(read_result "${TMP_DIR}/${lower}_${endpoint}_rss_kb.txt")
      rss_mb=$(kb_to_mb "$rss_kb")
      rps_fmt=$(format_rps "$rps")
      printf "%-12s %-12s %-10s %-10s %-12s\n" "$lang" "$rps_fmt" "$p50" "$p99" "$rss_mb"
    else
      printf "%-12s %-12s %-10s %-10s %-12s\n" "$lang" "N/A" "N/A" "N/A" "N/A"
    fi
  done
  echo ""
done

echo "Full results: benchmarks/RESULTS.md"
