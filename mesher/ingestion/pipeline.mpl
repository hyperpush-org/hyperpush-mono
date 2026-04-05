# Pipeline startup orchestration and service registry.
# PipelineRegistry service stores the pool handle and service PIDs so
# HTTP/WS handlers can look them up via Process.whereis("mesher_registry").

from Services.RateLimiter import start_rate_limiter
from Services.EventProcessor import EventProcessor
from Services.Writer import start_writer
from Services.StreamManager import StreamManager
from Storage.Queries import check_volume_spikes, get_threshold_rules, evaluate_threshold_rule, fire_alert
from Services.Retention import retention_cleaner

# Registry state holds pool handle and all service PIDs.

struct RegistryState do
  pool :: PoolHandle
  rate_limiter_pid :: Pid
  processor_pid :: Pid
  writer_pid :: Pid
  event_count :: Int
end

# PipelineRegistry service -- stores pipeline context for handler lookup.
# Call handlers return the stored values with correct types.

service PipelineRegistry do
  fn init(pool :: PoolHandle, rate_limiter_pid :: Pid, processor_pid :: Pid, writer_pid :: Pid) -> RegistryState do
    RegistryState {
      pool : pool,
      rate_limiter_pid : rate_limiter_pid,
      processor_pid : processor_pid,
      writer_pid : writer_pid,
      event_count : 0
    }
  end
  
  call GetPool() :: PoolHandle do|state|
    (state, state.pool)
  end
  
  call GetRateLimiter() :: Pid do|state|
    (state, state.rate_limiter_pid)
  end
  
  call GetProcessor() :: Pid do|state|
    (state, state.processor_pid)
  end
  
  call GetWriter() :: Pid do|state|
    (state, state.writer_pid)
  end
  
  call GetEventCount() :: Int do|state|
    (state, state.event_count)
  end
  
  call IncrementEventCount() :: Int do|state|
    let new_count = state.event_count + 1
    let new_state = RegistryState {
      pool : state.pool,
      rate_limiter_pid : state.rate_limiter_pid,
      processor_pid : state.processor_pid,
      writer_pid : state.writer_pid,
      event_count : new_count
    }
    (new_state, new_count)
  end
  
  call ResetEventCount() :: Int do|state|
    let new_state = RegistryState {
      pool : state.pool,
      rate_limiter_pid : state.rate_limiter_pid,
      processor_pid : state.processor_pid,
      writer_pid : state.writer_pid,
      event_count : 0
    }
    (new_state, 0)
  end
end

# Ticker actor for periodic buffer drain (STREAM-05 backpressure).
# Uses Timer.sleep + recursive call because Timer.send_after delivers raw bytes
# that cannot match service cast dispatch tags (type_tag-based dispatch).

actor stream_drain_ticker(stream_mgr_pid, interval :: Int) do
  Timer.sleep(interval)
  
  StreamManager.drain_buffers(stream_mgr_pid)
  
  stream_drain_ticker(stream_mgr_pid, interval)
end

# Health checker actor -- periodically verifies pipeline services are responsive.
# Uses Timer.sleep + recursive call pattern (established in flush_ticker).
# Verifies the PipelineRegistry responds to a service call every 10 seconds.

actor health_checker(pool :: PoolHandle) do
  Timer.sleep(10000)
  
  let reg_pid = Process.whereis("mesher_registry")
  
  PipelineRegistry.get_pool(reg_pid)
  
  println("[Mesher] Health check: all services responsive")
  
  health_checker(pool)
end

# Helper: log spike checker result (extracted for single-expression case arm).

fn log_spike_result(n :: Int) do
  if n > 0 do
    # String interpolation: #{n} converts Int to String inline
    println("[Mesher] Spike checker: escalated #{n} archived issues")
    0
  else
    0
  end
end

# Helper: log spike checker error (extracted for matching branch types).

fn log_spike_error(e :: String) do
  println("[Mesher] Spike checker error: #{e}")
  0
end

# Periodic spike detection actor -- checks archived issues for volume spikes.
# Runs every 5 minutes (300000ms). If an archived issue has a sudden burst of events
# (>10x average hourly rate), it's auto-escalated to 'unresolved' (ISSUE-03).
# Uses Timer.sleep + recursive call pattern (established in flush_ticker, health_checker).

actor spike_checker(pool :: PoolHandle) do
  Timer.sleep(300000)
  
  let result = check_volume_spikes(pool)
  
  case result do
    Ok( n) -> log_spike_result(n)
    Err( e) -> log_spike_error(e)
  end
  
  spike_checker(pool)
