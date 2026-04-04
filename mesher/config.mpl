pub fn database_url_key() -> String do
  "DATABASE_URL"
end

pub fn port_key() -> String do
  "PORT"
end

pub fn ws_port_key() -> String do
  "MESHER_WS_PORT"
end

pub fn rate_limit_window_seconds_key() -> String do
  "MESHER_RATE_LIMIT_WINDOW_SECONDS"
end

pub fn rate_limit_max_events_key() -> String do
  "MESHER_RATE_LIMIT_MAX_EVENTS"
end

pub fn default_port() -> Int do
  8080
end

pub fn default_ws_port() -> Int do
  8081
end

pub fn default_rate_limit_window_seconds() -> Int do
  60
end

pub fn default_rate_limit_max_events() -> Int do
  1000
end

pub fn missing_required_env(name :: String) -> String do
  "Missing required environment variable #{name}"
end

pub fn invalid_positive_int(name :: String) -> String do
  "Invalid #{name}: expected a positive integer"
end
