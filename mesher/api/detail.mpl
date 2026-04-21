# Event detail and navigation HTTP handlers for Mesher REST API.
# Provides full event payload including all JSONB fields (exception,
# stacktrace, breadcrumbs, tags, extra, user_context) and next/previous
# event navigation within an issue.
# JSONB fields are embedded RAW in JSON response (not double-quoted).

from Ingestion.Pipeline import PipelineRegistry
from Storage.Queries import get_event_detail, get_event_neighbors
from Types.Event import Event
from Api.Helpers import require_param, get_registry

# --- Helper functions (leaf first, per define-before-use requirement) ---
# Normalize the event row through the typed Event surface so caller-visible
# string fields stay stable even when direct Map.get stringification drifts.

fn detail_row_to_event(row) -> Event do
  Event {
    id : Map.get(row, "id"),
    project_id : Map.get(row, "project_id"),
    issue_id : Map.get(row, "issue_id"),
    level : Map.get(row, "level"),
    message : Map.get(row, "message"),
    fingerprint : Map.get(row, "fingerprint"),
    exception : Map.get(row, "exception"),
    stacktrace : Map.get(row, "stacktrace"),
    breadcrumbs : Map.get(row, "breadcrumbs"),
    tags : Map.get(row, "tags"),
    extra : Map.get(row, "extra"),
    user_context : Map.get(row, "user_context"),
    sdk_name : Map.get(row, "sdk_name"),
    sdk_version : Map.get(row, "sdk_version"),
    environment : Map.get(row, "environment"),
    session_id : Map.get(row, "session_id"),
    received_at : Map.get(row, "received_at")
  }
end

# Serialize complete event detail row to JSON.
# String fields get \" quoting. JSONB fields embedded raw (no quoting).
# JSONB: exception, stacktrace, breadcrumbs, tags, extra, user_context

fn event_detail_to_json(row) -> String do
  let normalized = Json.encode(detail_row_to_event(row))
  let id = Json.get(normalized, "id")
  let project_id = Json.get(normalized, "project_id")
  let issue_id = Json.get(normalized, "issue_id")
  let level = Json.get(normalized, "level")
  let message = Json.get(normalized, "message")
  let fingerprint = Json.get(normalized, "fingerprint")
  let exception = Map.get(row, "exception")
  let stacktrace = Map.get(row, "stacktrace")
  let breadcrumbs = Map.get(row, "breadcrumbs")
  let tags = Map.get(row, "tags")
  let extra = Map.get(row, "extra")
  let user_context = Map.get(row, "user_context")
  let sdk_name = Json.get(normalized, "sdk_name")
  let sdk_version = Json.get(normalized, "sdk_version")
  let environment = Json.get(normalized, "environment")
  let session_id = Json.get(normalized, "session_id")
  let received_at = Json.get(normalized, "received_at")
  """{"id":"#{id}","project_id":"#{project_id}","issue_id":"#{issue_id}","level":"#{level}","message":"#{message}","fingerprint":"#{fingerprint}","exception":#{exception},"stacktrace":#{stacktrace},"breadcrumbs":#{breadcrumbs},"tags":#{tags},"extra":#{extra},"user_context":#{user_context},"sdk_name":"#{sdk_name}","sdk_version":"#{sdk_version}","environment":"#{environment}","session_id":"#{session_id}","received_at":"#{received_at}"}"""
end

# Format a nullable neighbor ID for JSON output.
# Empty string -> null (no quotes), non-empty -> quoted string.

fn format_neighbor_id(val :: String) -> String do
  if String.length(val) == 0 do
    "null"
  else
    "\"#{val}\""
  end
end

# Serialize navigation row to JSON with next_id and prev_id.

fn neighbors_to_json(row) -> String do
  let next_id = Map.get(row, "next_id")
  let prev_id = Map.get(row, "prev_id")
  let next_str = format_neighbor_id(next_id)
  let prev_str = format_neighbor_id(prev_id)
  """{"next_id":#{next_str},"prev_id":#{prev_str}}"""
end

# Combine event detail JSON with navigation JSON into final response.

fn build_detail_response(detail_json :: String, nav_json :: String) -> String do
  """{"event":#{detail_json},"navigation":#{nav_json}}"""
end

fn empty_navigation_json() -> String do
  """{"next_id":null,"prev_id":null}"""
end

# Helper: build response from navigation rows.

fn build_nav_response(detail_json :: String, nav_rows) do
  if List.length(nav_rows) > 0 do
    let nav_row = List.head(nav_rows)
    let nav_json = neighbors_to_json(nav_row)
    HTTP.response(200, build_detail_response(detail_json, nav_json))
  else
    HTTP.response(200, build_detail_response(detail_json, empty_navigation_json()))
  end
end

# Helper: add navigation data to event detail and build final response.
# Makes the second query (get_event_neighbors) using issue_id and received_at
# from the detail row, then combines both into the response.

fn add_navigation(pool,
event_id :: String,
issue_id :: String,
received_at :: String,
detail_json :: String) do
  let nav_result = get_event_neighbors(pool, issue_id, received_at, event_id)
  case nav_result do
    Ok( nav_rows) -> build_nav_response(detail_json, nav_rows)
    Err -> HTTP.response(200, build_detail_response(detail_json, empty_navigation_json()))
  end
end

# Helper: process event detail rows into response with navigation.
# Extracts the single row, serializes it, then fetches navigation data.

fn build_event_response_from_rows(pool, event_id :: String, rows) do
  if List.length(rows) > 0 do
    let row = List.head(rows)
    let normalized = Json.encode(detail_row_to_event(row))
    let detail_json = event_detail_to_json(row)
    let issue_id = Json.get(normalized, "issue_id")
    let received_at = Json.get(normalized, "received_at")
    add_navigation(pool, event_id, issue_id, received_at, detail_json)
  else
    HTTP.response(404, json { error : "event not found" })
  end
end

# --- Handler functions (pub, defined after all helpers) ---
# Handle GET /api/v1/events/:event_id
# Returns full event payload with all JSONB fields and next/prev navigation.
# Makes two sequential queries: event detail, then event neighbors.

pub fn handle_event_detail(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let event_id = require_param(request, "event_id")
  let result = get_event_detail(pool, event_id)
  case result do
    Ok( rows) -> build_event_response_from_rows(pool, event_id, rows)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end
