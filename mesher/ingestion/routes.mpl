# HTTP route handlers for the ingestion API.
# Handlers are bare functions (HTTP routing does not support closures).
# Service PIDs and pool handle are obtained via the PipelineRegistry service,
# which is looked up by name using the cluster-aware get_registry() helper.

from Ingestion.Auth import authenticate_request
from Ingestion.Validation import validate_payload_size
from Ingestion.Pipeline import PipelineRegistry
from Services.RateLimiter import RateLimiter
from Services.EventProcessor import EventProcessor
from Types.Project import Project
from Types.Issue import Issue
from Storage.Queries import (
  resolve_issue,
  archive_issue,
  unresolve_issue,
  assign_issue,
  discard_issue,
  delete_issue,
  list_issues_by_status,
  check_new_issue,
  get_event_alert_rules,
  should_fire_by_cooldown,
  fire_alert,
  check_sample_rate,
  count_unresolved_issues,
  get_issue_project_id
)
from Api.Helpers import require_param, get_registry

# Helper: build 401 response

fn unauthorized_response() do
  HTTP.response(401, json { error : "unauthorized" })
end

# Helper: build 400 response with reason

fn bad_request_response(reason :: String) do
  HTTP.response(400, json { error : reason })
end

# Helper: build 429 rate-limited response with Retry-After header

fn rate_limited_response() do
  HTTP.response(429, json { error : "rate limited" })
end

# Helper: build 202 accepted response

fn accepted_response() do
  HTTP.response(202, json { status : "accepted" })
end

# --- Event broadcasting helpers (STREAM-01, STREAM-04) ---
# Defined before route_to_processor (Mesh requires define-before-use).
# Helper: broadcast issue count from query result rows

fn broadcast_count_from_rows(project_id :: String, rows) do
  if List.length(rows) > 0 do
    let count = Map.get(List.head(rows), "cnt")
    let room = "project:#{project_id}"
    Ws.broadcast(room, """{"type":"issue_count","project_id":"#{project_id}","count":#{count}}""")
    0
  else
    0
  end
end

# Helper: broadcast updated issue count for a project

fn broadcast_issue_count(project_id :: String) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let count_result = count_unresolved_issues(pool, project_id)
  case count_result do
    Ok( rows) -> broadcast_count_from_rows(project_id, rows)
    Err( _) -> 0
  end
end

# --- Event-based alert helpers (ALERT-03, ALERT-04, ALERT-05) ---
# Defined before broadcast_event (define-before-use, decision [90-03]).
# Broadcast alert notification to project WebSocket room (ALERT-04).

fn broadcast_alert_notification(project_id :: String,
alert_id :: String,
rule_name :: String,
condition_type :: String,
message :: String) do
  let room = "project:#{project_id}"
  let msg = """{"type":"alert","alert_id":"#{alert_id}","rule_name":"#{rule_name}","condition":"#{condition_type}","message":"#{message}"}"""
  Ws.broadcast(room, msg)
  0
end

# Fire alert if cooldown allows (ALERT-05).

fn fire_if_cooldown_ok(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
rule_name :: String,
condition_type :: String,
issue_id :: String,
should_fire :: Bool) do
  if should_fire do
    let message = "#{condition_type} detected for issue #{issue_id}"
    let result = fire_alert(pool, rule_id, project_id, message, condition_type, rule_name)
    case result do
      Ok( alert_id) -> broadcast_alert_notification(project_id,
      alert_id,
      rule_name,
      condition_type,
      message)
      Err( _) -> 0
    end
  else
    0
  end
end

# Fire and broadcast a single event-based alert with cooldown check.

fn fire_event_alert(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
rule_name :: String,
condition_type :: String,
cooldown_str :: String,
issue_id :: String) do
  let cooldown_ok = should_fire_by_cooldown(pool, rule_id, cooldown_str)
  case cooldown_ok do
    Ok( should_fire) -> fire_if_cooldown_ok(pool,
    rule_id,
    project_id,
    rule_name,
    condition_type,
    issue_id,
    should_fire)
    Err( _) -> 0
  end
end

