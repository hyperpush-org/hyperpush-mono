#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

PROOF_PAGE="website/docs/docs/production-backend-proof/index.md"
DISTRIBUTED_PAGE="website/docs/docs/distributed/index.md"
DISTRIBUTED_PROOF_PAGE="website/docs/docs/distributed-proof/index.md"
WEB_PAGE="website/docs/docs/web/index.md"
DATABASES_PAGE="website/docs/docs/databases/index.md"
TESTING_PAGE="website/docs/docs/testing/index.md"
CONCURRENCY_PAGE="website/docs/docs/concurrency/index.md"
SIDEBAR_FILE="website/docs/.vitepress/config.mts"
MESHER_RUNBOOK_FILE="mesher/README.md"
MESHER_VERIFIER_FILE="scripts/verify-m051-s01.sh"
RETAINED_VERIFIER_FILE="scripts/verify-m051-s02.sh"

PROOF_LINK="/docs/production-backend-proof/"
PRODUCTION_BACKEND_PROOF_LINK='[Production Backend Proof](/docs/production-backend-proof/)'
CLUSTERED_EXAMPLE_LINK='[Clustered Example](/docs/getting-started/clustered-example/)'
MESHER_RUNBOOK_LINK='[`mesher/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/mesher/README.md)'
MESHER_VERIFIER_COMMAND='bash scripts/verify-m051-s01.sh'
RETAINED_VERIFIER_COMMAND='bash scripts/verify-m051-s02.sh'
PROOF_SURFACE_VERIFIER_COMMAND='bash scripts/verify-production-proof-surface.sh'
STALE_RUNBOOK_LINK='[`reference-backend/README.md`](https://github.com/snowdamiz/mesh-lang/blob/main/reference-backend/README.md)'
STALE_FIXTURE_PATH='scripts/fixtures/backend/reference-backend/'
PROOF_ROLE_SENTENCE="This is the compact public-secondary handoff for Mesh's backend proof story."
PROOF_AUDIENCE_SENTENCE='Public readers should still stay scaffold/examples first:'
PROOF_BOUNDARY_SENTENCE='This page only names the deeper maintainer surfaces behind that public story: Mesher as the maintained app, and a retained backend-only verifier kept behind a named replay instead of a public repo-root runbook.'
DISTRIBUTED_PROOF_ROLE_SENTENCE='This is the only public-secondary docs page that carries the named clustered verifier rails.'
GENERIC_GUIDE_LINKS=(
  '/docs/web/'
  '/docs/databases/'
  '/docs/testing/'
  '/docs/concurrency/'
  '/docs/tooling/'
)
GENERIC_GUIDES=(
  "$WEB_PAGE"
  "$DATABASES_PAGE"
  "$TESTING_PAGE"
  "$CONCURRENCY_PAGE"
)
PROOF_DIRECT_RAIL_MARKERS=(
  'bash scripts/verify-m047-s04.sh'
  'bash scripts/verify-m047-s05.sh'
  'cargo test -p meshc --test e2e_m047_s07 -- --nocapture'
  'bash scripts/verify-m047-s06.sh'
  'bash scripts/verify-m043-s04-fly.sh --help'
)
STALE_TESTING_EXAMPLES=(
  'meshc test reference-backend'
  'meshc test reference-backend/tests'
  'meshc test reference-backend/tests/config.test.mpl'
  'meshc test --coverage reference-backend'
)

phase() {
  printf '[proof-docs] %s\n' "$*"
}

fail() {
  printf '[proof-docs] ERROR: %s\n' "$*" >&2
  exit 1
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    fail "required command missing from PATH: $command_name"
  fi
}

require_file() {
  local relative_path="$1"
  if [[ ! -f "$ROOT/$relative_path" ]]; then
    fail "missing file: $relative_path"
  fi
}

require_contains() {
  local relative_path="$1"
  local needle="$2"
  local description="$3"
  if ! rg -Fq "$needle" "$ROOT/$relative_path"; then
    fail "$relative_path missing ${description}: $needle"
  fi
}

require_not_contains() {
  local relative_path="$1"
  local needle="$2"
  local description="$3"
  if rg -Fq "$needle" "$ROOT/$relative_path"; then
    fail "$relative_path still contains ${description}: $needle"
  fi
}

