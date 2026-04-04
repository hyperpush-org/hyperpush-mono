#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

usage() {
  cat <<'EOF'
Usage:
  bash scripts/verify-whitespace.sh --staged [--fix]
  bash scripts/verify-whitespace.sh --diff-range <git-range>

Modes:
  --staged            Check the currently staged diff.
  --fix               When used with --staged, trim trailing spaces/tabs from fully
                      staged text files, re-add them to the index, then re-check.
  --diff-range RANGE  Check a committed diff range (for CI), e.g. base...HEAD.

Notes:
  - The fixer refuses to rewrite partially staged files because that would destroy
    the user's staged hunk boundaries.
  - The guard uses git's own whitespace checker, so the final verdict matches the
    commit/apply semantics that GitHub will see.
EOF
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "verify-whitespace: missing required command: $command_name" >&2
    exit 1
  fi
}

mode=""
fix="false"
diff_range=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --staged)
      if [[ -n "$mode" ]]; then
        echo "verify-whitespace: choose either --staged or --diff-range" >&2
        exit 1
      fi
      mode="staged"
      shift
      ;;
    --fix)
      fix="true"
      shift
      ;;
    --diff-range)
      if [[ -n "$mode" ]]; then
        echo "verify-whitespace: choose either --staged or --diff-range" >&2
        exit 1
      fi
      mode="diff-range"
      if [[ $# -lt 2 ]]; then
        echo "verify-whitespace: --diff-range requires a git range argument" >&2
        exit 1
      fi
      diff_range="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "verify-whitespace: unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

require_command git
require_command python3

if [[ -z "$mode" ]]; then
  echo "verify-whitespace: choose --staged or --diff-range" >&2
  usage >&2
  exit 1
fi

if [[ "$mode" != "staged" && "$fix" == "true" ]]; then
  echo "verify-whitespace: --fix only applies to --staged" >&2
  exit 1
fi

ensure_inside_git_repo() {
  if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
    echo "verify-whitespace: must run inside a git worktree" >&2
    exit 1
  fi
}

trim_staged_trailing_whitespace() {
  python3 - <<'PY'
from pathlib import Path
import subprocess
import sys


def paths_from_git(command):
    data = subprocess.check_output(command)
    return [entry.decode() for entry in data.split(b"\0") if entry]

staged = paths_from_git([
    "git", "diff", "--cached", "--name-only", "--diff-filter=ACMR", "-z"
])
unstaged = set(paths_from_git([
    "git", "diff", "--name-only", "--diff-filter=ACMR", "-z"
]))

partially_staged = [path for path in staged if path in unstaged]
if partially_staged:
    print(
        "verify-whitespace: refusing to auto-fix partially staged files; stage or stash the remaining hunks first:",
        file=sys.stderr,
    )
    for path in partially_staged:
        print(f"  - {path}", file=sys.stderr)
    raise SystemExit(1)

fixed = []
for rel in staged:
    path = Path(rel)
    if not path.is_file() or path.is_symlink():
        continue
    raw = path.read_bytes()
    if b"\0" in raw:
        continue
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        continue

    lines = text.splitlines(keepends=True)
    changed = False
    cleaned_lines = []
    for line in lines:
        newline = ""
        body = line
        if line.endswith("\r\n"):
            newline = "\r\n"
            body = line[:-2]
        elif line.endswith("\n"):
            newline = "\n"
            body = line[:-1]
        elif line.endswith("\r"):
            newline = "\r"
            body = line[:-1]
        cleaned = body.rstrip(" \t")
        if cleaned != body:
            changed = True
        cleaned_lines.append(cleaned + newline)

    cleaned_text = "".join(cleaned_lines)
    if changed and cleaned_text != text:
        path.write_text(cleaned_text)
        subprocess.check_call(["git", "add", "--", rel])
        fixed.append(rel)

if fixed:
    print("verify-whitespace: trimmed trailing whitespace in:")
    for rel in fixed:
        print(f"  - {rel}")
PY
}

run_staged_check() {
  local check_output

  if [[ "$fix" == "true" ]]; then
    trim_staged_trailing_whitespace
  fi

  if ! check_output="$(git diff --check --cached -- 2>&1)"; then
    printf '%s\n' "$check_output" >&2
    echo "verify-whitespace: staged diff still contains whitespace errors" >&2
    exit 1
  fi

  echo "verify-whitespace: staged diff is clean"
}

run_diff_range_check() {
  local check_output
  if ! check_output="$(git diff --check "$diff_range" -- 2>&1)"; then
    printf '%s\n' "$check_output" >&2
    echo "verify-whitespace: committed diff range contains whitespace errors: $diff_range" >&2
    exit 1
  fi

  echo "verify-whitespace: committed diff range is clean: $diff_range"
}

ensure_inside_git_repo

case "$mode" in
  staged)
    run_staged_check
    ;;
  diff-range)
    run_diff_range_check
    ;;
  *)
    echo "verify-whitespace: unreachable mode: $mode" >&2
    exit 1
    ;;
esac
