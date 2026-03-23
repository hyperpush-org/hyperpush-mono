from Types.Job import Job
from Storage.Jobs import create_job, get_job
from Runtime.Registry import get_pool

fn require_param(request, name :: String) -> String do
  let opt = Request.param(request, name)
  case opt do
    Some(v) -> v
    None -> ""
  end
end

fn encode_optional_string(value :: String) -> String do
  let wrapped = if String.length(value) > 0 do
    json { value: Some(value) }
  else
    json { value: None }
  end
  String.slice(wrapped, 9, String.length(wrapped) - 1)
end

fn job_to_json(job :: Job) -> String do
  "{\"id\":\"" <> job.id <> "\",\"status\":\"" <> job.status <> "\",\"attempts\":" <> job.attempts <> ",\"last_error\":" <> encode_optional_string(job.last_error) <> ",\"payload\":" <> job.payload <> ",\"created_at\":\"" <> job.created_at <> "\",\"updated_at\":\"" <> job.updated_at <> "\",\"processed_at\":" <> encode_optional_string(job.processed_at) <> "}"
end

fn log_create_success(job :: Job, payload :: String) do
  println("[reference-backend] Job created id=#{job.id} status=#{job.status} payload_bytes=#{String.length(payload)}")
end

fn log_get_success(job :: Job) do
  println("[reference-backend] Job fetched id=#{job.id} status=#{job.status} attempts=#{job.attempts} processed_at=#{job.processed_at}")
end

fn create_job_response(job :: Job, payload :: String) do
  let _ = log_create_success(job, payload)
  HTTP.response(201, job_to_json(job))
end

fn create_job_error_response(e :: String) do
  let _ = println("[reference-backend] Job create failed: #{e}")
  HTTP.response(400, json { error: e })
end

fn get_job_success_response(job :: Job) do
  let _ = log_get_success(job)
  HTTP.response(200, job_to_json(job))
end

fn get_job_error_response(job_id :: String, e :: String) do
  if e == "not found" do
    HTTP.response(404, json { error: "job not found" })
  else
    let _ = println("[reference-backend] Job fetch failed id=#{job_id}: #{e}")
    HTTP.response(500, json { error: e })
  end
end

pub fn handle_create_job(request) do
  let pool = get_pool()
  let body = Request.body(request)
  let result = create_job(pool, body)
  case result do
    Ok(job) -> create_job_response(job, body)
    Err(e) -> create_job_error_response(e)
  end
end

pub fn handle_get_job(request) do
  let pool = get_pool()
  let job_id = require_param(request, "id")
  let result = get_job(pool, job_id)
  case result do
    Ok(job) -> get_job_success_response(job)
    Err(e) -> get_job_error_response(job_id, e)
  end
end
