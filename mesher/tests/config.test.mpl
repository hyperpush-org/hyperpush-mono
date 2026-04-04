from Config import (
  database_url_key,
  port_key,
  ws_port_key,
  rate_limit_window_seconds_key,
  rate_limit_max_events_key,
  default_port,
  default_ws_port,
  default_rate_limit_window_seconds,
  default_rate_limit_max_events,
  missing_required_env,
  invalid_positive_int
)

describe("Config helpers") do
  test("exposes the canonical environment variable keys") do
    assert_eq(database_url_key(), "DATABASE_URL")
    assert_eq(port_key(), "PORT")
    assert_eq(ws_port_key(), "MESHER_WS_PORT")
    assert_eq(rate_limit_window_seconds_key(), "MESHER_RATE_LIMIT_WINDOW_SECONDS")
    assert_eq(rate_limit_max_events_key(), "MESHER_RATE_LIMIT_MAX_EVENTS")
  end

  test("exposes truthful local-development defaults") do
    assert(default_port() == 8080)
    assert(default_ws_port() == 8081)
    assert(default_rate_limit_window_seconds() == 60)
    assert(default_rate_limit_max_events() == 1000)
  end

  test("formats missing-env and invalid-int messages") do
    assert_eq(
      missing_required_env(database_url_key()),
      "Missing required environment variable DATABASE_URL"
    )
    assert_eq(
      invalid_positive_int(port_key()),
      "Invalid PORT: expected a positive integer"
    )
    assert_eq(
      invalid_positive_int(ws_port_key()),
      "Invalid MESHER_WS_PORT: expected a positive integer"
    )
    assert_eq(
      invalid_positive_int(rate_limit_window_seconds_key()),
      "Invalid MESHER_RATE_LIMIT_WINDOW_SECONDS: expected a positive integer"
    )
    assert_eq(
      invalid_positive_int(rate_limit_max_events_key()),
      "Invalid MESHER_RATE_LIMIT_MAX_EVENTS: expected a positive integer"
    )
  end
end
