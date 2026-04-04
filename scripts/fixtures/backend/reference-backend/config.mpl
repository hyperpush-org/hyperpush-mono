pub fn database_url_key() -> String do
  "DATABASE_URL"
end

pub fn port_key() -> String do
  "PORT"
end

pub fn job_poll_ms_key() -> String do
  "JOB_POLL_MS"
end

pub fn missing_required_env(name :: String) -> String do
  "Missing required environment variable #{name}"
end

pub fn invalid_positive_int(name :: String) -> String do
  "Invalid #{name}: expected a positive integer"
end
