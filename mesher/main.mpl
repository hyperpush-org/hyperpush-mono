# Mesher monitoring platform entry point.

# Connects to PostgreSQL, creates schema and partitions, starts all services.

# Services are defined in mesher/services/ modules.

# Ingestion pipeline wires HTTP routes and WS handler.

# Schema DDL is managed by `meshc migrate up` -- run before starting the application.

from Storage.Schema import create_partitions_ahead

from Services.Org import OrgService

from Services.Project import ProjectService

from Services.User import UserService

from Services.Writer import StorageWriter

from Ingestion.Pipeline import start_pipeline

from Ingestion.Routes import (
  handle_event,
  handle_bulk,
  handle_resolve_issue,
  handle_archive_issue,
  handle_unresolve_issue,
  handle_assign_issue,
  handle_discard_issue,
  handle_delete_issue
)

from Api.Search import handle_search_issues, handle_search_events, handle_filter_by_tag, handle_list_issue_events

from Api.Dashboard import (
  handle_event_volume,
  handle_error_breakdown,
  handle_top_issues,
  handle_tag_breakdown,
  handle_issue_timeline,
  handle_project_health
)

from Api.Detail import handle_event_detail

from Api.Team import (
  handle_list_members,
  handle_add_member,
  handle_update_member_role,
  handle_remove_member,
  handle_list_api_keys,
  handle_create_api_key,
  handle_revoke_api_key
)

from Api.Alerts import (
  handle_create_alert_rule,
  handle_list_alert_rules,
  handle_toggle_alert_rule,
  handle_delete_alert_rule,
  handle_list_alerts,
  handle_acknowledge_alert,
  handle_resolve_alert
)

from Api.Settings import handle_get_project_settings, handle_update_project_settings, handle_get_project_storage

from Ingestion.WsHandler import ws_on_connect, ws_on_message, ws_on_close

fn connect_to_peer(peer :: String) do
  if peer != "" do
    let connect_result = Node.connect(peer)
    if connect_result == 0 do
      println("[Mesher] Connected to peer: #{peer}")
    else
      println("[Mesher] Failed to connect to peer: #{peer}")
    end
  else
    println("[Mesher] No peers configured")
  end
end

fn try_connect_peers() do
  let peer = Env.get("MESHER_PEERS", "")
  if peer != "" do
    connect_to_peer(peer)
  else
    println("[Mesher] No peers configured")
  end
end

fn start_node_with(node_name :: String, cookie :: String) do
  let start_result = Node.start(node_name, cookie)
  if start_result == 0 do
    println("[Mesher] Node started: #{node_name}")
  else
    println("[Mesher] Node start failed for: #{node_name}")
  end
  try_connect_peers()
end

fn try_start_with_cookie(node_name :: String) do
  let cookie = Env.get("MESHER_COOKIE", "")
  if cookie != "" do
    start_node_with(node_name, cookie)
  else
    println("[Mesher] Running in standalone mode (no distribution)")
  end
end

fn start_node() do
  let name = Env.get("MESHER_NODE_NAME", "")
  if name != "" do
    try_start_with_cookie(name)
  else
    println("[Mesher] Running in standalone mode (no distribution)")
  end
end

fn on_ws_connect(conn, path, headers) do
  ws_on_connect(conn, path, headers)
end

fn on_ws_message(conn, msg) do
  ws_on_message(conn, msg)
end

fn on_ws_close(conn, code :: Int, reason :: String) do
  ws_on_close(conn, code, reason)
end

