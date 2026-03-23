from Types.Job import Job

fn jobs_table() -> String do
  "jobs"
end

fn job_from_row(row) -> Job do
  Job {
    id: Map.get(row, "id"),
    status: Map.get(row, "status"),
    attempts: Map.get(row, "attempts"),
    last_error: Map.get(row, "last_error"),
    payload: Map.get(row, "payload"),
    created_at: Map.get(row, "created_at"),
    updated_at: Map.get(row, "updated_at"),
    processed_at: Map.get(row, "processed_at")
  }
end

fn job_select_query() do
  Query.from(jobs_table())
    |> Query.select_raw([
      "id::text",
      "status",
      "attempts::text",
      "COALESCE(last_error, '') AS last_error",
      "payload::text",
      "created_at::text",
      "updated_at::text",
      "COALESCE(processed_at::text, '') AS processed_at"
    ])
end

fn find_single_job(rows, missing_message :: String) -> Job!String do
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

fn parse_attempts(value :: String) -> Int do
  let parsed = String.to_int(value)
  case parsed do
    Some(n) -> n
    None -> 0
  end
end

fn current_timestamp() -> String do
  DateTime.to_iso8601(DateTime.utc_now())
end

fn oldest_pending_job(pool :: PoolHandle) -> Job!String do
  let q = job_select_query()
    |> Query.where_raw("status = 'pending'", [])
    |> Query.order_by(:created_at, :asc)
    |> Query.limit(1)
  let rows = Repo.all(pool, q)?
  find_single_job(rows, "no pending jobs")
end

pub fn create_job(pool :: PoolHandle, payload :: String) -> Job!String do
  let now_ts = current_timestamp()
  let row = Repo.insert(pool, jobs_table(), %{
    "status" => "pending",
    "attempts" => "0",
    "payload" => payload,
    "updated_at" => now_ts
  })?
  let job_id = Map.get(row, "id")
  get_job(pool, job_id)
end

pub fn get_job(pool :: PoolHandle, job_id :: String) -> Job!String do
  let q = job_query_by_id(job_id)
  let rows = Repo.all(pool, q)?
  find_single_job(rows, "not found")
end

pub fn claim_next_pending_job(pool :: PoolHandle) -> Job!String do
  let job = oldest_pending_job(pool)?
  let ts = current_timestamp()
  let next_attempts = String.from(parse_attempts(job.attempts) + 1)
  let q = Query.from(jobs_table())
    |> Query.where_raw("id = ?::uuid AND status = 'pending'", [job.id])
  let _ = Repo.update_where(pool, jobs_table(), %{
    "status" => "processing",
    "attempts" => next_attempts,
    "updated_at" => ts
  }, q)?
  get_job(pool, job.id)
end

pub fn mark_job_processed(pool :: PoolHandle, job_id :: String) -> Job!String do
  let ts = current_timestamp()
  let q = Query.from(jobs_table())
    |> Query.where_raw("id = ?::uuid", [job_id])
  let _ = Repo.update_where(pool, jobs_table(), %{
    "status" => "processed",
    "last_error" => "",
    "processed_at" => ts,
    "updated_at" => ts
  }, q)?
  get_job(pool, job_id)
end

pub fn mark_job_failed(pool :: PoolHandle, job_id :: String, error_message :: String) -> Job!String do
  let ts = current_timestamp()
  let q = Query.from(jobs_table())
    |> Query.where_raw("id = ?::uuid", [job_id])
  let _ = Repo.update_where(pool, jobs_table(), %{
    "status" => "failed",
    "last_error" => error_message,
    "updated_at" => ts
  }, q)?
  get_job(pool, job_id)
end
