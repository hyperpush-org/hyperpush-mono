# Migration: create_jobs
# Canonical durable job lifecycle for the reference backend.

pub fn up(pool :: PoolHandle) -> Int ! String do
  Pool.execute(pool, "CREATE EXTENSION IF NOT EXISTS pgcrypto", []) ?
  Pool.execute(pool,
  "CREATE TABLE IF NOT EXISTS jobs (id UUID PRIMARY KEY DEFAULT gen_random_uuid(), status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'processed', 'failed')), attempts INTEGER NOT NULL DEFAULT 0, last_error TEXT, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), updated_at TIMESTAMPTZ NOT NULL DEFAULT now(), processed_at TIMESTAMPTZ)",
  []) ?
  Pool.execute(pool,
  "CREATE INDEX IF NOT EXISTS idx_jobs_pending_scan ON jobs (created_at, id) WHERE status = 'pending'",
  []) ?
  Pool.execute(pool,
  "CREATE INDEX IF NOT EXISTS idx_jobs_processing_reclaim_scan ON jobs (updated_at, id) WHERE status = 'processing'",
  []) ?
  Ok(0)
end

pub fn down(pool :: PoolHandle) -> Int ! String do
  Migration.drop_table(pool, "jobs") ?
  Ok(0)
end
