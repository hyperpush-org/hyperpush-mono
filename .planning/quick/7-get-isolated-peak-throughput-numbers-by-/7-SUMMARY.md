---
phase: quick-7
plan: 01
subsystem: benchmarks
tags: [benchmarks, fly-io, performance, bash, scripting]
dependency_graph:
  requires: []
  provides: [isolated-benchmark-scripts, isolated-results-tables, isolated-methodology-docs]
  affects: [benchmarks/fly/README.md, benchmarks/METHODOLOGY.md, benchmarks/RESULTS.md]
tech_stack:
  added: []
  patterns: [fly-machine-lifecycle, isolated-server-benchmarking]
key_files:
  created:
    - benchmarks/fly/start-server-isolated.sh
    - benchmarks/fly/run-benchmarks-isolated.sh
  modified:
    - benchmarks/METHODOLOGY.md
    - benchmarks/RESULTS.md
    - benchmarks/fly/README.md
decisions:
  - "start-server-isolated.sh uses a single SERVER_PID variable (not a PIDS array) since only one server runs at a time"
  - "run-benchmarks-isolated.sh uses bench-isolated-server as the fixed machine name and destroys any leftover from a prior run before creating a new one"
  - "SERVER_HOST uses Fly.io internal DNS (bench-isolated-server.vm.$APP.internal) to avoid IPv6 bracket notation issues"
  - "Machine ID extraction uses grep on fly machine run output since flyctl output format varies; falls back to machine list lookup"
metrics:
  duration: "~3 minutes"
  completed: "2026-02-26"
  tasks_completed: 2
  files_changed: 5
---

# Phase quick-7 Plan 01: Isolated Peak Throughput Benchmark Scripts Summary

Isolated benchmark mode for measuring true per-language peak throughput: two new scripts (start-server-isolated.sh + run-benchmarks-isolated.sh) plus updated METHODOLOGY.md, RESULTS.md, and README.md.

## What Was Built

### Task 1: Create isolated server entrypoint and load-gen orchestrator

**benchmarks/fly/start-server-isolated.sh** — runs on the server VM, starts exactly one language server based on `LANG` env var (Mesh/Go/Rust/Elixir). Validates `LANG` and exits 1 on unknown values. Uses same startup commands as `start-servers.sh` (same ports, same working dirs). Polls readiness via curl, prints `SERVER_READY` to stdout when ready, then logs RSS every 2s as `RSS,<Lang>,<epoch>,<kB>`. Handles EXIT trap to kill server process on exit.

**benchmarks/fly/run-benchmarks-isolated.sh** — runs on the load-gen VM, orchestrates the full isolated benchmark loop. For each language (Mesh/Go/Rust/Elixir):
1. Destroys any existing `bench-isolated-server` machine (leftover cleanup)
2. Launches a fresh `performance-2x` machine with `LANG=<language>` and `start-server-isolated.sh` as entrypoint
3. Polls fly logs for `SERVER_READY` signal (120s timeout)
4. Polls HTTP endpoint for reachability using internal DNS (`bench-isolated-server.vm.$APP.internal`)
5. Runs hey warmup + 5 timed runs using same parameters as `run-benchmarks.sh`
6. Collects peak RSS from machine logs
7. Stops and destroys the machine before next language
8. Prints per-language result immediately, then summary table at end

### Task 2: Update METHODOLOGY.md and RESULTS.md with isolated benchmark documentation

**benchmarks/METHODOLOGY.md** — new `## Isolated Peak Throughput` section after `## Caveats`: describes per-machine isolation procedure and provides copy-paste run instructions (docker build + fly push + script invocation).

**benchmarks/RESULTS.md** — new `## Isolated Peak Throughput Results` section with blank `/text` and `/json` tables (Run 1 excl., Runs 2–5 avg, p50, p99, Peak RSS) ready to be populated after a run. Includes comparison table with co-located numbers pre-filled.

**benchmarks/fly/README.md** — new `## Isolated Peak Throughput Run` section before `## Important Notes` with step-by-step instructions (4 steps) covering image build, script invocation, what the script does internally, and timing note (~40 minutes total).

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check

- [x] benchmarks/fly/start-server-isolated.sh created and executable
- [x] benchmarks/fly/run-benchmarks-isolated.sh created and executable
- [x] Both scripts pass `bash -n` syntax check
- [x] METHODOLOGY.md has "Isolated Peak Throughput" section
- [x] RESULTS.md has isolated results tables + comparison table
- [x] README.md references run-benchmarks-isolated.sh
- [x] Commits: d1828554 (Task 1), 30cc9dc8 (Task 2)