require_order() {
  local relative_path="$1"
  local first="$2"
  local second="$3"
  local description="$4"
  local output
  if ! output=$(python3 - "$ROOT/$relative_path" "$first" "$second" "$description" 2>&1 <<'PY'
from pathlib import Path
import sys

path = Path(sys.argv[1])
first = sys.argv[2]
second = sys.argv[3]
description = sys.argv[4]
text = path.read_text(errors='replace')
first_index = text.find(first)
if first_index == -1:
    raise SystemExit(f"{description}: missing first marker {first!r} in {path}")
second_index = text.find(second)
if second_index == -1:
    raise SystemExit(f"{description}: missing second marker {second!r} in {path}")
if first_index >= second_index:
    raise SystemExit(
        f"{description}: expected {first!r} before {second!r} in {path}, got indexes {first_index} >= {second_index}"
    )
print(f"{description}: {first!r} precedes {second!r}")
PY
); then
    fail "$output"
  fi
}

phase "checking prerequisites"
require_command rg
require_command python3

phase "checking canonical files exist"
for relative_path in \
  "$PROOF_PAGE" \
  "$DISTRIBUTED_PAGE" \
  "$DISTRIBUTED_PROOF_PAGE" \
  "$WEB_PAGE" \
  "$DATABASES_PAGE" \
  "$TESTING_PAGE" \
  "$CONCURRENCY_PAGE" \
  "$SIDEBAR_FILE" \
  "$MESHER_RUNBOOK_FILE" \
  "$MESHER_VERIFIER_FILE" \
  "$RETAINED_VERIFIER_FILE"; do
  require_file "$relative_path"
done

phase "checking the sidebar keeps the production proof surface public but secondary"
if ! python3 - "$ROOT/$SIDEBAR_FILE" "$PROOF_LINK" <<'PY'
from pathlib import Path
import re
import sys

config_path = Path(sys.argv[1])
proof_link = sys.argv[2]
text = config_path.read_text(errors='replace')
sidebar_block_match = re.search(
    r"sidebar:\s*{\s*'/docs/': \[(?P<block>[\s\S]*?)\n\s*\],\s*\n\s*},\s*\n\s*outline:",
    text,
    re.MULTILINE,
)
if not sidebar_block_match:
    raise SystemExit('unable to locate /docs/ sidebar block')
sidebar_block = sidebar_block_match.group('block')

group_pattern = re.compile(
    r"{\s*text:\s*'(?P<name>[^']+)'[\s\S]*?items:\s*\[(?P<items>[\s\S]*?)\]\s*,\s*}",
    re.MULTILINE,
)
groups = {match.group('name'): match.group('items') for match in group_pattern.finditer(sidebar_block)}

for required_group in ['Getting Started', 'Reference', 'Proof Surfaces']:
    if required_group not in groups:
        raise SystemExit(f'missing sidebar group: {required_group}')

proof_items = groups['Proof Surfaces']
proof_match = re.search(
    r"text:\s*'Production Backend Proof'[\s\S]*?link:\s*'(?P<link>[^']+)'(?P<tail>[\s\S]*?)}\s*as any",
    proof_items,
    re.MULTILINE,
)
if not proof_match:
    raise SystemExit('missing Production Backend Proof item inside Proof Surfaces')
if proof_match.group('link') != proof_link:
    raise SystemExit(
        f'production proof link drifted: expected {proof_link!r}, got {proof_match.group("link")!r}'
    )
if 'includeInFooter: false' not in proof_match.group('tail'):
    raise SystemExit('Production Backend Proof is no longer opted out of the footer chain')
if proof_link in groups['Getting Started']:
    raise SystemExit('Production Backend Proof drifted back into the Getting Started group')
if sidebar_block.count(proof_link) != 1:
    raise SystemExit(
        f'expected the production proof link exactly once in the /docs/ sidebar, found {sidebar_block.count(proof_link)} copies'
    )

reference_index = sidebar_block.find("text: 'Reference'")
proof_surfaces_index = sidebar_block.find("text: 'Proof Surfaces'")
if reference_index == -1 or proof_surfaces_index == -1:
    raise SystemExit('missing Reference or Proof Surfaces group ordering markers')
if proof_surfaces_index <= reference_index:
    raise SystemExit('Proof Surfaces no longer stays after Reference in the public docs graph')

print('proof-sidebar-secondary: ok')
PY
then
  fail "sidebar no longer keeps Production Backend Proof public-secondary"
fi

