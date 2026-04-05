from Config import database_url_key, port_key, job_poll_ms_key, missing_required_env, invalid_positive_int
from Api.Router import build_router
from Runtime.Registry import start_registry
from Jobs.Worker import start_worker

fn log_config_error(message :: String) do
  println("[reference-backend] Config error: #{message}")
end

fn required_positive_env_int(name :: String) -> Int do
  let raw = Env.get(name, "")
  if raw == "" do
    -1
  else
    let value = Env.get_int(name, -1)
    if value > 0 do
      value
    else
      -1
    end
  end
end

fn start_runtime(port :: Int, job_poll_ms :: Int) do
  println("[reference-backend] Runtime ready worker_poll_ms=#{job_poll_ms}")
  let router = build_router()
  println("[reference-backend] HTTP server starting on :#{port}")
  HTTP.serve(router, port)
end

fn on_pool_ready(database_url :: String, port :: Int, job_poll_ms :: Int, pool :: PoolHandle) do
  println("[reference-backend] PostgreSQL pool ready")
  start_registry(pool, database_url, job_poll_ms)
  println("[reference-backend] Runtime registry ready")
  start_worker(job_poll_ms)
  println("[reference-backend] Job worker ready")
  start_runtime(port, job_poll_ms)
end

fn start_with_values(database_url :: String, port :: Int, job_poll_ms :: Int) do
  println("[reference-backend] Config loaded port=#{port} job_poll_ms=#{job_poll_ms}")
  println("[reference-backend] Connecting to PostgreSQL pool...")
  let pool_result = Pool.open(database_url, 1, 4, 5000)
  case pool_result do
    Ok( pool) -> on_pool_ready(database_url, port, job_poll_ms, pool)
    Err( e) -> println("[reference-backend] PostgreSQL connect failed: #{e}")
  end
end

fn maybe_start_with_job_poll_ms(database_url :: String, port :: Int) do
  let job_poll_ms_env = job_poll_ms_key()
  let job_poll_ms_raw = Env.get(job_poll_ms_env, "")
  let job_poll_ms = required_positive_env_int(job_poll_ms_env)
  if job_poll_ms_raw == "" do
    log_config_error(missing_required_env(job_poll_ms_env))
  else
    if job_poll_ms > 0 do
      start_with_values(database_url, port, job_poll_ms)
    else
      log_config_error(invalid_positive_int(job_poll_ms_env))
    end
  end
end

fn maybe_start_with_port(database_url :: String) do
  let port_env = port_key()
  let port_raw = Env.get(port_env, "")
  let port = required_positive_env_int(port_env)
  if port_raw == "" do
    log_config_error(missing_required_env(port_env))
  else
    if port > 0 do
      maybe_start_with_job_poll_ms(database_url, port)
    else
      log_config_error(invalid_positive_int(port_env))
    end
  end
end

fn main() do
  let database_url_env = database_url_key()
  let database_url = Env.get(database_url_env, "")
  if database_url == "" do
    log_config_error(missing_required_env(database_url_env))
  else
    maybe_start_with_port(database_url)
  end
end
