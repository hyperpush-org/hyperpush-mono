from Types. Job import Job

from Storage. Jobs import RecoveryResult, claim_next_pending_job, reclaim_processing_jobs, mark_job_failed, mark_job_processed

struct WorkerState do
  poll_ms :: Int
  boot_id :: String
  started_at :: String
  last_tick_at :: String
  last_status :: String
  last_job_id :: String
  last_error :: String
  processed_jobs :: Int
  failed_jobs :: Int
  restart_count :: Int
  last_exit_reason :: String
  recovered_jobs :: Int
  last_recovery_at :: String
  last_recovery_job_id :: String
  last_recovery_count :: Int
end

service JobWorkerState do
  fn init(poll_ms :: Int) -> WorkerState do
    WorkerState { poll_ms : poll_ms, boot_id : "", started_at : "", last_tick_at : "", last_status : "starting", last_job_id : "", last_error : "", processed_jobs : 0, failed_jobs : 0, restart_count : 0, last_exit_reason : "", recovered_jobs : 0, last_recovery_at : "", last_recovery_job_id : "", last_recovery_count : 0 }
  end
  
  call GetPollMs() :: Int do|state|
    (state, state.poll_ms)
  end
  
  call GetBootId() :: String do|state|
    (state, state.boot_id)
  end
  
  call GetStartedAt() :: String do|state|
    (state, state.started_at)
  end
  
  call GetLastTickAt() :: String do|state|
    (state, state.last_tick_at)
  end
  
  call GetLastStatus() :: String do|state|
    (state, state.last_status)
  end
  
  call GetLastJobId() :: String do|state|
    (state, state.last_job_id)
  end
  
  call GetLastError() :: String do|state|
    (state, state.last_error)
  end
  
  call GetProcessedJobs() :: Int do|state|
    (state, state.processed_jobs)
  end
  
  call GetFailedJobs() :: Int do|state|
    (state, state.failed_jobs)
  end
  
  call GetRestartCount() :: Int do|state|
    (state, state.restart_count)
  end
  
  call GetLastExitReason() :: String do|state|
    (state, state.last_exit_reason)
  end
  
  call GetRecoveredJobs() :: Int do|state|
    (state, state.recovered_jobs)
  end
  
  call GetLastRecoveryAt() :: String do|state|
    (state, state.last_recovery_at)
  end
  
  call GetLastRecoveryJobId() :: String do|state|
    (state, state.last_recovery_job_id)
  end
  
  call GetLastRecoveryCount() :: Int do|state|
    (state, state.last_recovery_count)
  end
  
  call NoteBoot(ts :: String, boot_id :: String) :: Int do|state|
    let had_boot = String.length(state.started_at) > 0
    let next_status = if had_boot == true do
      "recovering"
    else
      "starting"
    end
    let next_restart_count = if had_boot == true do
      state.restart_count + 1
    else
      state.restart_count
    end
    let next_exit_reason = if had_boot == true do
      if state.last_status == "crashing" do
        if String.length(state.last_error) > 0 do
          state.last_error
        else
          "worker crashed unexpectedly"
        end
      else
        if state.last_status == "processing" do
          "worker exited while processing"
        else
          "worker restarted unexpectedly"
        end
      end
    else
      state.last_exit_reason
    end
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : boot_id, started_at : ts, last_tick_at : ts, last_status : next_status, last_job_id : state.last_job_id, last_error : "", processed_jobs : state.processed_jobs, failed_jobs : state.failed_jobs, restart_count : next_restart_count, last_exit_reason : next_exit_reason, recovered_jobs : state.recovered_jobs, last_recovery_at : state.last_recovery_at, last_recovery_job_id : state.last_recovery_job_id, last_recovery_count : state.last_recovery_count }
    (next_state, 0)
  end
  
  call NoteRecovery(ts :: String, recovery_count :: Int, last_job_id :: String) :: Int do|state|
    let next_status = if recovery_count > 0 do
      "recovering"
    else
      state.last_status
    end
    let next_recovery_at = if recovery_count > 0 do
      ts
    else
      state.last_recovery_at
    end
    let next_recovery_job_id = if recovery_count > 0 do
      last_job_id
    else
      state.last_recovery_job_id
    end
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : state.boot_id, started_at : state.started_at, last_tick_at : ts, last_status : next_status, last_job_id : state.last_job_id, last_error : state.last_error, processed_jobs : state.processed_jobs, failed_jobs : state.failed_jobs, restart_count : state.restart_count, last_exit_reason : state.last_exit_reason, recovered_jobs : state.recovered_jobs + recovery_count, last_recovery_at : next_recovery_at, last_recovery_job_id : next_recovery_job_id, last_recovery_count : recovery_count }
    (next_state, 0)
  end
  
  call NoteTick(ts :: String) :: Int do|state|
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : state.boot_id, started_at : state.started_at, last_tick_at : ts, last_status : state.last_status, last_job_id : state.last_job_id, last_error : state.last_error, processed_jobs : state.processed_jobs, failed_jobs : state.failed_jobs, restart_count : state.restart_count, last_exit_reason : state.last_exit_reason, recovered_jobs : state.recovered_jobs, last_recovery_at : state.last_recovery_at, last_recovery_job_id : state.last_recovery_job_id, last_recovery_count : state.last_recovery_count }
    (next_state, 0)
  end
  
  call NoteIdle(ts :: String) :: Int do|state|
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : state.boot_id, started_at : state.started_at, last_tick_at : ts, last_status : "idle", last_job_id : state.last_job_id, last_error : "", processed_jobs : state.processed_jobs, failed_jobs : state.failed_jobs, restart_count : state.restart_count, last_exit_reason : state.last_exit_reason, recovered_jobs : state.recovered_jobs, last_recovery_at : state.last_recovery_at, last_recovery_job_id : state.last_recovery_job_id, last_recovery_count : state.last_recovery_count }
    (next_state, 0)
  end
  
  call NoteClaimed(ts :: String, job_id :: String) :: Int do|state|
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : state.boot_id, started_at : state.started_at, last_tick_at : ts, last_status : "processing", last_job_id : job_id, last_error : "", processed_jobs : state.processed_jobs, failed_jobs : state.failed_jobs, restart_count : state.restart_count, last_exit_reason : state.last_exit_reason, recovered_jobs : state.recovered_jobs, last_recovery_at : state.last_recovery_at, last_recovery_job_id : state.last_recovery_job_id, last_recovery_count : state.last_recovery_count }
    (next_state, 0)
  end
  
  call NoteProcessed(ts :: String, job_id :: String) :: Int do|state|
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : state.boot_id, started_at : state.started_at, last_tick_at : ts, last_status : "processed", last_job_id : job_id, last_error : "", processed_jobs : state.processed_jobs + 1, failed_jobs : state.failed_jobs, restart_count : state.restart_count, last_exit_reason : state.last_exit_reason, recovered_jobs : state.recovered_jobs, last_recovery_at : state.last_recovery_at, last_recovery_job_id : state.last_recovery_job_id, last_recovery_count : state.last_recovery_count }
    (next_state, 0)
  end
  
  call NoteFailed(ts :: String, job_id :: String, error_message :: String) :: Int do|state|
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : state.boot_id, started_at : state.started_at, last_tick_at : ts, last_status : "failed", last_job_id : job_id, last_error : error_message, processed_jobs : state.processed_jobs, failed_jobs : state.failed_jobs + 1, restart_count : state.restart_count, last_exit_reason : state.last_exit_reason, recovered_jobs : state.recovered_jobs, last_recovery_at : state.last_recovery_at, last_recovery_job_id : state.last_recovery_job_id, last_recovery_count : state.last_recovery_count }
    (next_state, 0)
  end
  
  call NoteCrashSoon(ts :: String, job_id :: String, error_message :: String) :: Int do|state|
    let next_state = WorkerState { poll_ms : state.poll_ms, boot_id : state.boot_id, started_at : state.started_at, last_tick_at : ts, last_status : "crashing", last_job_id : job_id, last_error : error_message, processed_jobs : state.processed_jobs, failed_jobs : state.failed_jobs, restart_count : state.restart_count, last_exit_reason : state.last_exit_reason, recovered_jobs : state.recovered_jobs, last_recovery_at : state.last_recovery_at, last_recovery_job_id : state.last_recovery_job_id, last_recovery_count : state.last_recovery_count }
    (next_state, 0)
  end
