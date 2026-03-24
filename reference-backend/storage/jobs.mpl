from Types.Job import Job

pub struct RecoveryResult do
  count :: Int
  last_job_id :: String
end

fn jobs_table() -> String do
  "jobs"
end

fn job_from_row(row) -> Job do
  Job { id : Map.get(row, "id"), status : Map.get(row, "status"), attempts : Map.get(row,
  "attempts"), last_error : Map.get(row, "last_error"), payload : Map.get(row, "payload"), created_at : Map.get(row,
  "created_at"), updated_at : Map.get(row, "updated_at"), processed_at : Map.get(row,
  "processed_at") }
end

fn job_select_query() do
  Query.from(jobs_table())
    |> Query.select_raw(["id::text", "status", "attempts::text", "COALESCE(last_error, '') AS last_error", "payload::text", "created_at::text", "updated_at::text", "COALESCE(processed_at::text, '') AS processed_at"])
end

fn find_single_job(rows, missing_message :: String) -> Job ! String do
  if List.length(rows) > 0 do
    Ok(job_from_row(List.head(rows)))
  else
    Err(missing_message)
  end
end

fn job_query_by_id(job_id :: String) do
  job_select_query()
    |> Query.where_raw("id = ?::uuid", [job_id])
end

fn current_timestamp() -> String do
  DateTime.to_iso8601(DateTime.utc_now())
end

fn claim_pending_job_sql() -> String do
  let table = jobs_table()
  "UPDATE " <> table <> " SET status = 'processing', attempts = attempts + 1, updated_at = now() WHERE id = (SELECT id FROM " <> table <> " WHERE status = 'pending' ORDER BY created_at ASC, id ASC FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING id::text AS id, status, attempts::text AS attempts, COALESCE(last_error, '') AS last_error, payload::text AS payload, created_at::text AS created_at, updated_at::text AS updated_at, COALESCE(processed_at::text, '') AS processed_at"
end

fn reclaim_processing_jobs_sql() -> String do
  let table = jobs_table()
  "WITH recovered AS (SELECT id FROM " <> table <> " WHERE status = 'processing' AND updated_at <= to_timestamp($2::double precision / 1000.0) ORDER BY updated_at ASC, id ASC FOR UPDATE SKIP LOCKED), updated AS (UPDATE " <> table <> " SET status = 'pending', last_error = $1, processed_at = NULL, updated_at = now() WHERE id IN (SELECT id FROM recovered) RETURNING id) SELECT id::text AS id FROM updated ORDER BY id ASC"
end

fn recovery_last_job_id(rows) -> String do
  if List.length(rows) > 0 do
    Map.get(List.last(rows), "id")
  else
    ""
  end
end

pub fn create_job(pool :: PoolHandle, payload :: String) -> Job ! String do
  let now_ts = current_timestamp()
  let row = Repo.insert(pool,
  jobs_table(),
  %{"status" => "pending", "attempts" => "0", "payload" => payload, "updated_at" => now_ts}) ?
  let job_id = Map.get(row, "id")
  get_job(pool, job_id)
end

pub fn get_job(pool :: PoolHandle, job_id :: String) -> Job ! String do
  let q = job_query_by_id(job_id)
  let rows = Repo.all(pool, q) ?
  find_single_job(rows, "not found")
end

pub fn claim_next_pending_job(pool :: PoolHandle) -> Job ! String do
  let rows = Repo.query_raw(pool, claim_pending_job_sql(), []) ?
  find_single_job(rows, "no pending jobs")
end

pub fn reclaim_processing_jobs(pool :: PoolHandle,
error_message :: String,
stale_before_unix_ms :: Int) -> RecoveryResult ! String do
  let params = [error_message, String.from(stale_before_unix_ms)]
  let rows = Repo.query_raw(pool, reclaim_processing_jobs_sql(), params) ?
  Ok(RecoveryResult { count : List.length(rows), last_job_id : recovery_last_job_id(rows) })
end

pub fn mark_job_processed(pool :: PoolHandle, job_id :: String) -> Job ! String do
  let ts = current_timestamp()
  let q = Query.from(jobs_table())
    |> Query.where_raw("id = ?::uuid", [job_id])
  Repo.update_where(pool,
  jobs_table(),
  %{"status" => "processed", "last_error" => "", "processed_at" => ts, "updated_at" => ts},
  q) ?
  get_job(pool, job_id)
end

pub fn mark_job_failed(pool :: PoolHandle, job_id :: String, error_message :: String) -> Job ! String do
  let ts = current_timestamp()
  let q = Query.from(jobs_table())
    |> Query.where_raw("id = ?::uuid", [job_id])
  Repo.update_where(pool,
  jobs_table(),
  %{"status" => "failed", "last_error" => error_message, "updated_at" => ts},
  q) ?
  get_job(pool, job_id)
end
