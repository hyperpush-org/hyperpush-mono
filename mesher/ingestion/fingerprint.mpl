# Fingerprint computation module for error grouping.
# Computes a deterministic fingerprint from event payload fields using a
# priority-based fallback chain: custom fingerprint > stack trace frames >
# exception type + value > raw message.
#
# Fingerprints are plain concatenated strings (not hashed). The UNIQUE
# constraint on (project_id, fingerprint) in the issues table handles dedup.
# Line numbers are intentionally excluded from frame fingerprints because
# they change with unrelated code edits (per Rollbar/Sentry best practices).

from Types.Event import EventPayload, StackFrame, ExceptionInfo, Fingerprint

# Normalize an error message for stable fingerprinting.
# Lowercases and strips hex address prefixes (0x). Full regex not available
# in Mesh; this covers the most common source of fingerprint instability.
fn normalize_message(msg :: String) -> Fingerprint do
  # String.replace(subject, from, to) -- pipe msg as subject (arg 1)
  msg
    |> String.to_lower()
    |> String.replace("0x", "")
    |> String.trim()
end

# Build a fingerprint component from a single stack frame.
# Uses filename and function_name only (no line numbers -- GROUP-01).
fn fingerprint_frame(frame :: StackFrame) -> String do
  frame.filename <> "|" <> frame.function_name
end

# Build fingerprint from stack trace frames and message (GROUP-01).
# Format: "file|func;file|func;...:normalized_message"
fn fingerprint_from_frames(frames, msg :: String) -> Fingerprint do
  let suffix = ":" <> normalize_message(msg)
  let joined = frames |> List.map(fn(frame) do fingerprint_frame(frame) end) |> String.join(";")
  joined <> suffix
end

# Fallback fingerprint when no stack trace is available (GROUP-02).
# Priority: exception type:value > "msg:normalized_message"
fn fallback_fingerprint(payload :: EventPayload) -> String do
  case payload.exception do
    Some(exc) -> exc.type_name <> ":" <> normalize_message(exc.value)
    None -> "msg:" <> normalize_message(payload.message)
  end
end

# Try stacktrace fingerprint, falling back if empty.
# Extracted from case arm per Mesh single-expression case arm constraint.
fn try_stacktrace_fingerprint(frames, payload :: EventPayload) -> String do
  let fp = fingerprint_from_frames(frames, payload.message)
  if String.length(fp) > 0 do fp else fallback_fingerprint(payload) end
end

# Compute fingerprint from stacktrace with fallback chain.
# If stacktrace frames produce a non-empty fingerprint, use it;
# otherwise fall back to exception type or raw message.
fn compute_from_stacktrace_or_fallback(payload :: EventPayload) -> String do
  case payload.stacktrace do
    Some(frames) -> try_stacktrace_fingerprint(frames, payload)
    None -> fallback_fingerprint(payload)
  end
end

# Main entry point for fingerprint computation.
# Fallback chain (GROUP-01, GROUP-02, GROUP-03):
#   1. Custom fingerprint override (payload.fingerprint non-empty)
#   2. Stack trace frames (file + function + normalized message)
#   3. Exception type + normalized value
#   4. "msg:" + normalized message
pub fn compute_fingerprint(payload :: EventPayload) -> Fingerprint do
  if String.length(payload.fingerprint) > 0 do
    payload.fingerprint
  else
    compute_from_stacktrace_or_fallback(payload)
  end
end