end

supervisor JobWorkerSupervisor do
  strategy : one_for_one
  
  max_restarts : 20
  
  max_seconds : 60
  
  child worker do
    start: fn->spawn(supervised_job_worker)end
    restart: permanent
    shutdown: 5000
  end
end

fn worker_state_pid() do
  Process.whereis("reference_backend_worker_state")
end

fn current_timestamp() -> String do
  DateTime.to_iso8601(DateTime.utc_now())
end

fn current_unix_ms() -> Int do
  DateTime.to_unix_ms(DateTime.utc_now())
end

fn recovery_reclaim_grace_ms(poll_ms :: Int) -> Int do
  let doubled_poll_ms = poll_ms * 2
  if doubled_poll_ms < 500 do
    500
  else
    doubled_poll_ms
  end
end

fn recovery_stale_cutoff_unix_ms(worker_state) -> Int do
  let poll_ms = JobWorkerState.get_poll_ms(worker_state)
  current_unix_ms() - recovery_reclaim_grace_ms(poll_ms)
end

fn wait_for_reclaim_window(worker_state) do
  let poll_ms = JobWorkerState.get_poll_ms(worker_state)
  Timer.sleep(recovery_reclaim_grace_ms(poll_ms))
end

fn hold_after_claim_ms(poll_ms :: Int) -> Int do
  let scaled_hold_ms = poll_ms * 12
  if scaled_hold_ms < 1500 do
    1500
  else
    scaled_hold_ms
  end