# Loop through matching rules and fire alerts.

fn fire_event_alerts_loop(pool :: PoolHandle,
rules,
project_id :: String,
condition_type :: String,
issue_id :: String,
i :: Int,
total :: Int) do
  if i < total do
    let rule = List.get(rules, i)
    let rule_id = Map.get(rule, "id")
    let rule_name = Map.get(rule, "name")
    let cooldown_str = Map.get(rule, "cooldown_minutes")
    fire_event_alert(pool, rule_id, project_id, rule_name, condition_type, cooldown_str, issue_id)
    fire_event_alerts_loop(pool, rules, project_id, condition_type, issue_id, i + 1, total)
  else
    0
  end
end

# Get matching rules and fire alerts for a condition type.

fn fire_matching_event_alerts(pool :: PoolHandle,
project_id :: String,
condition_type :: String,
issue_id :: String) do
  let rules_result = get_event_alert_rules(pool, project_id, condition_type)
  case rules_result do
    Ok( rules) -> fire_event_alerts_loop(pool,
    rules,
    project_id,
    condition_type,
    issue_id,
    0,
    List.length(rules))
    Err( _) -> 0
  end
end

# Fire new_issue alerts if issue is new.

fn handle_new_issue_alert(pool :: PoolHandle,
project_id :: String,
issue_id :: String,
is_new :: Bool) do
  if is_new do
    fire_matching_event_alerts(pool, project_id, "new_issue", issue_id)
    0
  else
    0
  end
end

# Check for new-issue alerts after event processing (ALERT-03).

fn check_event_alerts(pool :: PoolHandle, project_id :: String, issue_id :: String) do
  let new_result = check_new_issue(pool, issue_id)
  case new_result do
    Ok( is_new) -> handle_new_issue_alert(pool, project_id, issue_id, is_new)
    Err( _) -> 0
  end
end

# Helper: broadcast event notification, issue count, check alerts, and return response

fn broadcast_event(project_id :: String, issue_id :: String, body :: String) do
  let room = "project:#{project_id}"
  let notification = """{"type":"event","issue_id":"#{issue_id}","data":#{body}}"""
  Ws.broadcast(room, notification)
  broadcast_issue_count(project_id)
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  check_event_alerts(pool, project_id, issue_id)
  accepted_response()
end

# Helper: route event to processor, broadcast on success, and build response

fn route_to_processor(processor_pid, project_id :: String, writer_pid, body :: String) do
  let result = EventProcessor.process_event(processor_pid, project_id, writer_pid, body)
  case result do
    Ok( issue_id) -> broadcast_event(project_id, issue_id, body)
    Err( reason) -> bad_request_response(reason)
  end
end

# Helper: process a single event after auth and rate-limit checks pass.

fn process_event_body(processor_pid, project_id :: String, writer_pid, body :: String) do
  let size_check = validate_payload_size(body, 1048576)
  case size_check do
    Err( reason) -> bad_request_response(reason)
    Ok( _) -> route_to_processor(processor_pid, project_id, writer_pid, body)
  end
end

# Helper: handle event after authentication succeeds

fn handle_event_authed(project_id :: String, rate_limiter_pid, processor_pid, writer_pid, request) do
  let allowed = RateLimiter.check_limit(rate_limiter_pid, project_id)
  if allowed do
    let body = Request.body(request)
    process_event_body(processor_pid, project_id, writer_pid, body)
  else
    rate_limited_response()
  end
end

# Helper: act on sampling decision (true = process, false = drop silently).

fn handle_event_sample_decision(should_keep :: Bool,
project_id :: String,
rate_limiter_pid,
processor_pid,
writer_pid,
request) do
  if should_keep do
    handle_event_authed(project_id, rate_limiter_pid, processor_pid, writer_pid, request)
  else
    accepted_response()
  end
end

# Helper: check sampling before proceeding to rate limit + process.

fn handle_event_sampled(pool :: PoolHandle,
project_id :: String,
rate_limiter_pid,
processor_pid,
writer_pid,
request) do
  handle_event_authed(project_id, rate_limiter_pid, processor_pid, writer_pid, request)