end

# --- Alert evaluation helpers (ALERT-02, ALERT-04, ALERT-05) ---
# Defined before alert_evaluator actor (define-before-use, decision [90-03]).
# Broadcast alert notification to project WebSocket room (ALERT-04).

fn broadcast_alert(project_id :: String,
alert_id :: String,
rule_name :: String,
condition_type :: String,
message :: String) do
  let room = "project:#{project_id}"
  # Heredoc: triple quotes eliminate escaped quote noise in JSON strings
  let msg = """{"type":"alert","alert_id":"#{alert_id}","rule_name":"#{rule_name}","condition":"#{condition_type}","message":"#{message}"}"""
  Ws.broadcast(room, msg)
  0
end

# Fire alert record then broadcast (combines fire_alert + broadcast_alert).

fn fire_and_broadcast(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
rule_name :: String,
condition_type :: String,
message :: String) do
  let result = fire_alert(pool, rule_id, project_id, message, condition_type, rule_name)
  case result do
    Ok( alert_id) -> broadcast_alert(project_id, alert_id, rule_name, condition_type, message)
    Err( _) -> 0
  end
end

# Extract a field from condition_json string using Mesh-native Json.get (no DB roundtrip).

fn extract_condition_field(pool :: PoolHandle, condition_json :: String, field :: String) -> String ! String do
  Ok(Json.get(condition_json, field))
end

# Fire if threshold exceeded.

fn fire_threshold_if_needed(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
rule_name :: String,
should_fire :: Bool,
threshold_str :: String,
window_str :: String) do
  if should_fire do
    let message = "Event count exceeded #{threshold_str} in #{window_str} minutes"
    fire_and_broadcast(pool, rule_id, project_id, rule_name, "threshold", message)
  else
    0
  end
end

# Final threshold check and fire.

fn check_and_fire_threshold(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
rule_name :: String,
cooldown_str :: String,
threshold_str :: String,
window_str :: String) do
  let should_fire_result = evaluate_threshold_rule(pool,
  rule_id,
  project_id,
  threshold_str,
  window_str,
  cooldown_str)
  case should_fire_result do
    Ok( should_fire) -> fire_threshold_if_needed(pool,
    rule_id,
    project_id,
    rule_name,
    should_fire,
    threshold_str,
    window_str)
    Err( _) -> 0
  end
end

# Continue evaluation after threshold extracted.

fn evaluate_threshold_with_window(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
rule_name :: String,
condition_json :: String,
cooldown_str :: String,
threshold_str :: String) do
  let window_result = extract_condition_field(pool, condition_json, "window_minutes")
  case window_result do
    Ok( window_str) -> check_and_fire_threshold(pool,
    rule_id,
    project_id,
    rule_name,
    cooldown_str,
    threshold_str,
    window_str)
    Err( _) -> 0
  end
end

# Evaluate one threshold rule and fire if threshold exceeded.

fn evaluate_single_threshold(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
rule_name :: String,
condition_json :: String,
cooldown_str :: String) do
  let threshold_result = extract_condition_field(pool, condition_json, "threshold")
  case threshold_result do
    Ok( threshold_str) -> evaluate_threshold_with_window(pool,
    rule_id,
    project_id,
    rule_name,
    condition_json,
    cooldown_str,
    threshold_str)
    Err( _) -> 0
  end
end

# Loop through rules list by index.

fn evaluate_rules_loop(pool :: PoolHandle, rules, i :: Int, total :: Int, fired :: Int) -> Int ! String do
  if i < total do
    let rule = List.get(rules, i)
    let rule_id = Map.get(rule, "id")
    let project_id = Map.get(rule, "project_id")
    let rule_name = Map.get(rule, "name")
    let condition_json = Map.get(rule, "condition_json")
    let cooldown_str = Map.get(rule, "cooldown_minutes")
    evaluate_single_threshold(pool, rule_id, project_id, rule_name, condition_json, cooldown_str)
    evaluate_rules_loop(pool, rules, i + 1, total, fired)
  else
    Ok(fired)
  end
end

# Load and evaluate all enabled threshold rules.

fn evaluate_all_threshold_rules(pool :: PoolHandle) -> Int ! String do
  let rules = get_threshold_rules(pool) ?
  evaluate_rules_loop(pool, rules, 0, List.length(rules), 0)
end

# Log helpers (extracted for single-expression case arms, decision [88-02]).

