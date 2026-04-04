from Config import database_url_key, port_key, job_poll_ms_key, missing_required_env, invalid_positive_int

describe("Config helpers")do test("exposes the canonical environment variable keys")do assert_eq(database_url_key(),
"DATABASE_URL")
assert_eq(port_key(), "PORT")
assert_eq(job_poll_ms_key(), "JOB_POLL_MS") end
test("formats missing-env and invalid-int messages")do assert_eq(missing_required_env(database_url_key()),
"Missing required environment variable DATABASE_URL")
assert_eq(invalid_positive_int(job_poll_ms_key()),
"Invalid JOB_POLL_MS: expected a positive integer") end end
