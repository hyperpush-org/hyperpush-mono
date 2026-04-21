# WebSocket callbacks for event streaming.
# Ws.serve provides on_connect, on_message, on_close callbacks.
# Each WS connection runs in its own actor (crash-isolated by runtime).
#
# Dual-purpose WS server on port 8081:
#   /ingest           -> SDK event ingestion (existing behavior)
#   /stream/projects/:id -> dashboard streaming subscription (Phase 90)

from Services.EventProcessor import EventProcessor
from Services.StreamManager import StreamManager
from Ingestion.Pipeline import PipelineRegistry
from Api.Helpers import get_registry

# Helper: check authorization header as fallback

fn check_authorization_header(conn, headers) do
  let has_auth = Map.has_key(headers, "authorization")
  if has_auth do
    conn
  else
    0
  end
end

# Helper: send a text frame to a WS connection (discards send result).

fn ws_write(conn, msg :: String) do
  let _result = Ws.send(conn, msg)
  nil
end

# Helper: send accepted response over WS

fn ws_send_accepted(conn) do
  ws_write(conn, json { status : "accepted" })
end

# Helper: send error response over WS

fn ws_send_error(conn, reason :: String) do
  let msg = json { error : reason }
  ws_write(conn, msg)
end

# Helper: handle a /stream/projects/:id or /ws/stream/projects/:id connection
# Join room and register in StreamManager.

fn handle_stream_connect(conn, path :: String) do
  let parts = String.split(path, "/")
  # /stream/projects/:id  -> ["", "stream", "projects", ":id"]  -> index 3
  # /ws/stream/projects/:id -> ["", "ws", "stream", "projects", ":id"] -> index 4
  let seg1 = List.get(parts, 1)
  let project_id = if seg1 == "ws" do
    List.get(parts, 4)
  else
    List.get(parts, 3)
  end
  let room = "project:#{project_id}"
  Ws.join(conn, room)
  let stream_mgr_pid = Process.whereis("stream_manager")
  StreamManager.register_client(stream_mgr_pid, conn, project_id, "", "")
  conn
end

# Helper: handle an /ingest (or other) connection -- auth check

fn handle_ingest_connect(conn, headers) do
  let has_key = Map.has_key(headers, "x-sentry-auth")
  if has_key do
    conn
  else
    check_authorization_header(conn, headers)
  end
end

# Helper: check if path contains /stream/projects/ (handles /ws prefix from proxy)

fn is_stream_path(path :: String) -> Bool do
  String.contains(path, "/stream/projects/")
end

# WebSocket on_connect callback.
# Receives (conn, path, headers) where headers is a Map<String, String>.
# Routes /stream/projects/:id to room subscription, /ingest to event ingestion auth.

pub fn ws_on_connect(conn, path, headers) do
  let is_stream = is_stream_path(path)
  if is_stream do
    handle_stream_connect(conn, path)
  else
    handle_ingest_connect(conn, headers)
  end
end

# Helper: update subscription filters from a JSON subscribe message.
# Uses Mesh-native Json.get_nested for nested field extraction (no DB roundtrip).

fn handle_subscribe_update(conn, message :: String) do
  let level = Json.get_nested(message, "filters", "level")
  let env = Json.get_nested(message, "filters", "environment")
  let stream_mgr_pid = Process.whereis("stream_manager")
  let project_id = StreamManager.get_project_id(stream_mgr_pid, conn)
  StreamManager.register_client(stream_mgr_pid, conn, project_id, level, env)
  ws_write(conn, "{\"type\":\"filters_updated\"}")
end

# Helper: handle message from an ingestion client (existing behavior)

fn handle_ingest_message(conn, message :: String) do
  let reg_pid = get_registry()
  let processor_pid = PipelineRegistry.get_processor(reg_pid)
  let writer_pid = PipelineRegistry.get_writer(reg_pid)
  let result = EventProcessor.process_event(processor_pid, "ws-project", writer_pid, message)
  case result do
    Ok -> ws_send_accepted(conn)
    Err( reason) -> ws_send_error(conn, reason)
  end
end

# WebSocket on_message callback.
# Receives (conn, message) where message is the raw text frame content.
# Routes streaming clients to subscription updates, ingestion clients to EventProcessor.

pub fn ws_on_message(conn, message) do
  let stream_mgr_pid = Process.whereis("stream_manager")
  let is_stream = StreamManager.is_stream_client(stream_mgr_pid, conn)
  if is_stream do
    handle_subscribe_update(conn, message)
  else
    handle_ingest_message(conn, message)
  end
end

# WebSocket on_close callback.
# Cleans up StreamManager state; Ws.join auto-cleanup handles room removal.

pub fn ws_on_close(conn, code :: Int, reason :: String) do
  let stream_mgr_pid = Process.whereis("stream_manager")
  StreamManager.remove_client(stream_mgr_pid, conn)
  println("[WS] Connection closed: #{code}")
end