end

# Handle POST /api/v1/events
# Flow: get registry -> get pool+pids -> auth -> rate limit -> validate -> process -> 202

pub fn handle_event(request) do
  let reg_pid = get_registry()
  PipelineRegistry.increment_event_count(reg_pid)
  let pool = PipelineRegistry.get_pool(reg_pid)
  let auth_result = authenticate_request(pool, request)
  case auth_result do
    Err( _) -> unauthorized_response()
    Ok( project) -> handle_event_sampled(pool,
    project.id,
    PipelineRegistry.get_rate_limiter(reg_pid),
    PipelineRegistry.get_processor(reg_pid),
    PipelineRegistry.get_writer(reg_pid),
    request)
  end
end

# Helper: handle bulk after authentication succeeds.
# Validates size (5MB limit for bulk), then routes the entire bulk payload
# to EventProcessor for persistence. Individual JSON array element parsing
# is not supported at the Mesh language level; the StorageWriter stores
# the complete bulk JSON for downstream processing.

fn handle_bulk_authed(project_id :: String, rate_limiter_pid, processor_pid, writer_pid, request) do
  let allowed = RateLimiter.check_limit(rate_limiter_pid, project_id)
  if allowed do
    let body = Request.body(request)
    let size_check = validate_payload_size(body, 5242880)
    case size_check do
      Err( reason) -> bad_request_response(reason)
      Ok( _) -> route_to_processor(processor_pid, project_id, writer_pid, body)
    end
  else
    rate_limited_response()
  end
end

# Helper: act on bulk sampling decision (true = process, false = drop silently).

fn handle_bulk_sample_decision(should_keep :: Bool,
project_id :: String,
rate_limiter_pid,
processor_pid,
writer_pid,
request) do
  if should_keep do
    handle_bulk_authed(project_id, rate_limiter_pid, processor_pid, writer_pid, request)
  else
    accepted_response()
  end
end

# Helper: check sampling before proceeding to bulk rate limit + process.

fn handle_bulk_sampled(pool :: PoolHandle,
project_id :: String,
rate_limiter_pid,
processor_pid,
writer_pid,
request) do
  let sample_result = check_sample_rate(pool, project_id)
  case sample_result do
    Ok( should_keep) -> handle_bulk_sample_decision(should_keep,
    project_id,
    rate_limiter_pid,
    processor_pid,
    writer_pid,
    request)
    Err( _) -> handle_bulk_authed(project_id, rate_limiter_pid, processor_pid, writer_pid, request)
  end
end

# Handle POST /api/v1/events/bulk

pub fn handle_bulk(request) do
  let reg_pid = get_registry()
  PipelineRegistry.increment_event_count(reg_pid)
  let pool = PipelineRegistry.get_pool(reg_pid)
  let rate_limiter_pid = PipelineRegistry.get_rate_limiter(reg_pid)
  let processor_pid = PipelineRegistry.get_processor(reg_pid)
  let writer_pid = PipelineRegistry.get_writer(reg_pid)
  let auth_result = authenticate_request(pool, request)
  case auth_result do
    Err( _) -> unauthorized_response()
    Ok( project) -> handle_bulk_sampled(pool,
    project.id,
    rate_limiter_pid,
    processor_pid,
    writer_pid,
    request)
  end
end

# --- Issue state change broadcasting helpers (STREAM-03) ---
# Defined before issue management handlers (Mesh requires define-before-use).
# Helper: broadcast issue update from project lookup rows

fn broadcast_update_from_rows(rows, issue_id :: String, action :: String) do
  if List.length(rows) > 0 do
    let project_id = Map.get(List.head(rows), "project_id")
    let room = "project:#{project_id}"
    let msg = """{"type":"issue","action":"#{action}","issue_id":"#{issue_id}"}"""
    Ws.broadcast(room, msg)
    0
  else
    0
  end
end

# Helper: look up project_id for an issue and broadcast state change notification

fn broadcast_issue_update(pool, issue_id :: String, action :: String) do
  let rows_result = get_issue_project_id(pool, issue_id)
  case rows_result do
    Ok( rows) -> broadcast_update_from_rows(rows, issue_id, action)
    Err( _) -> 0
  end
