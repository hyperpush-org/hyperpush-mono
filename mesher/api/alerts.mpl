# HTTP route handlers for alert rule management and alert state management.
# Alert rules define conditions for automated notifications (ALERT-01).
# Fired alerts have a lifecycle: active -> acknowledged -> resolved (ALERT-06).

from Ingestion.Pipeline import PipelineRegistry
from Storage.Queries import create_alert_rule, list_alert_rules, toggle_alert_rule, delete_alert_rule, list_alerts, acknowledge_alert, resolve_fired_alert
from Api.Helpers import require_param, query_or_default, to_json_array, get_registry, resolve_project_id

# --- Helper functions (defined before handlers) ---

# Format nullable timestamp: empty string -> JSON null, otherwise quoted string.
fn format_nullable_ts(ts :: String) -> String do
  if String.length(ts) > 0 do
    "\"" <> ts <> "\""
  else
    "null"
  end
end

# Serialize a single alert rule Map row to JSON string.
fn rule_row_to_json(row) -> String do
  "{\"id\":\"" <> Map.get(row, "id") <> "\",\"project_id\":\"" <> Map.get(row, "project_id") <> "\",\"name\":\"" <> Map.get(row, "name") <> "\",\"condition\":" <> Map.get(row, "condition_json") <> ",\"action\":" <> Map.get(row, "action_json") <> ",\"enabled\":" <> Map.get(row, "enabled") <> ",\"cooldown_minutes\":" <> Map.get(row, "cooldown_minutes") <> ",\"last_fired_at\":" <> format_nullable_ts(Map.get(row, "last_fired_at")) <> ",\"created_at\":\"" <> Map.get(row, "created_at") <> "\"}"
end

# Serialize a single alert Map row to JSON string.
fn alert_row_to_json(row) -> String do
  "{\"id\":\"" <> Map.get(row, "id") <> "\",\"rule_id\":\"" <> Map.get(row, "rule_id") <> "\",\"project_id\":\"" <> Map.get(row, "project_id") <> "\",\"status\":\"" <> Map.get(row, "status") <> "\",\"message\":\"" <> Map.get(row, "message") <> "\",\"condition_snapshot\":" <> Map.get(row, "condition_snapshot") <> ",\"triggered_at\":\"" <> Map.get(row, "triggered_at") <> "\",\"acknowledged_at\":" <> format_nullable_ts(Map.get(row, "acknowledged_at")) <> ",\"resolved_at\":" <> format_nullable_ts(Map.get(row, "resolved_at")) <> ",\"rule_name\":\"" <> Map.get(row, "rule_name") <> "\"}"
end

# Helper: perform toggle with extracted enabled value.
fn do_toggle(pool :: PoolHandle, rule_id :: String, enabled_str :: String) do
  let result = toggle_alert_rule(pool, rule_id, enabled_str)
  case result do
    Ok(n) -> HTTP.response(200, json { status: "ok", affected: n })
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# --- Handler functions (pub, defined after all helpers) ---

# Handle POST /api/v1/projects/:project_id/alert-rules (ALERT-01)
# Creates a new alert rule from JSON body.
pub fn handle_create_alert_rule(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let body = Request.body(request)
  let result = create_alert_rule(pool, project_id, body)
  case result do
    Ok(id) -> HTTP.response(201, json { id: id })
    Err(e) -> HTTP.response(400, json { error: e })
  end
end

# Handle GET /api/v1/projects/:project_id/alert-rules (ALERT-01)
# Lists all alert rules for a project.
pub fn handle_list_alert_rules(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let result = list_alert_rules(pool, project_id)
  case result do
    Ok(rows) -> HTTP.response(200, rows |> List.map(fn(row) do rule_row_to_json(row) end) |> to_json_array())
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Handle POST /api/v1/alert-rules/:rule_id/toggle (ALERT-01)
# Toggles an alert rule enabled/disabled.
# Uses Mesh-native Json.get for field extraction (no DB roundtrip).
pub fn handle_toggle_alert_rule(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let rule_id = require_param(request, "rule_id")
  let body = Request.body(request)
  let enabled_raw = Json.get(body, "enabled")
  let enabled = if String.length(enabled_raw) > 0 do enabled_raw else "true" end
  do_toggle(pool, rule_id, enabled)
end

# Handle POST /api/v1/alert-rules/:rule_id/delete (ALERT-01)
# Deletes an alert rule.
pub fn handle_delete_alert_rule(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let rule_id = require_param(request, "rule_id")
  let result = delete_alert_rule(pool, rule_id)
  case result do
    Ok(n) -> HTTP.response(200, json { status: "ok", affected: n })
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Handle GET /api/v1/projects/:project_id/alerts (ALERT-06)
# Lists alerts for a project with optional status filter.
pub fn handle_list_alerts(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let status = query_or_default(request, "status", "")
  let result = list_alerts(pool, project_id, status)
  case result do
    Ok(rows) -> HTTP.response(200, rows |> List.map(fn(row) do alert_row_to_json(row) end) |> to_json_array())
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Handle POST /api/v1/alerts/:id/acknowledge (ALERT-06)
# Transitions an active alert to acknowledged.
pub fn handle_acknowledge_alert(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let alert_id = require_param(request, "id")
  let result = acknowledge_alert(pool, alert_id)
  case result do
    Ok(n) -> HTTP.response(200, json { status: "ok", affected: n })
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Handle POST /api/v1/alerts/:id/resolve (ALERT-06)
# Transitions an active or acknowledged alert to resolved.
pub fn handle_resolve_alert(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let alert_id = require_param(request, "id")
  let result = resolve_fired_alert(pool, alert_id)
  case result do
    Ok(n) -> HTTP.response(200, json { status: "ok", affected: n })
    Err(e) -> HTTP.response(500, json { error: e })
  end
end
