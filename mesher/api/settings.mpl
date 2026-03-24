# HTTP route handlers for project settings and storage visibility.

# Provides CRUD for retention_days and sample_rate, plus storage usage.

# Wires RETAIN-01 (retention settings), RETAIN-03 (storage visibility).

from Ingestion.Pipeline import PipelineRegistry

from Storage.Queries import get_project_settings, update_project_settings, get_project_storage

from Api.Helpers import require_param, get_registry, resolve_project_id

# --- Helper functions (defined before handlers) ---

# Helper: format settings row to JSON response.

fn settings_row_to_json(rows) do
  if List.length(rows) > 0 do
    let row = List.head(rows)
    let retention_days_opt = String.to_int(Map.get(row, "retention_days"))
    let retention_days = case retention_days_opt do
      Some( n) -> n
      None -> 0
    end
    let sample_rate_opt = String.to_float(Map.get(row, "sample_rate"))
    let sample_rate = case sample_rate_opt do
      Some( f) -> f
      None -> 1.0
    end
    HTTP.response(200, json { retention_days : retention_days, sample_rate : sample_rate })
  else
    HTTP.response(404, json { error : "project not found" })
  end
end

# Helper: format storage row to JSON response.

fn storage_row_to_json(rows) do
  if List.length(rows) > 0 do
    let row = List.head(rows)
    let event_count_opt = String.to_int(Map.get(row, "event_count"))
    let event_count = case event_count_opt do
      Some( n) -> n
      None -> 0
    end
    let estimated_bytes_opt = String.to_int(Map.get(row, "estimated_bytes"))
    let estimated_bytes = case estimated_bytes_opt do
      Some( n) -> n
      None -> 0
    end
    HTTP.response(200, json { event_count : event_count, estimated_bytes : estimated_bytes })
  else
    HTTP.response(404, json { error : "project not found" })
  end
end

# --- Handler functions (pub, defined after all helpers) ---

# Handle GET /api/v1/projects/:project_id/settings (RETAIN-01)

# Returns retention_days and sample_rate for a project.

pub fn handle_get_project_settings(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let result = get_project_settings(pool, project_id)
  case result do
    Ok( rows) -> settings_row_to_json(rows)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Handle POST /api/v1/projects/:project_id/settings (RETAIN-01)

# Updates retention_days and/or sample_rate from JSON body.

pub fn handle_update_project_settings(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let body = Request.body(request)
  let result = update_project_settings(pool, project_id, body)
  case result do
    Ok( n) -> HTTP.response(200, json { status : "ok", affected : n })
    Err( e) -> HTTP.response(400, json { error : e })
  end
end

# Handle GET /api/v1/projects/:project_id/storage (RETAIN-03)

# Returns event_count and estimated_bytes for a project.

pub fn handle_get_project_storage(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let result = get_project_storage(pool, project_id)
  case result do
    Ok( rows) -> storage_row_to_json(rows)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end
