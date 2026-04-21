# EventProcessor service -- routes raw event JSON through SQL-backed field
# extraction, discard checks, issue upsert, and StorageWriter forwarding.
# Uses a synchronous call handler so HTTP routes get processing results back.
# The current ingestion route does auth/rate-limit/payload-size checks, then
# passes the raw body here; fingerprint, title, and level are derived by
# Storage.Queries.extract_event_fields(...), not by Mesh-side payload parsing.
#
# Event processing pipeline:
#   1. Extract fingerprint, title, and level via SQL (extract_event_fields)
#   2. Check whether the fingerprint is discarded (is_issue_discarded)
#   3. Upsert the issue with regression detection (upsert_issue)
#   4. Build an enriched entry and forward the original JSON to StorageWriter

from Storage.Queries import upsert_issue, is_issue_discarded, extract_event_fields
from Services.Writer import StorageWriter

struct ProcessorState do
  pool :: PoolHandle
  processed_count :: Int
end

# Build an enriched entry string for StorageWriter.
# Format: "project_id|||issue_id|||fingerprint|||event_json"
# The StorageWriter splits this to pass project_id, issue_id, and fingerprint
# as separate SQL parameters to insert_event (avoiding JSON field injection in Mesh).

fn build_enriched_entry(project_id :: String,
issue_id :: String,
fingerprint :: String,
event_json :: String) -> String do
  "#{project_id}|||#{issue_id}|||#{fingerprint}|||#{event_json}"
end

fn store_enriched_event(writer_pid,
project_id :: String,
event_json :: String,
issue_id :: String,
fingerprint :: String) -> String ! String do
  let enriched = build_enriched_entry(project_id, issue_id, fingerprint, event_json)
  StorageWriter.store(writer_pid, enriched)
  StorageWriter.flush(writer_pid)
  Ok(issue_id)
end

fn upsert_and_store(pool :: PoolHandle,
project_id :: String,
writer_pid,
event_json :: String,
fingerprint :: String,
title :: String,
level :: String) -> String ! String do
  let upsert_result = upsert_issue(pool, project_id, fingerprint, title, level)
  case upsert_result do
    Err( e) -> Err(e)
    Ok( issue_id) -> store_enriched_event(writer_pid, project_id, event_json, issue_id, fingerprint)
  end
end

fn process_discarded_result(pool :: PoolHandle,
project_id :: String,
writer_pid,
event_json :: String,
fingerprint :: String,
title :: String,
level :: String,
discarded :: Bool) -> String ! String do
  if discarded do
    Ok("discarded")
  else
    upsert_and_store(pool, project_id, writer_pid, event_json, fingerprint, title, level)
  end
end

fn process_fields(pool :: PoolHandle,
project_id :: String,
writer_pid,
event_json :: String,
fields :: Map < String, String >) -> String ! String do
  let fingerprint = Map.get(fields, "fingerprint")
  let title = Map.get(fields, "title")
  let level = Map.get(fields, "level")
  let discarded_result = is_issue_discarded(pool, project_id, fingerprint)
  case discarded_result do
    Err( e) -> Err(e)
    Ok( discarded) -> process_discarded_result(pool,
    project_id,
    writer_pid,
    event_json,
    fingerprint,
    title,
    level,
    discarded)
  end
end

# Route an event through the live ingestion path.
# Accepts raw event JSON from the route layer, asks Storage.Queries to extract
# fingerprint/title/level via SQL, then applies discard checks, issue upsert,
# and StorageWriter forwarding.

fn route_event(pool :: PoolHandle, project_id :: String, writer_pid, event_json :: String) -> String ! String do
  let fields_result = extract_event_fields(pool, event_json)
  case fields_result do
    Err( e) -> Err(e)
    Ok( fields) -> process_fields(pool, project_id, writer_pid, event_json, fields)
  end
end

fn next_processed_count(state :: ProcessorState, result :: String ! String) -> Int do
  case result do
    Ok( issue_id) -> if issue_id == "discarded" do
      state.processed_count
    else
      state.processed_count + 1
    end
    Err -> state.processed_count
  end
end

service EventProcessor do
  fn init(pool :: PoolHandle) -> ProcessorState do
    ProcessorState {
      pool : pool,
      processed_count : 0
    }
  end
  
  # Synchronous event processing for the live ingestion path.
  
  # Takes raw event JSON from the route layer, uses SQL-side extraction for
  
  # fingerprint/title/level, applies discard checks and issue upsert, and
  
  # forwards the enriched entry to StorageWriter.
  
  # Returns Ok(issue_id) on success, Ok("discarded") for suppressed events.
  
  call ProcessEvent(project_id :: String, writer_pid, event_json :: String) do|state|
    let result = route_event(state.pool, project_id, writer_pid, event_json)
    let new_state = ProcessorState {
      pool : state.pool,
      processed_count : next_processed_count(state, result)
    }
    (new_state, result)
  end
end