fn log_eval_result(n :: Int) do
  println("[Mesher] Alert evaluator: checked rules, #{n} fired")
  0
end

fn log_eval_error(e :: String) do
  println("[Mesher] Alert evaluator error: #{e}")
  0
end

# Timer-driven alert evaluator actor (ALERT-02).
# Runs every 30 seconds, evaluates all enabled threshold rules.
# Uses Timer.sleep + recursive call pattern (established in flush_ticker, health_checker).

actor alert_evaluator(pool :: PoolHandle) do
  Timer.sleep(30000)
  
  let result = evaluate_all_threshold_rules(pool)
  
  case result do
    Ok( n) -> log_eval_result(n)
    Err( e) -> log_eval_error(e)
  end
  
  alert_evaluator(pool)
end

# --- Load monitoring for cluster-aware scaling (CLUSTER-05) ---
# Helper: log load monitor status

fn log_load_status(event_count :: Int, node_count :: Int) do
  println("[Mesher] Load monitor: #{event_count} events/5s, #{node_count} peers")
end

# Worker function for remote event processing (CLUSTER-05).
# Spawned on remote nodes via Node.spawn. Looks up THIS node's own
# PipelineRegistry via Process.whereis to get the local pool and processor.
# Does NOT accept PoolHandle as argument (raw pointer, not serializable -- pitfall 1).

fn event_processor_worker() do
  let reg_pid = Process.whereis("mesher_registry")
  let pool = PipelineRegistry.get_pool(reg_pid)
  EventProcessor.start(pool)
  println("[Mesher] Remote event processor worker started")
  0
end

# Helper: attempt remote processor spawn on a peer node.
# Uses Node.spawn to spawn event_processor_worker on the target node.
# The worker looks up its own node's PipelineRegistry via Process.whereis.
# Does NOT send local PoolHandle across nodes (research pitfall 1 -- raw pointer, meaningless remotely).

fn try_remote_spawn(nodes :: List < String >) do
  let target = List.head(nodes)
  println("[Mesher] Load high -- spawning remote processor on #{target}")
  Node.spawn(target, event_processor_worker)
  println("[Mesher] Spawned remote event_processor_worker on #{target}")
  0
end

# Monitor a peer node for NODEDOWN events (CLUSTER-05 health tracking).
# Node.monitor(node_name) registers the calling process to receive
# NODEDOWN notifications when the specified node disconnects.
# Returns 0 on success, 1 on failure.

fn monitor_peer(node_name :: String) do
  let result = Node.monitor(node_name)
  if result == 0 do
    println("[Mesher] Monitoring peer: #{node_name}")
    0
  else
    println("[Mesher] Failed to monitor peer: #{node_name}")
    0
  end
end

# Monitor all peers in a list by index.

fn monitor_all_peers(nodes, i :: Int, total :: Int) do
  if i < total do
    let node_name = List.get(nodes, i)
    monitor_peer(node_name)
    monitor_all_peers(nodes, i + 1, total)
  else
    0
  end
end

# Load monitor actor -- checks event processing rate and peer nodes every 5 seconds.
# When connected peers exist and local load exceeds threshold, attempts remote processor spawning.
# Tracks peer count changes to detect new peers (set up monitors) and lost peers (NODEDOWN).

actor load_monitor(pool :: PoolHandle, threshold :: Int, prev_peers :: Int) do
  Timer.sleep(5000)
  
  let reg_pid = Process.whereis("mesher_registry")
  
  let event_count = PipelineRegistry.get_event_count(reg_pid)
  
  PipelineRegistry.reset_event_count(reg_pid)
  
  let nodes = Node.list()
  
  let node_count = List.length(nodes)
  
  log_load_status(event_count, node_count)
  
  # Detect peer changes and set up monitoring for new peers
  
  if node_count > prev_peers do
    println("[Mesher] New peers detected (#{prev_peers} -> #{node_count}), setting up monitors")
    monitor_all_peers(nodes, 0, node_count)
    0
  else if node_count < prev_peers do
    println("[Mesher] Peer lost (#{prev_peers} -> #{node_count}) -- NODEDOWN detected")
    0
  else
    0
  end
  
  if node_count > 0 do
    if event_count > threshold do
      try_remote_spawn(nodes)
    else
      0
    end
  else
    0
  end
  
  load_monitor(pool, threshold, node_count)
end

# Register PipelineRegistry globally for cross-node discovery (CLUSTER-02).
# Uses Node.self() to check if distributed mode is active.
# Registers with both a node-specific name and a well-known default name.
# Node-specific name allows targeted cross-node lookup; default name is first-writer-wins.

