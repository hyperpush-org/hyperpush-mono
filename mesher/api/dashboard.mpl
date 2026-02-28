# Dashboard aggregation HTTP handlers for Mesher REST API.
# Provides event volume, error breakdown, top issues, tag analysis,
# issue timeline, and project health summary endpoints.
# All handlers follow the PipelineRegistry pattern for pool lookup.

from Ingestion.Pipeline import PipelineRegistry
from Storage.Queries import event_volume_hourly, error_breakdown_by_level, top_issues_by_frequency, event_breakdown_by_tag, issue_event_timeline, project_health_summary
from Api.Helpers import query_or_default, to_json_array, require_param, get_registry, resolve_project_id

# --- Shared helpers (leaf functions first, per define-before-use requirement) ---

# Serialize {bucket, count} row to JSON. count is numeric (no quotes).
fn bucket_to_json(row) -> String do
  let bucket = Map.get(row, "bucket")
  let count = Map.get(row, "count")
  "{\"bucket\":\"" <> bucket <> "\",\"count\":" <> count <> "}"
end

# Serialize {level, count} row to JSON. count is numeric (no quotes).
fn level_to_json(row) -> String do
  let level = Map.get(row, "level")
  let count = Map.get(row, "count")
  "{\"level\":\"" <> level <> "\",\"count\":" <> count <> "}"
end

# Serialize top issue row to JSON. event_count is numeric.
fn top_issue_to_json(row) -> String do
  let id = Map.get(row, "id")
  let title = Map.get(row, "title")
  let level = Map.get(row, "level")
  let status = Map.get(row, "status")
  let event_count = Map.get(row, "event_count")
  let last_seen = Map.get(row, "last_seen")
  "{\"id\":\"" <> id <> "\",\"title\":\"" <> title <> "\",\"level\":\"" <> level <> "\",\"status\":\"" <> status <> "\",\"event_count\":" <> event_count <> ",\"last_seen\":\"" <> last_seen <> "\"}"
end

# Serialize {tag_value, count} row to JSON. count is numeric.
# tag_value may be empty if COALESCE returns empty string for null tags.
fn tag_entry_to_json(row) -> String do
  let tag_value = Map.get(row, "tag_value")
  let count = Map.get(row, "count")
  let value_str = if String.length(tag_value) == 0 do "null" else "\"" <> tag_value <> "\"" end
  "{\"value\":" <> value_str <> ",\"count\":" <> count <> "}"
end

# Serialize timeline event row to JSON.
fn timeline_event_to_json(row) -> String do
  let id = Map.get(row, "id")
  let level = Map.get(row, "level")
  let message = Map.get(row, "message")
  let received_at = Map.get(row, "received_at")
  "{\"id\":\"" <> id <> "\",\"level\":\"" <> level <> "\",\"message\":\"" <> message <> "\",\"received_at\":\"" <> received_at <> "\"}"
end


# --- Handler functions (pub, defined after all helpers) ---

# Helper: serialize volume rows and respond.
fn respond_volume(rows) do
  let body = rows |> List.map(fn(row) do bucket_to_json(row) end) |> to_json_array()
  HTTP.response(200, body)
end

# Handle GET /api/v1/projects/:project_id/dashboard/volume
# Returns event volume bucketed by hour or day.
pub fn handle_event_volume(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let bucket = query_or_default(request, "bucket", "hour")
  let result = event_volume_hourly(pool, project_id, bucket)
  case result do
    Ok(rows) -> respond_volume(rows)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Helper: serialize level breakdown rows and respond.
fn respond_levels(rows) do
  let body = rows |> List.map(fn(row) do level_to_json(row) end) |> to_json_array()
  HTTP.response(200, body)
end

# Handle GET /api/v1/projects/:project_id/dashboard/levels
# Returns error breakdown by severity level.
pub fn handle_error_breakdown(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let result = error_breakdown_by_level(pool, project_id)
  case result do
    Ok(rows) -> respond_levels(rows)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Helper: serialize top issues rows and respond.
fn respond_top_issues(rows) do
  let body = rows |> List.map(fn(row) do top_issue_to_json(row) end) |> to_json_array()
  HTTP.response(200, body)
end

# Handle GET /api/v1/projects/:project_id/dashboard/top-issues
# Returns top issues ranked by frequency.
pub fn handle_top_issues(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let limit = query_or_default(request, "limit", "10")
  let result = top_issues_by_frequency(pool, project_id, limit)
  case result do
    Ok(rows) -> respond_top_issues(rows)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Helper: serialize tag breakdown rows and respond.
fn respond_tag_breakdown(rows) do
  let body = rows |> List.map(fn(row) do tag_entry_to_json(row) end) |> to_json_array()
  HTTP.response(200, body)
end

# Helper: perform tag breakdown query and respond.
fn do_tag_breakdown(pool, project_id :: String, key :: String) do
  let result = event_breakdown_by_tag(pool, project_id, key)
  case result do
    Ok(rows) -> respond_tag_breakdown(rows)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Handle GET /api/v1/projects/:project_id/dashboard/tags?key=...
# Returns event breakdown by tag value for the specified tag key.
pub fn handle_tag_breakdown(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let key = query_or_default(request, "key", "")
  if String.length(key) == 0 do
    HTTP.response(400, json { error: "missing key parameter" })
  else
    do_tag_breakdown(pool, project_id, key)
  end
end

# Helper: serialize timeline event rows and respond.
fn respond_timeline(rows) do
  let body = rows |> List.map(fn(row) do timeline_event_to_json(row) end) |> to_json_array()
  HTTP.response(200, body)
end

# Handle GET /api/v1/issues/:issue_id/timeline
# Returns per-issue event timeline.
pub fn handle_issue_timeline(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let issue_id = require_param(request, "issue_id")
  let limit = query_or_default(request, "limit", "50")
  let result = issue_event_timeline(pool, issue_id, limit)
  case result do
    Ok(rows) -> respond_timeline(rows)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Helper: serialize health summary from single row.
fn respond_health(rows) do
  if List.length(rows) > 0 do
    let row = List.head(rows)
    let unresolved = Map.get(row, "unresolved_count")
    let events_24h = Map.get(row, "events_24h")
    let new_today = Map.get(row, "new_today")
    HTTP.response(200, "{\"unresolved_count\":" <> unresolved <> ",\"events_24h\":" <> events_24h <> ",\"new_today\":" <> new_today <> "}")
  else
    HTTP.response(404, json { error: "project not found" })
  end
end

# Handle GET /api/v1/projects/:project_id/dashboard/health
# Returns project health summary: unresolved count, 24h events, new today.
pub fn handle_project_health(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let result = project_health_summary(pool, project_id)
  case result do
    Ok(rows) -> respond_health(rows)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end
