# StorageWriter service -- per-project batch writer with bounded buffer and dual flush triggers.
# Receives enriched event entries (project_id|||issue_id|||fingerprint|||event_json), buffers them,
# and flushes to PostgreSQL in batches.
# Dual flush triggers: size-based (batch_size threshold) and timer-based (flush_ticker actor).
# Retry strategy: 3 attempts with exponential backoff (100ms, 500ms), drop on final failure.
# Buffer backpressure: drop-oldest when max_buffer exceeded.

from Storage.Writer import flush_batch

# WriterState holds all service state for a single project writer.
# Buffer stores enriched event entries as "project_id|||issue_id|||fingerprint|||event_json"
# strings. The flush loop splits these to pass project_id, issue_id, and fingerprint as
# separate SQL parameters to flush_batch.

struct WriterState do
  pool :: PoolHandle
  project_id :: String
  buffer :: List < String >
  buffer_len :: Int
  batch_size :: Int
  max_buffer :: Int
end

# --- Batch flush and retry logic ---
# LOCKED: 3 attempts with exponential backoff (100ms, 500ms), drop on final failure.
# Uses recursive loop since Mesh has no mutable variable assignment.

fn flush_drop(project_id :: String, count_val :: Int) -> String ! String do
  println("[StorageWriter] Dropping batch of #{count_val} events for project #{project_id} after 3 retries")
  Ok("dropped")
end

fn flush_retry3(pool :: PoolHandle, project_id :: String, events, event_count :: Int) -> String ! String do
  Timer.sleep(500)
  let r3 = flush_batch(pool, events)
  case r3 do
    Ok -> Ok("flushed")
    Err -> flush_drop(project_id, event_count)
  end
end

fn flush_retry2(pool :: PoolHandle, project_id :: String, events, event_count :: Int) -> String ! String do
  Timer.sleep(100)
  let r2 = flush_batch(pool, events)
  case r2 do
    Ok -> Ok("flushed")
    Err -> flush_retry3(pool, project_id, events, event_count)
  end
end

fn flush_with_retry(pool :: PoolHandle, project_id :: String, events) -> String ! String do
  let event_count = List.length(events)
  let r1 = flush_batch(pool, events)
  case r1 do
    Ok -> Ok("flushed")
    Err -> flush_retry2(pool, project_id, events, event_count)
  end
end

# --- Buffer management helpers ---
# Kept as standalone functions for local reuse and to keep the service block
# readable; the equivalent append/capacity/rebuild logic now compiles inline.

fn writer_store(state :: WriterState, event_json :: String) -> WriterState do
  let appended = List.append(state.buffer, event_json)
  let new_len = state.buffer_len + 1
  # Drop oldest if over capacity (LOCKED: drop-oldest backpressure)
  let buf = if new_len > state.max_buffer do
    List.drop(appended, new_len - state.max_buffer)
  else
    appended
  end
  let blen = if new_len > state.max_buffer do
    state.max_buffer
  else
    new_len
  end
  # Flush if batch size reached (LOCKED: size trigger)
  if blen >= state.batch_size do
    flush_with_retry(state.pool, state.project_id, buf)
    WriterState {
      pool : state.pool,
      project_id : state.project_id,
      buffer : List.new(),
      buffer_len : 0,
      batch_size : state.batch_size,
      max_buffer : state.max_buffer
    }
  else
    WriterState {
      pool : state.pool,
      project_id : state.project_id,
      buffer : buf,
      buffer_len : blen,
      batch_size : state.batch_size,
      max_buffer : state.max_buffer
    }
  end
end

fn writer_flush(state :: WriterState) -> WriterState do
  if state.buffer_len > 0 do
    flush_with_retry(state.pool, state.project_id, state.buffer)
    WriterState {
      pool : state.pool,
      project_id : state.project_id,
      buffer : List.new(),
      buffer_len : 0,
      batch_size : state.batch_size,
      max_buffer : state.max_buffer
    }
  else
    state
  end
end

# --- Storage Writer Service ---
# Per-project batch writer with bounded buffer and dual flush triggers.
# LOCKED: per-project writers for isolation, drop-oldest backpressure,
# size + timer flush triggers, retry with exponential backoff.
# flush_batch imported from Storage.Writer. All service state stays local.

service StorageWriter do
  fn init(pool :: PoolHandle, project_id :: String) -> WriterState do
    WriterState {
      pool : pool,
      project_id : project_id,
      buffer : List.new(),
      buffer_len : 0,
      batch_size : 50,
      max_buffer : 500
    }
  end
  
  # Store an enriched event entry (project_id|||issue_id|||fingerprint|||event_json) in the buffer.
  
  # Drops oldest events if buffer exceeds capacity.
  
  # Triggers immediate flush if buffer reaches batch_size threshold.
  
  cast Store(event_json) do|state|
    writer_store(state, event_json)
  end
  
  # Flush all buffered events to PostgreSQL. Called by ticker actor on timer.
  
  cast Flush() do|state|
    writer_flush(state)
  end
end

# Ticker actor for periodic flush (LOCKED: timer flush trigger).
# Uses Timer.sleep + recursive call because Timer.send_after delivers raw bytes
# that cannot match service cast dispatch tags (type_tag-based dispatch).
# Spawned alongside each StorageWriter to provide the timer-based flush trigger.

actor flush_ticker(writer_pid, interval :: Int) do
  Timer.sleep(interval)
  
  StorageWriter.flush(writer_pid)
  
  flush_ticker(writer_pid, interval)
end

pub fn start_writer(pool :: PoolHandle, project_id :: String) do
  let writer_pid = StorageWriter.start(pool, project_id)
  spawn(flush_ticker, writer_pid, 1000)
  writer_pid
end