phase "checking the proof page opts out of the footer chain"
require_contains "$PROOF_PAGE" 'prev: false' 'proof-page footer prev opt-out marker'
require_contains "$PROOF_PAGE" 'next: false' 'proof-page footer next opt-out marker'

phase "checking the proof page markers and maintainer commands"
for needle in \
  "$PROOF_ROLE_SENTENCE" \
  "$PROOF_AUDIENCE_SENTENCE" \
  "$PROOF_BOUNDARY_SENTENCE" \
  '## Canonical surfaces' \
  '## Named maintainer verifiers' \
  '## Retained backend-only recovery signals' \
  '## When to use this page vs the generic guides' \
  '## Failure inspection map' \
  "$CLUSTERED_EXAMPLE_LINK" \
  "$MESHER_RUNBOOK_LINK" \
  "$MESHER_VERIFIER_COMMAND" \
  "$RETAINED_VERIFIER_COMMAND" \
  "$PROOF_SURFACE_VERIFIER_COMMAND" \
  'restart_count' \
  'last_exit_reason' \
  'recovered_jobs' \
  'last_recovery_at' \
  'last_recovery_job_id' \
  'last_recovery_count' \
  'recovery_active'; do
  require_contains "$PROOF_PAGE" "$needle" 'proof-page marker'
done
for needle in "${GENERIC_GUIDE_LINKS[@]}"; do
  require_contains "$PROOF_PAGE" "$needle" 'generic-guide routing marker'
done
for needle in \
  "$STALE_RUNBOOK_LINK" \
  "$STALE_FIXTURE_PATH"; do
  require_not_contains "$PROOF_PAGE" "$needle" 'stale public backend handoff'
done
require_order "$PROOF_PAGE" '## Canonical surfaces' '## Named maintainer verifiers' 'proof page keeps surfaces ahead of named maintainer verifiers'
require_order "$PROOF_PAGE" '## Named maintainer verifiers' '## Retained backend-only recovery signals' 'proof page keeps named maintainer verifiers ahead of retained recovery signals'
require_order "$PROOF_PAGE" '## Retained backend-only recovery signals' '## When to use this page vs the generic guides' 'proof page keeps retained recovery signals ahead of generic-guide handoff'
require_order "$PROOF_PAGE" '## When to use this page vs the generic guides' '## Failure inspection map' 'proof page keeps generic-guide handoff ahead of the failure map'
require_order "$PROOF_PAGE" "$CLUSTERED_EXAMPLE_LINK" "$MESHER_RUNBOOK_LINK" 'proof page keeps the clustered starter handoff ahead of the Mesher runbook link'
require_order "$PROOF_PAGE" "$MESHER_RUNBOOK_LINK" "$MESHER_VERIFIER_COMMAND" 'proof page keeps Mesher runbook ahead of Mesher verifier command'
require_order "$PROOF_PAGE" "$MESHER_VERIFIER_COMMAND" "$RETAINED_VERIFIER_COMMAND" 'proof page keeps Mesher verifier ahead of retained verifier command'

phase "checking generic guide handoffs"
for guide in "${GENERIC_GUIDES[@]}"; do
  require_contains "$guide" '> **Production backend proof:**' 'guide proof-callout marker'
  require_contains "$guide" "$PRODUCTION_BACKEND_PROOF_LINK" 'Production Backend Proof handoff'
  require_contains "$guide" "$MESHER_RUNBOOK_LINK" 'Mesher maintainer handoff'
  require_contains "$guide" "$MESHER_VERIFIER_COMMAND" 'Mesher verifier handoff'
  require_contains "$guide" "$RETAINED_VERIFIER_COMMAND" 'retained verifier handoff'
  require_not_contains "$guide" "$STALE_RUNBOOK_LINK" 'stale repo-root backend link'
  require_not_contains "$guide" "$STALE_FIXTURE_PATH" 'leaked retained fixture path'
  require_order "$guide" "$PRODUCTION_BACKEND_PROOF_LINK" "$MESHER_RUNBOOK_LINK" 'generic guide keeps Production Backend Proof ahead of Mesher'
  require_order "$guide" "$MESHER_RUNBOOK_LINK" "$MESHER_VERIFIER_COMMAND" 'generic guide keeps Mesher link ahead of Mesher verifier'
  require_order "$guide" "$MESHER_VERIFIER_COMMAND" "$RETAINED_VERIFIER_COMMAND" 'generic guide keeps Mesher verifier ahead of retained verifier'
