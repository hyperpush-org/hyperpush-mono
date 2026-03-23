from Types.Job import Job
from Storage.Jobs import claim_next_pending_job, mark_job_failed, mark_job_processed

struct WorkerState do
  poll_ms :: Int
  started_at :: String
  last_tick_at :: String
  last_status :: String
  last_job_id :: String
  last_error :: String
  processed_jobs :: Int
  failed_jobs :: Int
end

service JobWorkerState do
  fn init(poll_ms :: Int, started_at :: String) -> WorkerState do
    WorkerState {
      poll_ms: poll_ms,
      started_at: started_at,
      last_tick_at: started_at,
      last_status: "starting",
      last_job_id: "",
      last_error: "",
      processed_jobs: 0,
      failed_jobs: 0
    }
  end

  call GetPollMs() :: Int do |state|
    (state, state.poll_ms)
  end

  call GetStartedAt() :: String do |state|
    (state, state.started_at)
  end

  call GetLastTickAt() :: String do |state|
    (state, state.last_tick_at)
  end

  call GetLastStatus() :: String do |state|
    (state, state.last_status)
  end

  call GetLastJobId() :: String do |state|
    (state, state.last_job_id)
  end

  call GetLastError() :: String do |state|
    (state, state.last_error)
  end

  call GetProcessedJobs() :: Int do |state|
    (state, state.processed_jobs)
  end

  call GetFailedJobs() :: Int do |state|
    (state, state.failed_jobs)
  end

  cast NoteTick(ts :: String) do |state|
    WorkerState {
      poll_ms: state.poll_ms,
      started_at: state.started_at,
      last_tick_at: ts,
      last_status: state.last_status,
      last_job_id: state.last_job_id,
      last_error: state.last_error,
      processed_jobs: state.processed_jobs,
      failed_jobs: state.failed_jobs
    }
  end

  cast NoteIdle(ts :: String) do |state|
    WorkerState {
      poll_ms: state.poll_ms,
      started_at: state.started_at,
      last_tick_at: ts,
      last_status: "idle",
      last_job_id: state.last_job_id,
      last_error: "",
      processed_jobs: state.processed_jobs,
      failed_jobs: state.failed_jobs
    }
  end

  cast NoteClaimed(ts :: String, job_id :: String) do |state|
    WorkerState {
      poll_ms: state.poll_ms,
      started_at: state.started_at,
      last_tick_at: ts,
      last_status: "processing",
      last_job_id: job_id,
      last_error: "",
      processed_jobs: state.processed_jobs,
      failed_jobs: state.failed_jobs
    }
  end

  cast NoteProcessed(ts :: String, job_id :: String) do |state|
    WorkerState {
      poll_ms: state.poll_ms,
      started_at: state.started_at,
      last_tick_at: ts,
      last_status: "processed",
      last_job_id: job_id,
      last_error: "",
      processed_jobs: state.processed_jobs + 1,
      failed_jobs: state.failed_jobs
    }
  end

  cast NoteFailed(ts :: String, job_id :: String, error_message :: String) do |state|
    WorkerState {
      poll_ms: state.poll_ms,
      started_at: state.started_at,
      last_tick_at: ts,
      last_status: "failed",
      last_job_id: job_id,
      last_error: error_message,
      processed_jobs: state.processed_jobs,
      failed_jobs: state.failed_jobs + 1
    }
  end
end

fn worker_state_pid() do
  Process.whereis("reference_backend_worker_state")
end

fn current_timestamp() -> String do
  DateTime.to_iso8601(DateTime.utc_now())
end

fn log_worker_tick(job_poll_ms :: Int) do
  println("[reference-backend] Job worker tick poll_ms=#{job_poll_ms}")
end

fn log_worker_idle() do
  println("[reference-backend] Job worker idle")
end

fn log_worker_claimed(job :: Job) do
  println("[reference-backend] Job worker claimed id=#{job.id} attempts=#{job.attempts}")
end

fn log_worker_processed(job :: Job) do
  println("[reference-backend] Job worker processed id=#{job.id} status=#{job.status} attempts=#{job.attempts}")
end

fn log_worker_failure(job_id :: String, error_message :: String) do
  if String.length(job_id) > 0 do
    println("[reference-backend] Job worker failed id=#{job_id}: #{error_message}")
  else
    println("[reference-backend] Job worker failed: #{error_message}")
  end
end

fn note_failed(worker_state, job_id :: String, error_message :: String) do
  let ts = current_timestamp()
  let _ = JobWorkerState.note_failed(worker_state, ts, job_id, error_message)
  log_worker_failure(job_id, error_message)
end

fn process_claimed_job(pool :: PoolHandle, worker_state, job :: Job) do
  let claim_ts = current_timestamp()
  let _ = JobWorkerState.note_claimed(worker_state, claim_ts, job.id)
  let _ = log_worker_claimed(job)

  let processed_result = mark_job_processed(pool, job.id)
  case processed_result do
    Ok(processed_job) -> do
      let processed_ts = if String.length(processed_job.processed_at) > 0 do processed_job.processed_at else current_timestamp() end
      let _ = JobWorkerState.note_processed(worker_state, processed_ts, processed_job.id)
      log_worker_processed(processed_job)
    end
    Err(e) -> do
      let failed_result = mark_job_failed(pool, job.id, e)
      case failed_result do
        Ok(_) -> note_failed(worker_state, job.id, e)
        Err(mark_failed_error) -> note_failed(worker_state, job.id, mark_failed_error)
      end
    end
  end
end

fn run_worker_iteration(pool :: PoolHandle, job_poll_ms :: Int, worker_state) do
  let tick_ts = current_timestamp()
  let _ = JobWorkerState.note_tick(worker_state, tick_ts)
  let _ = log_worker_tick(job_poll_ms)

  let claim_result = claim_next_pending_job(pool)
  case claim_result do
    Ok(job) -> process_claimed_job(pool, worker_state, job)
    Err(e) -> do
      if e == "no pending jobs" do
        let _ = JobWorkerState.note_idle(worker_state, tick_ts)
        log_worker_idle()
      else
        note_failed(worker_state, "", e)
      end
    end
  end
end

actor job_worker(pool :: PoolHandle, job_poll_ms :: Int, worker_state) do
  Timer.sleep(job_poll_ms)
  run_worker_iteration(pool, job_poll_ms, worker_state)
  job_worker(pool, job_poll_ms, worker_state)
end

pub fn start_worker(pool :: PoolHandle, job_poll_ms :: Int) do
  let started_at = current_timestamp()
  let worker_state = JobWorkerState.start(job_poll_ms, started_at)
  let _ = Process.register("reference_backend_worker_state", worker_state)
  let worker_pid = spawn(job_worker, pool, job_poll_ms, worker_state)
  let _ = Process.register("reference_backend_worker", worker_pid)
  println("[reference-backend] Job worker started poll_ms=#{job_poll_ms}")
  worker_pid
end

pub fn get_worker_poll_ms() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_poll_ms(worker_state)
end

pub fn get_worker_started_at() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_started_at(worker_state)
end

pub fn get_worker_last_tick_at() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_last_tick_at(worker_state)
end

pub fn get_worker_last_status() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_last_status(worker_state)
end

pub fn get_worker_last_job_id() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_last_job_id(worker_state)
end

pub fn get_worker_last_error() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_last_error(worker_state)
end

pub fn get_worker_processed_jobs() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_processed_jobs(worker_state)
end

pub fn get_worker_failed_jobs() do
  let worker_state = worker_state_pid()
  JobWorkerState.get_failed_jobs(worker_state)
end