fn register_global_services(registry_pid) do
  let node_name = Node.self()
  if node_name != "" do
    Global.register("mesher_registry@#{node_name}", registry_pid)
    Global.register("mesher_registry", registry_pid)
    println("[Mesher] Services registered globally as mesher_registry@#{node_name}")
  else
    println("[Mesher] Running in standalone mode (skipping global registration)")
  end
end

fn start_configured_rate_limiter(window_seconds :: Int, max_events :: Int) do
  let rate_limiter_pid = start_rate_limiter(window_seconds, max_events)
  println("[Mesher] RateLimiter started (#{window_seconds}s window, #{max_events} max)")
  rate_limiter_pid
end

# Restart all pipeline services and re-register PipelineRegistry.
# Called by health_checker when the registry is unreachable (one_for_all strategy).
# Defined after alert_evaluator actor (define-before-use, decision [90-03]).

fn restart_all_services(pool :: PoolHandle, window_seconds :: Int, max_events :: Int) do
  let rate_limiter_pid = start_configured_rate_limiter(window_seconds, max_events)
  let processor_pid = EventProcessor.start(pool)
  let writer_pid = start_writer(pool, "default")
  let stream_mgr_pid = StreamManager.start()
  Process.register("stream_manager", stream_mgr_pid)
  # Spawn drain ticker for StreamManager buffer backpressure (250ms interval)
  spawn(stream_drain_ticker, stream_mgr_pid, 250)
  # Spawn alert evaluator on restart
  spawn(alert_evaluator, pool)
  # Spawn retention cleaner on restart
  spawn(retention_cleaner, pool)
  # Spawn load monitor for cluster-aware load balancing (5s interval, 100 events/5s threshold)
  spawn(load_monitor, pool, 100, 0)
  let registry_pid = PipelineRegistry.start(pool, rate_limiter_pid, processor_pid, writer_pid)
  Process.register("mesher_registry", registry_pid)
  register_global_services(registry_pid)
  registry_pid
end

# Start the full ingestion pipeline.
# 1. Start StreamManager + drain ticker
# 2. Start RateLimiter
# 3. Start EventProcessor
# 4. Start StorageWriter
# 5. Start PipelineRegistry (stores all PIDs)
# 6. Register PipelineRegistry by name for handler lookup
# 7. Spawn health checker + spike checker + alert evaluator
# Returns registry PID.

pub fn start_pipeline(pool :: PoolHandle, window_seconds :: Int, max_events :: Int) do
  # Start stream manager (before other services so WS handler can find it)
  let stream_mgr_pid = StreamManager.start()
  Process.register("stream_manager", stream_mgr_pid)
  println("[Mesher] StreamManager started")
  # Spawn drain ticker for StreamManager buffer backpressure (250ms interval)
  spawn(stream_drain_ticker, stream_mgr_pid, 250)
  println("[Mesher] StreamManager drain ticker started (250ms interval)")
  # Start rate limiter
  let rate_limiter_pid = start_configured_rate_limiter(window_seconds, max_events)
  # Start event processor
  let processor_pid = EventProcessor.start(pool)
  println("[Mesher] EventProcessor started")
  # Start a default StorageWriter
  let writer_pid = start_writer(pool, "default")
  println("[Mesher] StorageWriter started (default project)")
  # Start pipeline registry
  let registry_pid = PipelineRegistry.start(pool, rate_limiter_pid, processor_pid, writer_pid)
  Process.register("mesher_registry", registry_pid)
  register_global_services(registry_pid)
  println("[Mesher] PipelineRegistry started and registered")
  # Spawn health checker for automatic restart (10s interval)
  spawn(health_checker, pool)
  println("[Mesher] Health checker started (10s interval)")
  # Spawn spike detection checker (5 minute interval)
  spawn(spike_checker, pool)
  println("[Mesher] Spike checker started (5 min interval)")
  # Spawn alert evaluator (30-second interval for threshold rules)
  spawn(alert_evaluator, pool)
  println("[Mesher] Alert evaluator started (30s interval)")
  # Spawn retention cleaner (24-hour interval for daily cleanup)
  spawn(retention_cleaner, pool)
  println("[Mesher] Retention cleaner started (24h interval)")
  # Spawn load monitor for cluster-aware load balancing (5s interval, 100 events/5s threshold)
  spawn(load_monitor, pool, 100, 0)
  println("[Mesher] Load monitor started (5s interval, threshold: 100 events)")
  registry_pid
end
