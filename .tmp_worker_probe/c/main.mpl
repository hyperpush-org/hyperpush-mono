fn force_worker_crash(0) = 0

fn log_worker_failure(job_id :: String, error_message :: String) do
  if String.length(job_id) > 0 do
    let _ = println("failed id=#{job_id}: #{error_message}")
    0
  else
    let _ = println("failed: #{error_message}")
    0
  end
end

fn main() do
  log_worker_failure("", "boom")
  force_worker_crash(1)
end