done
for needle in "${STALE_TESTING_EXAMPLES[@]}"; do
  require_not_contains "$TESTING_PAGE" "$needle" 'stale testing example'
done

phase "checking distributed guide and distributed-proof handoffs"
for needle in \
  '> **Clustered proof surfaces:**' \
  "$CLUSTERED_EXAMPLE_LINK" \
  "$PRODUCTION_BACKEND_PROOF_LINK" \
  "$MESHER_RUNBOOK_LINK" \
  "$MESHER_VERIFIER_COMMAND" \
  "$RETAINED_VERIFIER_COMMAND"; do
  require_contains "$DISTRIBUTED_PAGE" "$needle" 'distributed guide proof marker'
done
require_not_contains "$DISTRIBUTED_PAGE" "$STALE_RUNBOOK_LINK" 'stale repo-root backend link on Distributed Actors'
require_not_contains "$DISTRIBUTED_PAGE" "$STALE_FIXTURE_PATH" 'leaked retained fixture path on Distributed Actors'
for needle in "${PROOF_DIRECT_RAIL_MARKERS[@]}"; do
  require_not_contains "$DISTRIBUTED_PAGE" "$needle" 'named clustered verifier rail on Distributed Actors'
done
require_order "$DISTRIBUTED_PAGE" "$CLUSTERED_EXAMPLE_LINK" "$PRODUCTION_BACKEND_PROOF_LINK" 'Distributed Actors keeps Clustered Example ahead of Production Backend Proof'
require_order "$DISTRIBUTED_PAGE" "$PRODUCTION_BACKEND_PROOF_LINK" "$MESHER_RUNBOOK_LINK" 'Distributed Actors keeps Production Backend Proof ahead of Mesher'
require_order "$DISTRIBUTED_PAGE" "$MESHER_RUNBOOK_LINK" "$MESHER_VERIFIER_COMMAND" 'Distributed Actors keeps Mesher link ahead of Mesher verifier'
require_order "$DISTRIBUTED_PAGE" "$MESHER_VERIFIER_COMMAND" "$RETAINED_VERIFIER_COMMAND" 'Distributed Actors keeps Mesher verifier ahead of retained verifier'

for needle in \
  "$DISTRIBUTED_PROOF_ROLE_SENTENCE" \
  '## Public surfaces and verifier rails' \
  '## Named proof commands' \
  "$CLUSTERED_EXAMPLE_LINK" \
  "$PRODUCTION_BACKEND_PROOF_LINK" \
  "$MESHER_RUNBOOK_LINK" \
  "$MESHER_VERIFIER_COMMAND" \
  "$RETAINED_VERIFIER_COMMAND"; do
  require_contains "$DISTRIBUTED_PROOF_PAGE" "$needle" 'distributed-proof marker'
done
require_not_contains "$DISTRIBUTED_PROOF_PAGE" "$STALE_RUNBOOK_LINK" 'stale repo-root backend link on Distributed Proof'
require_not_contains "$DISTRIBUTED_PROOF_PAGE" "$STALE_FIXTURE_PATH" 'leaked retained fixture path on Distributed Proof'
require_order "$DISTRIBUTED_PROOF_PAGE" "$DISTRIBUTED_PROOF_ROLE_SENTENCE" '## Public surfaces and verifier rails' 'Distributed Proof keeps the role sentence ahead of the verifier ledger'
require_order "$DISTRIBUTED_PROOF_PAGE" "$PRODUCTION_BACKEND_PROOF_LINK" "$MESHER_RUNBOOK_LINK" 'Distributed Proof keeps Production Backend Proof ahead of Mesher'
require_order "$DISTRIBUTED_PROOF_PAGE" "$MESHER_RUNBOOK_LINK" "$MESHER_VERIFIER_COMMAND" 'Distributed Proof keeps Mesher link ahead of Mesher verifier'
require_order "$DISTRIBUTED_PROOF_PAGE" "$MESHER_VERIFIER_COMMAND" "$RETAINED_VERIFIER_COMMAND" 'Distributed Proof keeps Mesher verifier ahead of retained verifier'
require_order "$DISTRIBUTED_PROOF_PAGE" '## Public surfaces and verifier rails' '## Named proof commands' 'Distributed Proof keeps the verifier ledger ahead of named commands'

phase "production proof surface verified"
