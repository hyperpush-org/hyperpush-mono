#!/usr/bin/env bash
set -e

REPO_ROOT="/app"
PIDS=()

cleanup() {
  echo "Stopping all servers..."
  for pid in "${PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
  done
}
trap cleanup EXIT INT TERM

wait_for_server() {
  local port=$1
  local name=$2
  local max_wait=60
  local elapsed=0
  echo "Waiting for $name on port $port..."
  until curl -sf "http://localhost:$port/text" > /dev/null 2>&1; do
    sleep 1
    elapsed=$((elapsed + 1))
    if [ $elapsed -ge $max_wait ]; then
      echo "ERROR: $name on port $port failed to start within ${max_wait}s" >&2
      return 1
    fi
  done
  echo "$name ready on port $port"
}

# Start Mesh server (port 3000)
if command -v meshc &> /dev/null; then
  meshc run "$REPO_ROOT/benchmarks/mesh/bench.mpl" &
  MESH_PID=$!
  PIDS+=($MESH_PID)
  wait_for_server 3000 "Mesh" && echo "MESH_PID=$MESH_PID"
else
  echo "WARNING: meshc not found, skipping Mesh server" >&2
  MESH_PID=""
fi

# Start Go server (port 3001)
cd "$REPO_ROOT/benchmarks/go" && go run . &
GO_PID=$!
PIDS+=($GO_PID)
wait_for_server 3001 "Go"
echo "GO_PID=$GO_PID"

# Start Rust server (port 3002)
"$REPO_ROOT/benchmarks/rust/target/release/bench" &
RUST_PID=$!
PIDS+=($RUST_PID)
wait_for_server 3002 "Rust"
echo "RUST_PID=$RUST_PID"

# Start Elixir server (port 3003)
cd "$REPO_ROOT/benchmarks/elixir" && MIX_ENV=prod mix run --no-halt &
ELIXIR_PID=$!
PIDS+=($ELIXIR_PID)
wait_for_server 3003 "Elixir"
echo "ELIXIR_PID=$ELIXIR_PID"

echo ""
echo "=== All servers running ==="
echo "Mesh:   http://localhost:3000 (PID: ${MESH_PID:-N/A})"
echo "Go:     http://localhost:3001 (PID: $GO_PID)"
echo "Rust:   http://localhost:3002 (PID: $RUST_PID)"
echo "Elixir: http://localhost:3003 (PID: $ELIXIR_PID)"
echo ""
echo "Ready for benchmark run — sampling RSS every 2s to stdout"
echo ""

# Periodically sample peak RSS of all server processes and log to stdout
# Load gen VM can retrieve this via: fly logs --machine <id> | grep '^RSS,'
declare -A PEAK_RSS
PEAK_RSS[Mesh]=0
PEAK_RSS[Go]=0
PEAK_RSS[Rust]=0
PEAK_RSS[Elixir]=0

declare -A LANG_PIDS
LANG_PIDS[Mesh]="$MESH_PID"
LANG_PIDS[Go]="$GO_PID"
LANG_PIDS[Rust]="$RUST_PID"
LANG_PIDS[Elixir]="$ELIXIR_PID"

while true; do
  for lang in Mesh Go Rust Elixir; do
    pid="${LANG_PIDS[$lang]}"
    if [ -z "$pid" ]; then continue; fi
    rss=$(grep VmRSS /proc/$pid/status 2>/dev/null | awk '{print $2}')
    if [ -n "$rss" ] && [ "$rss" -gt "${PEAK_RSS[$lang]}" ] 2>/dev/null; then
      PEAK_RSS[$lang]=$rss
    fi
    echo "RSS,$lang,$(date +%s),$rss"
  done
  sleep 2
done
