# Seed migration: default organization, project, and API key for development.
# The canonical maintainer runbook lives in `mesher/README.md`.
# After running `cargo run -q -p meshc -- migrate mesher up` and starting `./mesher/mesher`, use:
#
#   curl -X POST http://127.0.0.1:8080/api/v1/events \
#     -H "x-sentry-auth: mshr_devdefaultapikey000000000000000000000000000" \
#     -H "Content-Type: application/json" \
#     -d '{"message":"README smoke event","level":"error"}'
#
#   curl http://127.0.0.1:8080/api/v1/projects/default/issues?status=unresolved
#
# This seed is idempotent: safe to apply multiple times (ON CONFLICT DO NOTHING).

pub fn up(pool :: PoolHandle) -> Int ! String do
  # 1. Insert default organization (idempotent via slug UNIQUE column constraint)
  Pool.execute(pool,
  "INSERT INTO organizations (name, slug) VALUES ('Default Organization', 'default') ON CONFLICT (slug) DO NOTHING",
  []) ?
  # 2. Get the org id (whether just inserted or already existed)
  let org_rows = Repo.query_raw(pool,
  "SELECT id::text FROM organizations WHERE slug = 'default'",
  []) ?
  if List.length(org_rows) > 0 do
    let org_id = Map.get(List.head(org_rows), "id")
    # 3. Insert default project
    # NOTE: projects.slug has only a partial unique index (WHERE slug IS NOT NULL),
    # not a column-level UNIQUE constraint. PostgreSQL requires the partial index
    # predicate in the ON CONFLICT clause — omitting WHERE slug IS NOT NULL causes
    # "ERROR: there is no unique or exclusion constraint matching the ON CONFLICT specification".
    Pool.execute(pool,
    "INSERT INTO projects (org_id, name, platform, slug) VALUES ($1::uuid, 'Default Project', 'mesh', 'default') ON CONFLICT (slug) WHERE slug IS NOT NULL DO NOTHING",
    [org_id]) ?
    # 4. Get the project id
    let proj_rows = Repo.query_raw(pool, "SELECT id::text FROM projects WHERE slug = 'default'", []) ?
    if List.length(proj_rows) > 0 do
      let project_id = Map.get(List.head(proj_rows), "id")
      # 5. Insert default API key (idempotent via key_value UNIQUE column constraint)
      Pool.execute(pool,
      "INSERT INTO api_keys (project_id, key_value, label) VALUES ($1::uuid, 'mshr_devdefaultapikey000000000000000000000000000', 'dev-default') ON CONFLICT (key_value) DO NOTHING",
      [project_id]) ?
      Ok(0)
    else
      Err("seed: failed to find default project after insert")
    end
  else
    Err("seed: failed to find default organization after insert")
  end
end

pub fn down(pool :: PoolHandle) -> Int ! String do
  # Delete in reverse FK order
  Pool.execute(pool,
  "DELETE FROM api_keys WHERE key_value = 'mshr_devdefaultapikey000000000000000000000000000'",
  []) ?
  Pool.execute(pool, "DELETE FROM projects WHERE slug = 'default'", []) ?
  Pool.execute(pool, "DELETE FROM organizations WHERE slug = 'default'", []) ?
  Ok(0)
end
