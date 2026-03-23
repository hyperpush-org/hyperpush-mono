from Jobs.Worker import get_worker_failed_jobs, get_worker_last_error, get_worker_last_job_id, get_worker_last_status, get_worker_last_tick_at, get_worker_poll_ms, get_worker_processed_jobs, get_worker_started_at

fn encode_optional_string(value :: String) -> String do
  let wrapped = if String.length(value) > 0 do
    json { value: Some(value) }
  else
    json { value: None }
  end
  String.slice(wrapped, 9, String.length(wrapped) - 1)
end

fn health_json() -> String do
  let worker_poll_ms = get_worker_poll_ms()
  let worker_started_at = get_worker_started_at()
  let worker_last_tick_at = get_worker_last_tick_at()
  let worker_last_status = get_worker_last_status()
  let worker_last_job_id = get_worker_last_job_id()
  let worker_last_error = get_worker_last_error()
  let worker_processed_jobs = get_worker_processed_jobs()
  let worker_failed_jobs = get_worker_failed_jobs()

  "{\"status\":\"ok\",\"worker\":{\"status\":\"" <> worker_last_status <> "\",\"poll_ms\":" <> String.from(worker_poll_ms) <> ",\"started_at\":" <> encode_optional_string(worker_started_at) <> ",\"last_tick_at\":" <> encode_optional_string(worker_last_tick_at) <> ",\"last_job_id\":" <> encode_optional_string(worker_last_job_id) <> ",\"last_error\":" <> encode_optional_string(worker_last_error) <> ",\"processed_jobs\":" <> String.from(worker_processed_jobs) <> ",\"failed_jobs\":" <> String.from(worker_failed_jobs) <> "}}"
end

pub fn handle_health(request) do
  HTTP.response(200, health_json())
end