end

fn hold_after_claim_step_ms(poll_ms :: Int) -> Int do
  if poll_ms < 100 do
    poll_ms
  else
    100
  end
end

fn hold_after_claim_ticks(worker_state, remaining_ms :: Int, step_ms :: Int) do
  if remaining_ms > 0 do
    let current_step_ms = if remaining_ms < step_ms do
      remaining_ms
    else
      step_ms
    end
    Timer.sleep(current_step_ms)
    let _ = JobWorkerState.note_tick(worker_state, current_timestamp())
    hold_after_claim_ticks(worker_state, remaining_ms - current_step_ms, step_ms)
  else
    0
  end
end

fn pause_after_recovery(worker_state, recovered_jobs :: Int) do
  if recovered_jobs > 0 do
    let poll_ms = JobWorkerState.get_poll_ms(worker_state)
    Timer.sleep(poll_ms)
    0
  else
    0
  end
end

fn parse_attempts(value :: String) -> Int do
  let parsed = String.to_int(value)
  case parsed do
    Some( n) -> n
    None -> 0
  end
end

fn recovery_hint(restart_count :: Int) -> String do
  if restart_count > 0 do
    "requeued after worker restart"
  else
    "requeued abandoned processing job during boot recovery"
  end
end

fn log_worker_boot(boot_id :: String, restart_count :: Int) do
  let _ = println("[reference-backend] Job worker boot id=#{boot_id} restart_count=#{restart_count}")
  0
end

fn log_worker_recovery(recovery_count :: Int, last_job_id :: String) do
  if recovery_count > 0 do
    let _ = println("[reference-backend] Job worker recovered jobs=#{recovery_count} last_job_id=#{last_job_id}")
    0
  else
    0
  end
end

fn log_worker_idle() do
  let _ = println("[reference-backend] Job worker idle")
  0
end

fn log_worker_claim_miss(error_message :: String) do
  let _ = println("[reference-backend] Job worker contention miss treated as idle: #{error_message}")
  0
end

fn log_worker_claimed(job :: Job) do
  let _ = println("[reference-backend] Job worker claimed id=#{job.id} attempts=#{job.attempts}")
  0
end

fn log_worker_processed(job :: Job) do
  let _ = println("[reference-backend] Job worker processed id=#{job.id} status=#{job.status} attempts=#{job.attempts}")
  0
end

fn log_worker_failure(job_id :: String, error_message :: String) do
  if String.length(job_id) > 0 do
    let _ = println("[reference-backend] Job worker failed id=#{job_id}: #{error_message}")
    0
  else
    let _ = println("[reference-backend] Job worker failed: #{error_message}")
    0
  end
end

fn record_idle(worker_state, ts :: String) do
  let _ = JobWorkerState.note_idle(worker_state, ts)
  let _ = log_worker_idle()
  0
end

fn record_idle_claim_miss(worker_state, ts :: String, error_message :: String) do
  let _ = JobWorkerState.note_idle(worker_state, ts)
  let _ = log_worker_claim_miss(error_message)
  0
end

fn record_failure(worker_state, job_id :: String, error_message :: String) do
  let ts = current_timestamp()
  let _ = JobWorkerState.note_failed(worker_state, ts, job_id, error_message)
  let _ = log_worker_failure(job_id, error_message)
  0
end

fn record_processed(worker_state, job :: Job) do
  let ts = current_timestamp()
  let _ = JobWorkerState.note_processed(worker_state, ts, job.id)
  let _ = log_worker_processed(job)
  0
end

