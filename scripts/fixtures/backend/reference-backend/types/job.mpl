# Shared job row and lifecycle types for the reference backend.
# Database-backed job statuses move through pending -> processing -> processed/failed.

pub type JobStatus = String

# Job row shape exposed across storage, worker, and HTTP modules.
# Keep fields as strings because Repo rows arrive through the text protocol.

pub struct Job do
  id :: String
  status :: JobStatus
  attempts :: String
  last_error :: String
  payload :: String
  created_at :: String
  updated_at :: String
  processed_at :: String
end
