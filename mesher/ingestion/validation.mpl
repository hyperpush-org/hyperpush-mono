# Event payload validation functions.
# Checks required fields, valid severity levels, and payload constraints.

from Types.Event import EventPayload

# Check if the severity level is valid.
pub fn validate_level(level :: String) -> String!String do
  let valid_levels = ["fatal", "error", "warning", "info", "debug"]
  let is_valid = List.any(valid_levels, fn(l) do l == level end)
  if is_valid do
    Ok("valid")
  else
    Err("invalid level: must be fatal, error, warning, info, or debug")
  end
end

# Validate a single event payload. Checks required fields and valid severity level.
# Returns Ok("valid") or Err with a descriptive error message.
pub fn validate_event(payload :: EventPayload) -> String!String do
  let msg_len = String.length(payload.message)
  if msg_len == 0 do
    Err("missing required field: message")
  else
    validate_level(payload.level)
  end
end

# Validate payload size. Returns Err if body exceeds max bytes (1MB default).
pub fn validate_payload_size(body :: String, max_bytes :: Int) -> String!String do
  let body_len = String.length(body)
  if body_len > max_bytes do
    Err("payload too large")
  else
    Ok("ok")
  end
end

# Validate bulk event count. Returns Err if count exceeds max (100 default).
pub fn validate_bulk_count(count :: Int, max_events :: Int) -> String!String do
  if count > max_events do
    Err("too many events in bulk request (max " <> String.from(max_events) <> ")")
  else
    Ok("ok")
  end
end
