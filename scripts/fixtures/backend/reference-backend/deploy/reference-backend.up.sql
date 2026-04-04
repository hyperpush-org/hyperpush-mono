-- Derived from reference-backend/migrations/20260323010000_create_jobs.mpl.
-- Keep the Mesh migration file canonical for compiler-driven workflows.

BEGIN;

CREATE TABLE IF NOT EXISTS _mesh_migrations (
  version BIGINT PRIMARY KEY,
  name TEXT NOT NULL,
  applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS jobs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'processed', 'failed')),
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  payload JSONB NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  processed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_jobs_pending_scan
  ON jobs (created_at, id)
  WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_jobs_processing_reclaim_scan
  ON jobs (updated_at, id)
  WHERE status = 'processing';

INSERT INTO _mesh_migrations (version, name)
VALUES (20260323010000, 'create_jobs')
ON CONFLICT (version) DO NOTHING;

COMMIT;