end

# Helper: broadcast resolve notification then return success response

fn resolve_success(pool, issue_id :: String, n :: Int) do
  broadcast_issue_update(pool, issue_id, "resolved")
  HTTP.response(200, json { status : "ok", affected : n })
end

# Helper: broadcast archive notification then return success response

fn archive_success(pool, issue_id :: String, n :: Int) do
  broadcast_issue_update(pool, issue_id, "archived")
  HTTP.response(200, json { status : "ok", affected : n })
end

# Helper: broadcast unresolve notification then return success response

fn unresolve_success(pool, issue_id :: String, n :: Int) do
  broadcast_issue_update(pool, issue_id, "unresolved")
  HTTP.response(200, json { status : "ok", affected : n })
end

# Helper: broadcast discard notification then return success response

fn discard_success(pool, issue_id :: String, n :: Int) do
  broadcast_issue_update(pool, issue_id, "discarded")
  HTTP.response(200, json { status : "ok", affected : n })
end

# --- Issue management route handlers (Phase 89 Plan 02) ---
# Helper: build a JSON string for a single Issue.
# Uses deriving(Json) on the Issue struct for automatic serialization.

fn issue_to_json_str(issue :: Issue) -> String do
  Json.encode(issue)
end

# Build JSON array from list of issues.

fn issues_to_json(issues :: List < Issue >) -> String do
  let items = issues
    |> List.map(fn (issue) do issue_to_json_str(issue) end)
  "[#{String.join(items, ",")}]"
end

# Handle GET /api/v1/projects/:project_id/issues?status=unresolved

pub fn handle_list_issues(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let project_id = require_param(request, "project_id")
  let status_opt = Request.query(request, "status")
  let status = case status_opt do
    Some( value) -> value
    None -> "unresolved"
  end
  let result = list_issues_by_status(pool, project_id, status)
  case result do
    Ok( issues) -> HTTP.response(200, issues_to_json(issues))
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Handle POST /api/v1/issues/:id/resolve

pub fn handle_resolve_issue(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let issue_id = require_param(request, "id")
  let result = resolve_issue(pool, issue_id)
  case result do
    Ok( n) -> resolve_success(pool, issue_id, n)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Handle POST /api/v1/issues/:id/archive

pub fn handle_archive_issue(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let issue_id = require_param(request, "id")
  let result = archive_issue(pool, issue_id)
  case result do
    Ok( n) -> archive_success(pool, issue_id, n)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Handle POST /api/v1/issues/:id/unresolve

pub fn handle_unresolve_issue(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let issue_id = require_param(request, "id")
  let result = unresolve_issue(pool, issue_id)
  case result do
    Ok( n) -> unresolve_success(pool, issue_id, n)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Helper: perform assignment after extracting user_id from parsed JSON rows.

fn assign_with_user_id(pool :: PoolHandle, issue_id :: String, user_id :: String) do
  let result = assign_issue(pool, issue_id, user_id)
  case result do
    Ok( n) -> HTTP.response(200, json { status : "ok" })
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Handle POST /api/v1/issues/:id/assign
# Extracts user_id from JSON body using Mesh-native Json.get (no DB roundtrip).

pub fn handle_assign_issue(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let issue_id = require_param(request, "id")
  let body = Request.body(request)
  let user_id = Json.get(body, "user_id")
  assign_with_user_id(pool, issue_id, user_id)
end

# Handle POST /api/v1/issues/:id/discard

pub fn handle_discard_issue(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let issue_id = require_param(request, "id")
  let result = discard_issue(pool, issue_id)
  case result do
    Ok( n) -> discard_success(pool, issue_id, n)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Handle POST /api/v1/issues/:id/delete

pub fn handle_delete_issue(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let issue_id = require_param(request, "id")
  let result = delete_issue(pool, issue_id)
  case result do
    Ok( n) -> HTTP.response(200, json { status : "ok", affected : n })
    Err( e) -> HTTP.response(500, json { error : e })
  end
end