fn record_recovery_result(worker_state, result :: RecoveryResult) do
  let ts = current_timestamp()
  let _ = JobWorkerState.note_recovery(worker_state, ts, result.count, result.last_job_id)
  let _ = log_worker_recovery(result.count, result.last_job_id)
  0
end

fn mark_failed_after_processing(pool :: PoolHandle,
worker_state,
job :: Job,
error_message :: String) do
  let failed_result = mark_job_failed(pool, job.id, error_message)
  case failed_result do
    Ok( failed_job) -> record_failure(worker_state, failed_job.id, error_message)
    Err( mark_failed_error) -> record_failure(worker_state, job.id, mark_failed_error)
  end
end

fn handle_process_claim_error(pool :: PoolHandle, worker_state, job :: Job, error_message :: String) do
  if String.contains(error_message, "no rows matched") == true do
    record_idle_claim_miss(worker_state, current_timestamp(), error_message)
  else
    mark_failed_after_processing(pool, worker_state, job, error_message)
  end
end

fn should_crash_after_claim(job :: Job) -> Bool do
  if String.contains(job.payload, "crash_after_claim_once") == true do
    parse_attempts(job.attempts) == 1
  else
    false
  end
end

fn should_hold_after_claim(job :: Job) -> Bool do
  String.contains(job.payload, "hold_after_claim_once") == true
end

fn log_worker_hold(job :: Job, hold_ms :: Int) do
  let _ = println("[reference-backend] Job worker hold-after-claim id=#{job.id} attempts=#{job.attempts} hold_ms=#{hold_ms}")
  0
end

fn hold_after_claim(worker_state, job :: Job) do
  let poll_ms = JobWorkerState.get_poll_ms(worker_state)
  let hold_ms = hold_after_claim_ms(poll_ms)
  let step_ms = hold_after_claim_step_ms(poll_ms)
  let _ = log_worker_hold(job, hold_ms)
  hold_after_claim_ticks(worker_state, hold_ms, step_ms)
end

fn crash_after_claim(worker_state, job :: Job) -> Bool do
  let crash_ts = current_timestamp()
  let reason = "worker_crash_after_claim"
  let _ = JobWorkerState.note_crash_soon(worker_state, crash_ts, job.id, reason)
  let _ = println("[reference-backend] Job worker crash injected id=#{job.id} attempts=#{job.attempts}")
  false
end

fn finish_processed_job(worker_state, processed_job :: Job) -> Bool do
  let _ = record_processed(worker_state, processed_job)
  true
end

fn finish_processing_error(pool :: PoolHandle, worker_state, job :: Job, error_message :: String) -> Bool do
  let _ = handle_process_claim_error(pool, worker_state, job, error_message)
  true
end

fn process_claimed_job(pool :: PoolHandle, worker_state, job :: Job) -> Bool do
  let processed_result = mark_job_processed(pool, job.id)
  case processed_result do
    Ok( processed_job) -> finish_processed_job(worker_state, processed_job)
    Err( error_message) -> finish_processing_error(pool, worker_state, job, error_message)
  end
end

fn handle_claimed_job(pool :: PoolHandle, worker_state, job :: Job) -> Bool do
  let claim_ts = current_timestamp()
  let _ = JobWorkerState.note_claimed(worker_state, claim_ts, job.id)
  let _ = log_worker_claimed(job)
  let _ = if should_hold_after_claim(job) == true do
    hold_after_claim(worker_state, job)
  else
    0
  end
  if should_crash_after_claim(job) == true do
    crash_after_claim(worker_state, job)
  else
    process_claimed_job(pool, worker_state, job)
  end
end

fn handle_claim_error(worker_state, error_message :: String) -> Bool do
  if error_message == "no pending jobs" do
    let _ = record_idle(worker_state, current_timestamp())
    true
  else
    if String.contains(error_message, "no rows matched") == true do
      let _ = record_idle_claim_miss(worker_state, current_timestamp(), error_message)
      true
    else
      let _ = record_failure(worker_state, "", error_message)
      true
    end
  end
end

fn handle_claim_result(pool :: PoolHandle, worker_state, claim_result) -> Bool do
  case claim_result do
    Ok( job) -> handle_claimed_job(pool, worker_state, job)
    Err( error_message) -> handle_claim_error(worker_state, error_message)
  end
end

fn process_next_job(pool :: PoolHandle, worker_state) -> Bool do
  let _ = JobWorkerState.note_tick(worker_state, current_timestamp())
  let claim_result = claim_next_pending_job(pool)
  handle_claim_result(pool, worker_state, claim_result)
