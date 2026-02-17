# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-17)

**Core value:** Expressive, readable concurrency -- writing concurrent programs should feel as natural and clean as writing sequential code, with the safety net of supervision and fault tolerance built into the language.
**Current focus:** v11.0 Query Builder -- Phase 106 (Advanced WHERE Operators and Raw SQL Fragments)

## Current Position

Phase: 106 of 114 (Advanced WHERE Operators and Raw SQL Fragments)
Plan: 0 of TBD in current phase
Status: Ready to plan
Last activity: 2026-02-17 -- Roadmap created for v11.0 (9 phases, 32 requirements)

Progress: [░░░░░░░░░░] 0% (v11.0)

## Performance Metrics

**All-time Totals:**
- Plans completed: 311
- Phases completed: 105
- Milestones shipped: 20 (v1.0-v10.1)
- Lines of Rust: ~98,850
- Lines of website: ~5,500
- Lines of Mesh: ~4,020
- Timeline: 12 days (2026-02-05 -> 2026-02-17)

| Phase | Plan | Duration | Tasks | Files |
|-------|------|----------|-------|-------|
| 104   | 01   | 12min    | 2     | 3     |
| 105   | 01   | 18min    | 3     | 1     |
| 105   | 02   | 8min     | 3     | 4     |
| 105.1 | 02   | 9min     | 1     | 1     |
| 105.1 | 01   | 17min    | 2     | 5     |
| 105.1 | 03   | 9min     | 2     | 2     |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Phase 105.1: Pass MIR return type to codegen_service_call_helper for type-aware reply conversion
- Phase 105.1: Construction-side fix only for struct-in-Result: existing codegen_leaf deref logic handles destructuring
- Phase 105.1: Auth workaround reverted -- authenticate_request returns Project!String directly
- Phase 105.1: EventProcessor service call SIGSEGV persists -- needs dedicated investigation

### Roadmap Evolution

- v11.0 roadmap created: 9 phases (106-114), 32 requirements mapped

### Pending Todos

None.

### Blockers/Concerns

- Event ingestion (POST /api/v1/events) crashes during background EventProcessor service call after HTTP response is sent. Requires deeper investigation of EventProcessor service loop state or call dispatch.

## Session Continuity

Last session: 2026-02-17
Stopped at: v11.0 roadmap created, ready to plan Phase 106
Resume file: None
Next action: `/gsd:plan-phase 106`