fn start_services(pool :: PoolHandle) do
  # Start distributed node if configured (before services, after PG)
  start_node()
  # Create initial partitions (7 days ahead)
  let partition_result = create_partitions_ahead(pool, 7)
  case partition_result do
    Ok( _) -> println("[Mesher] Partitions created (7 days ahead)")
    Err( _) -> println("[Mesher] Partition error")
  end
  # Start services
  let org_svc = OrgService.start(pool)
  println("[Mesher] OrgService started")
  let project_svc = ProjectService.start(pool)
  println("[Mesher] ProjectService started")
  let user_svc = UserService.start(pool)
  println("[Mesher] UserService started")
  # Start ingestion pipeline (registers PipelineRegistry by name)
  let registry_pid = start_pipeline(pool)
  println("[Mesher] Foundation ready")
  # Read configurable ports from environment (defaults: 8080 HTTP, 8081 WS)
  let ws_port = Env.get_int("MESHER_WS_PORT", 8081)
  let http_port = Env.get_int("MESHER_HTTP_PORT", 8080)
  # Start WebSocket server (non-blocking -- spawns accept thread in runtime)
  println("[Mesher] WebSocket server starting on :#{ws_port}")
  Ws.serve(on_ws_connect, on_ws_message, on_ws_close, ws_port)
  # Set up HTTP routes and start server (ingestion, search, dashboard, detail, issues, team, API keys)
  println("[Mesher] HTTP server starting on :#{http_port}")
  let router = HTTP.router()
                                                                          |> HTTP.on_post("/api/v1/events",
                                                                          handle_event)
                                                                        |> HTTP.on_post("/api/v1/events/bulk",
                                                                        handle_bulk)
                                                                      |> HTTP.on_get("/api/v1/projects/:project_id/issues",
                                                                      handle_search_issues)
                                                                    |> HTTP.on_get("/api/v1/projects/:project_id/events/search",
                                                                    handle_search_events)
                                                                  |> HTTP.on_get("/api/v1/projects/:project_id/events/tags",
                                                                  handle_filter_by_tag)
                                                                |> HTTP.on_get("/api/v1/issues/:issue_id/events",
                                                                handle_list_issue_events)
                                                              |> HTTP.on_get("/api/v1/projects/:project_id/dashboard/volume",
                                                              handle_event_volume)
                                                            |> HTTP.on_get("/api/v1/projects/:project_id/dashboard/levels",
                                                            handle_error_breakdown)
                                                          |> HTTP.on_get("/api/v1/projects/:project_id/dashboard/top-issues",
                                                          handle_top_issues)
                                                        |> HTTP.on_get("/api/v1/projects/:project_id/dashboard/tags",
                                                        handle_tag_breakdown)
                                                      |> HTTP.on_get("/api/v1/issues/:issue_id/timeline",
                                                      handle_issue_timeline)
                                                    |> HTTP.on_get("/api/v1/projects/:project_id/dashboard/health",
                                                    handle_project_health)
                                                  |> HTTP.on_get("/api/v1/events/:event_id",
                                                  handle_event_detail)
                                                |> HTTP.on_post("/api/v1/issues/:id/resolve",
                                                handle_resolve_issue)
                                              |> HTTP.on_post("/api/v1/issues/:id/archive",
                                              handle_archive_issue)
                                            |> HTTP.on_post("/api/v1/issues/:id/unresolve",
                                            handle_unresolve_issue)
                                          |> HTTP.on_post("/api/v1/issues/:id/assign",
                                          handle_assign_issue)
                                        |> HTTP.on_post("/api/v1/issues/:id/discard",
                                        handle_discard_issue)
                                      |> HTTP.on_post("/api/v1/issues/:id/delete",
                                      handle_delete_issue)
                                    |> HTTP.on_get("/api/v1/orgs/:org_id/members",
                                    handle_list_members)
                                  |> HTTP.on_post("/api/v1/orgs/:org_id/members", handle_add_member)
                                |> HTTP.on_post("/api/v1/orgs/:org_id/members/:membership_id/role",
                                handle_update_member_role)
                              |> HTTP.on_post("/api/v1/orgs/:org_id/members/:membership_id/remove",
                              handle_remove_member)
                            |> HTTP.on_get("/api/v1/projects/:project_id/api-keys",
                            handle_list_api_keys)
                          |> HTTP.on_post("/api/v1/projects/:project_id/api-keys",
                          handle_create_api_key)
                        |> HTTP.on_post("/api/v1/api-keys/:key_id/revoke", handle_revoke_api_key)
                      |> HTTP.on_get("/api/v1/projects/:project_id/alert-rules",
                      handle_list_alert_rules)
                    |> HTTP.on_post("/api/v1/projects/:project_id/alert-rules",
                    handle_create_alert_rule)
                  |> HTTP.on_post("/api/v1/alert-rules/:rule_id/toggle", handle_toggle_alert_rule)
                |> HTTP.on_post("/api/v1/alert-rules/:rule_id/delete", handle_delete_alert_rule)
              |> HTTP.on_get("/api/v1/projects/:project_id/alerts", handle_list_alerts)
            |> HTTP.on_post("/api/v1/alerts/:id/acknowledge", handle_acknowledge_alert)
          |> HTTP.on_post("/api/v1/alerts/:id/resolve", handle_resolve_alert)
        |> HTTP.on_get("/api/v1/projects/:project_id/settings", handle_get_project_settings)
      |> HTTP.on_post("/api/v1/projects/:project_id/settings", handle_update_project_settings)
    |> HTTP.on_get("/api/v1/projects/:project_id/storage", handle_get_project_storage)
  HTTP.serve(router, http_port)
end

fn main() do
  println("[Mesher] Connecting to PostgreSQL...")
  let pool_result = Pool.open("postgres://mesh:mesh@localhost:5432/mesher", 2, 10, 5000)
  case pool_result do
    Ok( pool) -> start_services(pool)
    Err( _) -> println("[Mesher] Failed to connect to PostgreSQL")
  end
end