end

fn job_worker_loop(pool :: PoolHandle, worker_state) do
  let continue_loop = process_next_job(pool, worker_state)
  if continue_loop == true do
    let poll_ms = JobWorkerState.get_poll_ms(worker_state)
    Timer.sleep(poll_ms)
    job_worker_loop(pool, worker_state)
  else
    0
  end
end

fn handle_worker_recovery_success(pool :: PoolHandle, worker_state, result :: RecoveryResult) do
  let _ = record_recovery_result(worker_state, result)
  let _ = pause_after_recovery(worker_state, result.count)
  job_worker_loop(pool, worker_state)
end

fn handle_worker_recovery_failure(pool :: PoolHandle, worker_state, error_message :: String) do
  let _ = record_failure(worker_state, "", error_message)
  job_worker_loop(pool, worker_state)
end

fn handle_worker_pool_open(worker_state, restart_count :: Int, pool :: PoolHandle) do
  let _ = wait_for_reclaim_window(worker_state)
  let stale_cutoff_unix_ms = recovery_stale_cutoff_unix_ms(worker_state)
  let recovery_result = reclaim_processing_jobs(pool,
  recovery_hint(restart_count),
  stale_cutoff_unix_ms)
  case recovery_result do
    Ok( result) -> handle_worker_recovery_success(pool, worker_state, result)
    Err( error_message) -> handle_worker_recovery_failure(pool, worker_state, error_message)
  end
end

fn handle_worker_pool_open_error(worker_state, error_message :: String) do
  let _ = record_failure(worker_state, "", error_message)
  let poll_ms = JobWorkerState.get_poll_ms(worker_state)
  Timer.sleep(poll_ms)
  0
end

actor supervised_job_worker() do
  let worker_state = worker_state_pid()
  
  let boot_ts = current_timestamp()
  
  let _ = JobWorkerState.note_boot(worker_state, boot_ts, boot_ts)
  
  let restart_count = JobWorkerState.get_restart_count(worker_state)
  
  let _ = log_worker_boot(boot_ts, restart_count)
  
  let database_url = Env.get("DATABASE_URL", "")
  
  let pool_result = Pool.open(database_url, 1, 4, 5000)
  
  case pool_result do
    Ok( pool) -> handle_worker_pool_open(worker_state, restart_count, pool)
    Err( error_message) -> handle_worker_pool_open_error(worker_state, error_message)
  end
end

pub fn start_worker(job_poll_ms :: Int) do
  let worker_state = JobWorkerState.start(job_poll_ms)
  let _ = Process.register("reference_backend_worker_state", worker_state)
  spawn(JobWorkerSupervisor)
end

pub fn get_worker_poll_ms() -> Int do
  JobWorkerState.get_poll_ms(worker_state_pid())
end

pub fn get_worker_boot_id() -> String do
  JobWorkerState.get_boot_id(worker_state_pid())
end

pub fn get_worker_started_at() -> String do
  JobWorkerState.get_started_at(worker_state_pid())
end

pub fn get_worker_last_tick_at() -> String do
  JobWorkerState.get_last_tick_at(worker_state_pid())
end

pub fn get_worker_last_status() -> String do
  JobWorkerState.get_last_status(worker_state_pid())
end

pub fn get_worker_last_job_id() -> String do
  JobWorkerState.get_last_job_id(worker_state_pid())
end

pub fn get_worker_last_error() -> String do
  JobWorkerState.get_last_error(worker_state_pid())
end

pub fn get_worker_processed_jobs() -> Int do
  JobWorkerState.get_processed_jobs(worker_state_pid())
end

pub fn get_worker_failed_jobs() -> Int do
  JobWorkerState.get_failed_jobs(worker_state_pid())
end

pub fn get_worker_restart_count() -> Int do
  JobWorkerState.get_restart_count(worker_state_pid())
end

pub fn get_worker_last_exit_reason() -> String do
  JobWorkerState.get_last_exit_reason(worker_state_pid())
end

pub fn get_worker_recovered_jobs() -> Int do
  JobWorkerState.get_recovered_jobs(worker_state_pid())
end

pub fn get_worker_last_recovery_at() -> String do
  JobWorkerState.get_last_recovery_at(worker_state_pid())
end

pub fn get_worker_last_recovery_job_id() -> String do
  JobWorkerState.get_last_recovery_job_id(worker_state_pid())
end

pub fn get_worker_last_recovery_count() -> Int do
  JobWorkerState.get_last_recovery_count(worker_state_pid())
end
