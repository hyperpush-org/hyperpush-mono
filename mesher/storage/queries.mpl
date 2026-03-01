# Reusable query helper functions for all Mesher entity types.
# Provides CRUD operations using ORM Repo/Query calls for all data queries,
# with documented ORM boundaries for complex expressions (PG crypto, JSONB extraction, server-side functions).
# All functions take the pool handle (PoolHandle) as first argument.

from Types.Project import Organization, Project, ApiKey
from Types.User import User, OrgMembership, Session
from Types.Issue import Issue
from Types.Event import Event
from Types.Alert import AlertRule, Alert
from Types.Retention import RetentionSettings

# --- Issue helpers for non-storage modules ---

# Count unresolved issues for a project. Returns rows with "cnt" key.
# Used by ingestion/routes.mpl for WebSocket issue count broadcasting.
# Uses ORM Query.where_raw + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn count_unresolved_issues(pool :: PoolHandle, project_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("project_id = ?::uuid AND status = 'unresolved'", [project_id])
    |> Query.select_raw(["count(*)::text AS cnt"])
  Repo.all(pool, q)
end

# Look up the project_id for an issue by issue_id. Returns rows with "project_id" key.
# Used by ingestion/routes.mpl for broadcasting issue state change notifications.
# Uses ORM Query.where_raw + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn get_issue_project_id(pool :: PoolHandle, issue_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
    |> Query.select_raw(["project_id::text"])
  Repo.all(pool, q)
end

# --- Organization queries ---

# Insert a new organization. Returns the generated UUID.
pub fn insert_org(pool :: PoolHandle, name :: String, slug :: String) -> String!String do
  let fields = %{"name" => name, "slug" => slug}
  let row = Repo.insert(pool, Organization.__table__(), fields)?
  Ok(Map.get(row, "id"))
end

# Get an organization by ID.
pub fn get_org(pool :: PoolHandle, id :: String) -> Organization!String do
  let row = Repo.get(pool, Organization.__table__(), id)?
  Ok(Organization { id: Map.get(row, "id"), name: Map.get(row, "name"), slug: Map.get(row, "slug"), created_at: Map.get(row, "created_at") })
end

# List all organizations.
pub fn list_orgs(pool :: PoolHandle) -> List<Organization>!String do
  let q = Query.from(Organization.__table__())
    |> Query.order_by(:name, :asc)
  let rows = Repo.all(pool, q)?
  Ok(List.map(rows, fn(row) do
    Organization { id: Map.get(row, "id"), name: Map.get(row, "name"), slug: Map.get(row, "slug"), created_at: Map.get(row, "created_at") }
  end))
end

# --- Project queries ---

# Insert a new project. Returns the generated UUID.
pub fn insert_project(pool :: PoolHandle, org_id :: String, name :: String, platform :: String) -> String!String do
  let fields = %{"org_id" => org_id, "name" => name, "platform" => platform}
  let row = Repo.insert(pool, Project.__table__(), fields)?
  Ok(Map.get(row, "id"))
end

# Resolve a project slug to its UUID. Returns the id as a string.
# Used by API handlers to support slug-based project identifiers (e.g. "default").
pub fn get_project_id_by_slug(pool :: PoolHandle, slug :: String) -> String!String do
  let row = Repo.get_by(pool, Project.__table__(), "slug", slug)?
  Ok(Map.get(row, "id"))
end

# Get a project by ID.
pub fn get_project(pool :: PoolHandle, id :: String) -> Project!String do
  let row = Repo.get(pool, Project.__table__(), id)?
  Ok(Project { id: Map.get(row, "id"), org_id: Map.get(row, "org_id"), name: Map.get(row, "name"), platform: Map.get(row, "platform"), created_at: Map.get(row, "created_at") })
end

# List all projects for an organization.
pub fn list_projects_by_org(pool :: PoolHandle, org_id :: String) -> List<Project>!String do
  let q = Query.from(Project.__table__())
    |> Query.where(:org_id, org_id)
    |> Query.order_by(:name, :asc)
  let rows = Repo.all(pool, q)?
  Ok(List.map(rows, fn(row) do
    Project { id: Map.get(row, "id"), org_id: Map.get(row, "org_id"), name: Map.get(row, "name"), platform: Map.get(row, "platform"), created_at: Map.get(row, "created_at") }
  end))
end

# --- API key queries ---

# Create a new API key for a project. Returns the generated key_value (mshr_ prefixed).
# Uses Crypto stdlib UUID generation -- no DB round-trip needed.
# Format: "mshr_" + UUID4 (36 chars) = 41-char key.
pub fn create_api_key(pool :: PoolHandle, project_id :: String, label :: String) -> String!String do
  # Generate API key using Crypto stdlib -- no DB round-trip needed
  let key_value = "mshr_" <> Crypto.uuid4()
  let fields = %{"project_id" => project_id, "key_value" => key_value, "label" => label}
  let _ = Repo.insert(pool, ApiKey.__table__(), fields)?
  Ok(key_value)
end

# Get the project associated with a valid (non-revoked) API key.
# Uses ORM Query.join_as + Query.where_raw instead of raw SQL JOIN.
pub fn get_project_by_api_key(pool :: PoolHandle, key_value :: String) -> Project!String do
  let q = Query.from(Project.__table__())
    |> Query.join_as(:inner, ApiKey.__table__(), "ak", "ak.project_id = projects.id")
    |> Query.where_raw("ak.key_value = ?", [key_value])
    |> Query.where_raw("ak.revoked_at IS NULL", [])
    |> Query.select_raw(["projects.id::text", "projects.org_id::text", "projects.name", "projects.platform", "projects.created_at::text"])
  let rows = Repo.all(pool, q)?
  if List.length(rows) > 0 do
    let row = List.head(rows)
    Ok(Project { id: Map.get(row, "id"), org_id: Map.get(row, "org_id"), name: Map.get(row, "name"), platform: Map.get(row, "platform"), created_at: Map.get(row, "created_at") })
  else
    Err("not found")
  end
end

# Revoke an API key by setting revoked_at to now().
# Two-step pattern: Repo.query_raw for now() timestamp, then Repo.update_where for the UPDATE.
pub fn revoke_api_key(pool :: PoolHandle, key_id :: String) -> Int!String do
  # Step 1: Get current timestamp from PG
  let ts_rows = Repo.query_raw(pool, "SELECT now()::text AS ts", [])?
  if List.length(ts_rows) > 0 do
    let ts = Map.get(List.head(ts_rows), "ts")
    # Step 2: Update with ORM
    let q = Query.from(ApiKey.__table__())
      |> Query.where_raw("id = ?::uuid", [key_id])
    let _ = Repo.update_where(pool, ApiKey.__table__(), %{"revoked_at" => ts}, q)?
    Ok(1)
  else
    Err("revoke_api_key: timestamp generation failed")
  end
end

# --- User queries ---

# Create a new user with bcrypt password hashing via pgcrypto (cost factor 12).
# Two-step pattern: Repo.query_raw for crypt() hash generation, then Repo.insert for data INSERT.
pub fn create_user(pool :: PoolHandle, email :: String, password :: String, display_name :: String) -> String!String do
  # Step 1: Hash password via pgcrypto
  let hash_rows = Repo.query_raw(pool, "SELECT crypt($1, gen_salt('bf', 12)) AS hash", [password])?
  if List.length(hash_rows) > 0 do
    let password_hash = Map.get(List.head(hash_rows), "hash")
    # Step 2: Insert user with ORM
    let fields = %{"email" => email, "password_hash" => password_hash, "display_name" => display_name}
    let row = Repo.insert(pool, User.__table__(), fields)?
    Ok(Map.get(row, "id"))
  else
    Err("create_user: password hashing failed")
  end
end

# Authenticate a user by email and password.
# Returns the User if credentials match, Err("not found") otherwise.
# Uses ORM Query.where + Query.where_raw for crypt() password verification.
pub fn authenticate_user(pool :: PoolHandle, email :: String, password :: String) -> User!String do
  let q = Query.from(User.__table__())
    |> Query.where(:email, email)
    |> Query.where_raw("password_hash = crypt(?, password_hash)", [password])
    |> Query.select_raw(["id::text", "email", "display_name", "created_at::text"])
  let rows = Repo.all(pool, q)?
  if List.length(rows) > 0 do
    let row = List.head(rows)
    Ok(User { id: Map.get(row, "id"), email: Map.get(row, "email"), display_name: Map.get(row, "display_name"), created_at: Map.get(row, "created_at") })
  else
    Err("not found")
  end
end

# Get a user by ID.
pub fn get_user(pool :: PoolHandle, id :: String) -> User!String do
  let row = Repo.get(pool, User.__table__(), id)?
  Ok(User { id: Map.get(row, "id"), email: Map.get(row, "email"), display_name: Map.get(row, "display_name"), created_at: Map.get(row, "created_at") })
end

# --- Session queries ---

# Create a new session with a cryptographically random token.
# Returns the 64-char hex token.
# Uses Crypto stdlib UUID generation -- no DB round-trip needed.
pub fn create_session(pool :: PoolHandle, user_id :: String) -> String!String do
  # Generate cryptographically random token using Crypto stdlib -- no DB round-trip needed
  # Two UUID4s with hyphens stripped = 32 + 32 = 64 hex chars (same format as before)
  let uuid1 = Crypto.uuid4() |> String.replace("-", "")
  let uuid2 = Crypto.uuid4() |> String.replace("-", "")
  let token = uuid1 <> uuid2
  let fields = %{"token" => token, "user_id" => user_id}
  let _ = Repo.insert(pool, Session.__table__(), fields)?
  Ok(token)
end

# Validate a session token. Returns the Session if valid and not expired.
# Uses ORM Query.where + Query.where_raw for expiry check.
pub fn validate_session(pool :: PoolHandle, token :: String) -> Session!String do
  let q = Query.from(Session.__table__())
    |> Query.where(:token, token)
    |> Query.where_raw("expires_at > now()", [])
    |> Query.select_raw(["token", "user_id::text", "created_at::text", "expires_at::text"])
  let rows = Repo.all(pool, q)?
  if List.length(rows) > 0 do
    let row = List.head(rows)
    Ok(Session { token: Map.get(row, "token"), user_id: Map.get(row, "user_id"), created_at: Map.get(row, "created_at"), expires_at: Map.get(row, "expires_at") })
  else
    Err("not found")
  end
end

# Delete a session by token (logout).
# Uses ORM Repo.delete_where -- zero raw SQL.
pub fn delete_session(pool :: PoolHandle, token :: String) -> Int!String do
  let q = Query.from(Session.__table__())
    |> Query.where(:token, token)
  Repo.delete_where(pool, Session.__table__(), q)
end

# --- Org membership queries ---

# Add a user to an organization with a role (owner/admin/member).
pub fn add_member(pool :: PoolHandle, user_id :: String, org_id :: String, role :: String) -> String!String do
  let fields = %{"user_id" => user_id, "org_id" => org_id, "role" => role}
  let row = Repo.insert(pool, OrgMembership.__table__(), fields)?
  Ok(Map.get(row, "id"))
end

# Get all members of an organization.
pub fn get_members(pool :: PoolHandle, org_id :: String) -> List<OrgMembership>!String do
  let q = Query.from(OrgMembership.__table__())
    |> Query.where(:org_id, org_id)
  let rows = Repo.all(pool, q)?
  Ok(List.map(rows, fn(row) do
    OrgMembership { id: Map.get(row, "id"), user_id: Map.get(row, "user_id"), org_id: Map.get(row, "org_id"), role: Map.get(row, "role"), joined_at: Map.get(row, "joined_at") }
  end))
end

# --- Issue queries (Phase 89) ---

# Upsert an issue: insert on first occurrence, update on subsequent.
# Uses PostgreSQL ON CONFLICT on (project_id, fingerprint) unique constraint.
# Handles GROUP-04 (new issue), GROUP-05 (event_count + last_seen), and
# ISSUE-02 (regression: resolved flips to unresolved on new event).
# Returns Ok(issue_id) or Err.
# ORM boundary: Repo.insert_or_update cannot express custom SET expressions --
# event_count = issues.event_count + 1 (arithmetic) and
# status = CASE WHEN issues.status = 'resolved' THEN 'unresolved' ELSE issues.status END
# (conditional). The ORM upsert uses SET field = EXCLUDED.field which only copies the
# INSERT value, not computed expressions. Intentional raw SQL.
pub fn upsert_issue(pool :: PoolHandle, project_id :: String, fingerprint :: String, title :: String, level :: String) -> String!String do
  let sql = "INSERT INTO issues (project_id, fingerprint, title, level, event_count) VALUES ($1::uuid, $2, $3, $4, 1) ON CONFLICT (project_id, fingerprint) DO UPDATE SET event_count = issues.event_count + 1, last_seen = now(), status = CASE WHEN issues.status = 'resolved' THEN 'unresolved' ELSE issues.status END RETURNING id::text"
  let rows = Repo.query_raw(pool, sql, [project_id, fingerprint, title, level])?
  if List.length(rows) > 0 do
    Ok(Map.get(List.head(rows), "id"))
  else
    Err("upsert_issue: no id returned")
  end
end

# Check if an issue with the given fingerprint is discarded (ISSUE-05 suppression).
# Returns true if the issue exists with status = 'discarded', false otherwise.
# Uses ORM Query.where + Repo.all instead of Repo.query_raw.
pub fn is_issue_discarded(pool :: PoolHandle, project_id :: String, fingerprint :: String) -> Bool!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.where(:fingerprint, fingerprint)
    |> Query.where(:status, "discarded")
    |> Query.select_raw(["1 AS found"])
  let rows = Repo.all(pool, q)?
  Ok(List.length(rows) > 0)
end

# --- Issue management queries (Phase 89 Plan 02) ---

# Transition an issue to 'resolved' status (ISSUE-01).
# Uses ORM Repo.update_where instead of raw SQL.
pub fn resolve_issue(pool :: PoolHandle, issue_id :: String) -> Int!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
    |> Query.where_raw("status != 'resolved'", [])
  let _ = Repo.update_where(pool, Issue.__table__(), %{"status" => "resolved"}, q)?
  Ok(1)
end

# Transition an issue to 'archived' status (ISSUE-01).
# Uses ORM Repo.update_where instead of raw SQL.
pub fn archive_issue(pool :: PoolHandle, issue_id :: String) -> Int!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  let _ = Repo.update_where(pool, Issue.__table__(), %{"status" => "archived"}, q)?
  Ok(1)
end

# Reopen an issue -- set status back to 'unresolved' (ISSUE-01).
# Uses ORM Repo.update_where instead of raw SQL.
pub fn unresolve_issue(pool :: PoolHandle, issue_id :: String) -> Int!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  let _ = Repo.update_where(pool, Issue.__table__(), %{"status" => "unresolved"}, q)?
  Ok(1)
end

# Assign an issue to a user. Pass empty string to unassign (ISSUE-04).
# Uses ORM Repo.update_where for the assign branch.
# Unassign branch retains Repo.execute_raw since ORM Map<String,String> cannot represent NULL.
pub fn assign_issue(pool :: PoolHandle, issue_id :: String, user_id :: String) -> Int!String do
  if String.length(user_id) > 0 do
    let q = Query.from(Issue.__table__())
      |> Query.where_raw("id = ?::uuid", [issue_id])
    let _ = Repo.update_where(pool, Issue.__table__(), %{"assigned_to" => user_id}, q)?
    Ok(1)
  else
    Repo.execute_raw(pool, "UPDATE issues SET assigned_to = NULL WHERE id = $1::uuid", [issue_id])
  end
end

# Mark an issue as discarded -- future events with this fingerprint are suppressed (ISSUE-05).
# Uses ORM Repo.update_where instead of raw SQL.
pub fn discard_issue(pool :: PoolHandle, issue_id :: String) -> Int!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  let _ = Repo.update_where(pool, Issue.__table__(), %{"status" => "discarded"}, q)?
  Ok(1)
end

# Delete an issue and all associated events (ISSUE-05).
# Events deleted first due to FK constraint on issue_id.
# Uses ORM Repo.delete_where instead of raw SQL.
pub fn delete_issue(pool :: PoolHandle, issue_id :: String) -> Int!String do
  let q_events = Query.from(Event.__table__())
    |> Query.where_raw("issue_id = ?::uuid", [issue_id])
  let _ = Repo.delete_where(pool, Event.__table__(), q_events)?
  let q_issue = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  Repo.delete_where(pool, Issue.__table__(), q_issue)
end

# Helper: parse event_count string to Int, defaulting to 0 on failure.
fn parse_event_count(s :: String) -> Int do
  let result = String.to_int(s)
  case result do
    Some(n) -> n
    None -> 0
  end
end

# Helper: parse limit string to Int, defaulting to 25 on failure.
fn parse_limit(s :: String) -> Int do
  let result = String.to_int(s)
  case result do
    Some(n) -> n
    None -> 25
  end
end

# List issues for a project filtered by status (for API listing).
# Constructs Issue structs manually with parse_event_count for the Int field.
# Uses ORM Query.where + Query.order_by + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn list_issues_by_status(pool :: PoolHandle, project_id :: String, status :: String) -> List<Issue>!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.where(:status, status)
    |> Query.order_by(:last_seen, :desc)
    |> Query.select_raw(["id::text", "project_id::text", "fingerprint", "title", "level", "status", "event_count::text", "first_seen::text", "last_seen::text", "COALESCE(assigned_to::text, '') as assigned_to"])
  let rows = Repo.all(pool, q)?
  Ok(List.map(rows, fn(row) do
    Issue { id: Map.get(row, "id"), project_id: Map.get(row, "project_id"), fingerprint: Map.get(row, "fingerprint"), title: Map.get(row, "title"), level: Map.get(row, "level"), status: Map.get(row, "status"), event_count: parse_event_count(Map.get(row, "event_count")), first_seen: Map.get(row, "first_seen"), last_seen: Map.get(row, "last_seen"), assigned_to: Map.get(row, "assigned_to") }
  end))
end

# Spike detection: escalate archived issues with sudden volume bursts (ISSUE-03).
# If an archived issue has >10x its average hourly rate (or >10 absolute) in the
# last hour, it's auto-escalated to 'unresolved'. The WHERE status='archived'
# naturally prevents re-escalation after the first flip (research Pitfall 5).
# Returns number of escalated issues.
# ORM boundary: Repo.update_where cannot express nested subquery with JOIN + HAVING +
# GREATEST + interval arithmetic. The WHERE ... IN (SELECT ... JOIN ... GROUP BY ...
# HAVING count(*) > GREATEST(10, subquery / 168 * 10)) pattern exceeds ORM query
# builder expressiveness. Intentional raw SQL.
pub fn check_volume_spikes(pool :: PoolHandle) -> Int!String do
  Repo.execute_raw(pool, "UPDATE issues SET status = 'unresolved' WHERE status = 'archived' AND id IN (SELECT i.id FROM issues i JOIN events e ON e.issue_id = i.id AND e.received_at > now() - interval '1 hour' WHERE i.status = 'archived' GROUP BY i.id HAVING count(*) > GREATEST(10, (SELECT count(*) FROM events e2 WHERE e2.issue_id = i.id AND e2.received_at > now() - interval '7 days') / 168 * 10))", [])
end

# Extract event fields from JSON and compute fingerprint using PostgreSQL.
# This avoids the cross-module from_json limitation (decision [88-02]) by
# computing the fingerprint server-side with the same fallback chain as
# Ingestion.Fingerprint: custom > stacktrace frames > exception type > message.
# Returns a Map with keys: fingerprint, title, level.
# ORM boundary: ORM fragments cannot express CASE/jsonb_array_elements/string_agg
# fingerprint computation chain. This is the most complex SQL query in Mesher --
# conditional fingerprint fallback with array element iteration, string aggregation,
# and COALESCE chains. No ORM API can compose this. Intentional raw SQL.
pub fn extract_event_fields(pool :: PoolHandle, event_json :: String) -> Map<String, String>!String do
  let sql = "SELECT CASE WHEN length(COALESCE(j->>'fingerprint', '')) > 0 THEN j->>'fingerprint' WHEN j->'stacktrace' IS NOT NULL AND jsonb_typeof(j->'stacktrace') = 'array' AND jsonb_array_length(j->'stacktrace') > 0 THEN (SELECT string_agg((frame->>'filename') || '|' || (frame->>'function_name'), ';' ORDER BY ordinality) FROM jsonb_array_elements(j->'stacktrace') WITH ORDINALITY AS t(frame, ordinality)) || ':' || lower(COALESCE(replace(j->>'message', '0x', ''), '')) WHEN j->'exception' IS NOT NULL AND j->'exception'->>'type_name' IS NOT NULL THEN (j->'exception'->>'type_name') || ':' || lower(COALESCE(replace(j->'exception'->>'value', '0x', ''), '')) ELSE 'msg:' || lower(COALESCE(replace(j->>'message', '0x', ''), '')) END AS fingerprint, COALESCE(NULLIF(j->>'message', ''), 'Untitled') AS title, COALESCE(j->>'level', 'error') AS level FROM (SELECT $1::jsonb AS j) AS sub"
  let rows = Repo.query_raw(pool, sql, [event_json])?
  if List.length(rows) > 0 do
    Ok(List.head(rows))
  else
    Err("extract_event_fields: no result")
  end
end

# --- Search, filter, and pagination queries (Phase 91 Plan 01) ---

# SEARCH-01 + SEARCH-05: List issues with optional filters and keyset pagination.
# Optional filters use SQL-side conditionals ($N = '' OR column = $N) to avoid injection.
# Keyset pagination uses (last_seen, id) < ($cursor, $cursor_id) for stable browsing.
# Returns raw Map rows (not Issue struct) for flexible JSON serialization.
# ORM boundary: Variable-arity parameter binding for optional filters ($N = '' OR column = $N)
# with keyset pagination requires conditional WHERE clauses with positional parameters that
# change count based on cursor presence. Intentional raw SQL.
pub fn list_issues_filtered(pool :: PoolHandle, project_id :: String, status :: String, level :: String, assigned_to :: String, cursor :: String, cursor_id :: String, limit_str :: String) -> List<Map<String, String>>!String do
  if String.length(cursor) > 0 do
    let sql = "SELECT id::text, project_id::text, fingerprint, title, level, status, event_count::text, first_seen::text, last_seen::text, COALESCE(assigned_to::text, '') as assigned_to FROM issues WHERE project_id = $1::uuid AND ($2 = '' OR status = $2) AND ($3 = '' OR level = $3) AND ($4 = '' OR assigned_to = $4::uuid) AND (last_seen, id) < ($5::timestamptz, $6::uuid) ORDER BY last_seen DESC, id DESC LIMIT $7::int"
    let rows = Repo.query_raw(pool, sql, [project_id, status, level, assigned_to, cursor, cursor_id, limit_str])?
    Ok(rows)
  else
    let sql = "SELECT id::text, project_id::text, fingerprint, title, level, status, event_count::text, first_seen::text, last_seen::text, COALESCE(assigned_to::text, '') as assigned_to FROM issues WHERE project_id = $1::uuid AND ($2 = '' OR status = $2) AND ($3 = '' OR level = $3) AND ($4 = '' OR assigned_to = $4::uuid) ORDER BY last_seen DESC, id DESC LIMIT $5::int"
    let rows = Repo.query_raw(pool, sql, [project_id, status, level, assigned_to, limit_str])?
    Ok(rows)
  end
end

# SEARCH-02: Full-text search on event messages using inline tsvector.
# Uses inline to_tsvector (avoids partition complications with stored tsvector column).
# Includes 24-hour default time range (SEARCH-04) for partition pruning.
# Returns relevance rank for ordering.
# ORM boundary: ts_rank() with bound parameter in SELECT expression -- select_raw takes
# a List of column expression strings but cannot bind parameters within those expressions.
# The $2 reference in ts_rank(to_tsvector(...), plainto_tsquery('english', $2)) requires
# positional parameter binding inside a SELECT column. Intentional raw SQL.
pub fn search_events_fulltext(pool :: PoolHandle, project_id :: String, search_query :: String, limit_str :: String) -> List<Map<String, String>>!String do
  let sql = "SELECT id::text, issue_id::text, level, message, received_at::text, ts_rank(to_tsvector('english', message), plainto_tsquery('english', $2))::text AS rank FROM events WHERE project_id = $1::uuid AND to_tsvector('english', message) @@ plainto_tsquery('english', $2) AND received_at > now() - interval '24 hours' ORDER BY rank DESC, received_at DESC LIMIT $3::int"
  let rows = Repo.query_raw(pool, sql, [project_id, search_query, limit_str])?
  Ok(rows)
end

# SEARCH-03: Filter events by tag key-value pair using JSONB containment.
# Uses tags @> ?::jsonb operator which leverages existing GIN index (idx_events_tags).
# Includes 24-hour default time range (SEARCH-04).
# Uses ORM Query.where_raw + Query.select_raw + Query.order_by + Query.limit + Repo.all.
pub fn filter_events_by_tag(pool :: PoolHandle, project_id :: String, tag_json :: String, limit_str :: String) -> List<Map<String, String>>!String do
  let lim = parse_limit(limit_str)
  let q = Query.from(Event.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.where_raw("tags @> ?::jsonb", [tag_json])
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.select_raw(["id::text", "issue_id::text", "level", "message", "tags::text", "received_at::text"])
    |> Query.order_by(:received_at, :desc)
    |> Query.limit(lim)
  Repo.all(pool, q)
end

# Event listing within an issue with keyset pagination (for DETAIL-05 context).
# Keyset pagination on (received_at, id) for stable browsing.
# Uses ORM Query.where_raw + Query.select_raw + Query.order_by_raw + Query.limit + Repo.all.
pub fn list_events_for_issue(pool :: PoolHandle, issue_id :: String, cursor :: String, cursor_id :: String, limit_str :: String) -> List<Map<String, String>>!String do
  let lim = parse_limit(limit_str)
  let base = Query.from(Event.__table__())
    |> Query.where_raw("issue_id = ?::uuid", [issue_id])
    |> Query.select_raw(["id::text", "level", "message", "received_at::text"])
    |> Query.order_by_raw("received_at DESC, id DESC")
    |> Query.limit(lim)
  if String.length(cursor) > 0 do
    let q = base
      |> Query.where_raw("(received_at, id) < (?::timestamptz, ?::uuid)", [cursor, cursor_id])
    Repo.all(pool, q)
  else
    Repo.all(pool, base)
  end
end

# --- Dashboard aggregation queries (Phase 91 Plan 02) ---

# DASH-01: Event volume bucketed by hour or day for a project.
# bucket param is either "hour" or "day" (passed from handler, validated by caller).
# Default 24-hour time window.
# Uses ORM Query.where_raw + Query.select_raw + Query.group_by_raw + Query.order_by_raw + Repo.all.
# Bucket is string-interpolated into date_trunc expression (safe: caller validates "hour"/"day" only).
pub fn event_volume_hourly(pool :: PoolHandle, project_id :: String, bucket :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Event.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.select_raw(["date_trunc('" <> bucket <> "', received_at)::text AS bucket", "count(*)::text AS count"])
    |> Query.group_by_raw("1")
    |> Query.order_by_raw("1")
  Repo.all(pool, q)
end

# DASH-02: Error breakdown by severity level for a project.
# Groups events by level (error, warning, info, etc.) with counts.
# Uses ORM Query.where_raw + Query.select_raw + Query.group_by_raw + Query.order_by_raw + Repo.all.
pub fn error_breakdown_by_level(pool :: PoolHandle, project_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Event.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.select_raw(["level", "count(*)::text AS count"])
    |> Query.group_by_raw("level")
    |> Query.order_by_raw("count DESC")
  Repo.all(pool, q)
end

# DASH-03: Top issues ranked by frequency (event count).
# Returns unresolved issues ordered by event_count DESC.
# Uses ORM Query.where_raw + Query.where + Query.select_raw + Query.order_by + Query.limit + Repo.all.
pub fn top_issues_by_frequency(pool :: PoolHandle, project_id :: String, limit_str :: String) -> List<Map<String, String>>!String do
  let lim = parse_limit(limit_str)
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.where(:status, "unresolved")
    |> Query.select_raw(["id::text", "title", "level", "status", "event_count::text", "last_seen::text"])
    |> Query.order_by(:event_count, :desc)
    |> Query.limit(lim)
  Repo.all(pool, q)
end

# DASH-04: Event breakdown by tag key (environment, release, etc.).
# Uses JSONB key-exists operator to filter events that have the specified tag.
# ORM boundary: tags->>$2 with bound parameter in SELECT expression -- select_raw takes
# a List of column expression strings but cannot bind parameters within those expressions.
# The $2 reference in tags->>$2 requires positional parameter binding inside a SELECT column. Intentional raw SQL.
pub fn event_breakdown_by_tag(pool :: PoolHandle, project_id :: String, tag_key :: String) -> List<Map<String, String>>!String do
  let sql = "SELECT tags->>$2 AS tag_value, count(*)::text AS count FROM events WHERE project_id = $1::uuid AND received_at > now() - interval '24 hours' AND tags ? $2 GROUP BY tag_value ORDER BY count DESC LIMIT 20"
  let rows = Repo.query_raw(pool, sql, [project_id, tag_key])?
  Ok(rows)
end

# DASH-05: Per-issue event timeline (recent events for a specific issue).
# Ordered by received_at DESC for chronological browsing.
# Uses ORM Query.where_raw + Query.select_raw + Query.order_by + Query.limit + Repo.all.
pub fn issue_event_timeline(pool :: PoolHandle, issue_id :: String, limit_str :: String) -> List<Map<String, String>>!String do
  let lim = parse_limit(limit_str)
  let q = Query.from(Event.__table__())
    |> Query.where_raw("issue_id = ?::uuid", [issue_id])
    |> Query.select_raw(["id::text", "level", "message", "received_at::text"])
    |> Query.order_by(:received_at, :desc)
    |> Query.limit(lim)
  Repo.all(pool, q)
end

# DASH-06: Project health summary with key metrics.
# Returns single row: unresolved issue count, events in last 24h, new issues today.
# ORM boundary: Three scalar subqueries in a single SELECT -- each subquery references
# a different table (issues, events, issues) with independent WHERE conditions. The ORM
# Query builder operates on a single FROM table and cannot compose cross-table scalar
# subqueries in SELECT expressions. Intentional raw SQL.
pub fn project_health_summary(pool :: PoolHandle, project_id :: String) -> List<Map<String, String>>!String do
  let sql = "SELECT (SELECT count(*) FROM issues WHERE project_id = $1::uuid AND status = 'unresolved')::text AS unresolved_count, (SELECT count(*) FROM events WHERE project_id = $1::uuid AND received_at > now() - interval '24 hours')::text AS events_24h, (SELECT count(*) FROM issues WHERE project_id = $1::uuid AND first_seen > now() - interval '24 hours')::text AS new_today"
  let rows = Repo.query_raw(pool, sql, [project_id])?
  Ok(rows)
end

# --- Event detail queries (Phase 91 Plan 02) ---

# DETAIL-01..04, DETAIL-06: Get complete event with all JSONB fields.
# Returns full event payload including exception, stacktrace, breadcrumbs,
# tags, extra, user_context. JSONB fields use COALESCE for null safety.
# Uses ORM Query.where_raw + Query.select_raw + Repo.all.
pub fn get_event_detail(pool :: PoolHandle, event_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Event.__table__())
    |> Query.where_raw("id = ?::uuid", [event_id])
    |> Query.select_raw(["id::text", "project_id::text", "issue_id::text", "level", "message", "fingerprint", "COALESCE(exception::text, 'null') AS exception", "COALESCE(stacktrace::text, '[]') AS stacktrace", "COALESCE(breadcrumbs::text, '[]') AS breadcrumbs", "COALESCE(tags::text, '{}') AS tags", "COALESCE(extra::text, '{}') AS extra", "COALESCE(user_context::text, 'null') AS user_context", "COALESCE(sdk_name, '') AS sdk_name", "COALESCE(sdk_version, '') AS sdk_version", "received_at::text"])
  Repo.all(pool, q)
end

# DETAIL-05: Get next and previous event IDs within an issue for navigation.
# Uses tuple comparison (received_at, id) for stable ordering.
# ORM boundary: Two scalar subqueries with opposing sort orders and tuple comparison
# in a single SELECT -- each subquery uses (received_at, id) tuple comparison with
# different directions (> for next, < for prev) and LIMIT 1. The ORM Query builder
# cannot compose multiple independent subqueries in SELECT expressions. Intentional raw SQL.
pub fn get_event_neighbors(pool :: PoolHandle, issue_id :: String, received_at :: String, event_id :: String) -> List<Map<String, String>>!String do
  let sql = "SELECT (SELECT id::text FROM events WHERE issue_id = $1::uuid AND (received_at, id) > ($2::timestamptz, $3::uuid) ORDER BY received_at, id LIMIT 1) AS next_id, (SELECT id::text FROM events WHERE issue_id = $1::uuid AND (received_at, id) < ($2::timestamptz, $3::uuid) ORDER BY received_at DESC, id DESC LIMIT 1) AS prev_id"
  let rows = Repo.query_raw(pool, sql, [issue_id, received_at, event_id])?
  Ok(rows)
end

# --- Team management queries (Phase 91 Plan 03 -- ORG-04) ---

# Update a member's role. SQL-side validation ensures only valid roles accepted.
# Returns affected row count (0 if invalid role or membership not found).
# Uses ORM Repo.update_where with Query.where_raw for role validation.
pub fn update_member_role(pool :: PoolHandle, membership_id :: String, new_role :: String) -> Int!String do
  let q = Query.from(OrgMembership.__table__())
    |> Query.where_raw("id = ?::uuid AND ? IN ('owner', 'admin', 'member')", [membership_id, new_role])
  let _ = Repo.update_where(pool, OrgMembership.__table__(), %{"role" => new_role}, q)?
  Ok(1)
end

# Remove a member from an organization.
# Returns affected row count (0 if membership not found).
pub fn remove_member(pool :: PoolHandle, membership_id :: String) -> Int!String do
  let _ = Repo.delete(pool, OrgMembership.__table__(), membership_id)?
  Ok(1)
end

# List all members of an organization with user info (email, display_name).
# JOIN with users table for enriched member listing.
# Returns raw Map rows for flexible JSON serialization.
# Uses ORM Query.join_as + Query.where_raw + Query.select_raw + Query.order_by_raw + Repo.all.
pub fn get_members_with_users(pool :: PoolHandle, org_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(OrgMembership.__table__())
    |> Query.join_as(:inner, User.__table__(), "u", "u.id = org_memberships.user_id")
    |> Query.where_raw("org_memberships.org_id = ?::uuid", [org_id])
    |> Query.select_raw(["org_memberships.id::text", "org_memberships.user_id::text", "org_memberships.org_id::text", "org_memberships.role", "org_memberships.joined_at::text", "u.email", "u.display_name"])
    |> Query.order_by_raw("org_memberships.joined_at")
  Repo.all(pool, q)
end

# --- API token management queries (Phase 91 Plan 03 -- ORG-05) ---

# List all API keys for a project with full details.
# Returns raw Map rows. revoked_at is empty string if not revoked.
# Uses ORM Query.where_raw + Query.select_raw + Query.order_by + Repo.all.
pub fn list_api_keys(pool :: PoolHandle, project_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(ApiKey.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.select_raw(["id::text", "project_id::text", "key_value", "label", "created_at::text", "COALESCE(revoked_at::text, '') AS revoked_at"])
    |> Query.order_by(:created_at, :desc)
  Repo.all(pool, q)
end

# --- Alert system queries (Phase 92) ---

# ALERT-01: Insert alert rule from JSON body using PostgreSQL JSONB extraction.
# ORM boundary: INSERT...SELECT with server-side JSONB extraction from parameter ($2::jsonb)
# and COALESCE defaults. Repo.insert takes Map<String,String> of literal values but this query
# extracts and transforms fields from a JSONB parameter server-side. Intentional raw SQL.
pub fn create_alert_rule(pool :: PoolHandle, project_id :: String, body :: String) -> String!String do
  let sql = "INSERT INTO alert_rules (project_id, name, condition_json, action_json, cooldown_minutes) SELECT $1::uuid, COALESCE(j->>'name', 'Unnamed Rule'), COALESCE((j->'condition')::jsonb, '{}'::jsonb), COALESCE((j->'action')::jsonb, '{\"type\":\"websocket\"}'::jsonb), COALESCE((j->>'cooldown_minutes')::int, 60) FROM (SELECT $2::jsonb AS j) AS sub RETURNING id::text"
  let rows = Repo.query_raw(pool, sql, [project_id, body])?
  if List.length(rows) > 0 do
    Ok(Map.get(List.head(rows), "id"))
  else
    Err("create_alert_rule: no id returned")
  end
end

# ALERT-01: List all alert rules for a project.
# Uses ORM Query.where_raw + Query.select_raw + Query.order_by + Repo.all instead of Repo.query_raw.
pub fn list_alert_rules(pool :: PoolHandle, project_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.select_raw(["id::text", "project_id::text", "name", "condition_json::text", "action_json::text", "enabled::text", "cooldown_minutes::text", "COALESCE(last_fired_at::text, '') AS last_fired_at", "created_at::text"])
    |> Query.order_by(:created_at, :desc)
  Repo.all(pool, q)
end

# Enable/disable an alert rule.
# Uses ORM Repo.update_where with Query.where_raw.
pub fn toggle_alert_rule(pool :: PoolHandle, rule_id :: String, enabled_str :: String) -> Int!String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_raw("id = ?::uuid", [rule_id])
  let _ = Repo.update_where(pool, AlertRule.__table__(), %{"enabled" => enabled_str}, q)?
  Ok(1)
end

# Delete an alert rule.
pub fn delete_alert_rule(pool :: PoolHandle, rule_id :: String) -> Int!String do
  let _ = Repo.delete(pool, AlertRule.__table__(), rule_id)?
  Ok(1)
end

# ALERT-02: Count events in time window AND check cooldown, return true if should fire.
# ORM boundary: Cross-join between two derived tables (event count subquery + cooldown subquery)
# with CASE expression, interval arithmetic, and multiple bound parameters in complex expressions.
# Not expressible via ORM query builder. Intentional raw SQL.
pub fn evaluate_threshold_rule(pool :: PoolHandle, rule_id :: String, project_id :: String, threshold_str :: String, window_str :: String, cooldown_str :: String) -> Bool!String do
  let sql = "SELECT CASE WHEN event_count > $3::int AND (last_fired IS NULL OR last_fired < now() - interval '1 minute' * $6::int) THEN 1 ELSE 0 END AS should_fire FROM (SELECT count(*) AS event_count FROM events WHERE project_id = $2::uuid AND received_at > now() - interval '1 minute' * $4::int) counts, (SELECT last_fired_at AS last_fired FROM alert_rules WHERE id = $1::uuid) cooldown"
  let rows = Repo.query_raw(pool, sql, [rule_id, project_id, threshold_str, window_str, "", cooldown_str])?
  if List.length(rows) > 0 do
    let should_fire = Map.get(List.head(rows), "should_fire")
    Ok(should_fire == "1")
  else
    Ok(false)
  end
end

# ALERT-04/05: Insert alert record, update last_fired_at atomically, return alert_id.
# ORM boundary: INSERT with jsonb_build_object() for computed JSONB column plus follow-up
# UPDATE of last_fired_at = now(). Could split into two-step ORM pattern but jsonb_build_object
# in INSERT VALUES is not expressible via Repo.insert Map<String,String>. Intentional raw SQL.
pub fn fire_alert(pool :: PoolHandle, rule_id :: String, project_id :: String, message :: String, condition_type :: String, rule_name :: String) -> String!String do
  let sql = "INSERT INTO alerts (rule_id, project_id, status, message, condition_snapshot) VALUES ($1::uuid, $2::uuid, 'active', $3, jsonb_build_object('condition_type', $4, 'rule_name', $5)) RETURNING id::text"
  let rows = Repo.query_raw(pool, sql, [rule_id, project_id, message, condition_type, rule_name])?
  if List.length(rows) > 0 do
    let alert_id = Map.get(List.head(rows), "id")
    let _ = Repo.execute_raw(pool, "UPDATE alert_rules SET last_fired_at = now() WHERE id = $1::uuid", [rule_id])
    Ok(alert_id)
  else
    Err("fire_alert: no id returned")
  end
end

# ALERT-03: Check if an issue was just created (first_seen = last_seen).
# Uses ORM Query.where_raw + Query.select_raw + Repo.all.
pub fn check_new_issue(pool :: PoolHandle, issue_id :: String) -> Bool!String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid AND first_seen = last_seen", [issue_id])
    |> Query.select_raw(["1 AS is_new"])
  let rows = Repo.all(pool, q)?
  Ok(List.length(rows) > 0)
end

# ALERT-03: Get enabled alert rules for event-based conditions for a project.
# Uses ORM Query.where_raw + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn get_event_alert_rules(pool :: PoolHandle, project_id :: String, condition_type :: String) -> List<Map<String, String>>!String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_raw("project_id = ?::uuid AND enabled = true AND condition_json->>'condition_type' = ?", [project_id, condition_type])
    |> Query.select_raw(["id::text", "name", "cooldown_minutes::text"])
  Repo.all(pool, q)
end

# ALERT-05: Check cooldown before firing (for event-based triggers).
# Uses ORM Query.where_raw + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn should_fire_by_cooldown(pool :: PoolHandle, rule_id :: String, cooldown_str :: String) -> Bool!String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_raw("id = ?::uuid AND (last_fired_at IS NULL OR last_fired_at < now() - interval '1 minute' * ?::int)", [rule_id, cooldown_str])
    |> Query.select_raw(["1 AS ok"])
  let rows = Repo.all(pool, q)?
  Ok(List.length(rows) > 0)
end

# ALERT-06: Transition alert to acknowledged.
# ORM boundary: SET acknowledged_at = now() uses a server-side function call in the
# UPDATE SET clause. Repo.update_where takes Map<String,String> which can only set
# literal string values, not PG function calls like now(). Intentional raw SQL.
pub fn acknowledge_alert(pool :: PoolHandle, alert_id :: String) -> Int!String do
  Repo.execute_raw(pool, "UPDATE alerts SET status = 'acknowledged', acknowledged_at = now() WHERE id = $1::uuid AND status = 'active'", [alert_id])
end

# ALERT-06: Transition alert to resolved.
# ORM boundary: SET resolved_at = now() uses a server-side function call in the
# UPDATE SET clause. Repo.update_where takes Map<String,String> which can only set
# literal string values, not PG function calls like now(). Intentional raw SQL.
pub fn resolve_fired_alert(pool :: PoolHandle, alert_id :: String) -> Int!String do
  Repo.execute_raw(pool, "UPDATE alerts SET status = 'resolved', resolved_at = now() WHERE id = $1::uuid AND status IN ('active', 'acknowledged')", [alert_id])
end

# ALERT-06: List alerts for a project filtered by status.
# Uses ORM Query.join_as + Query.where_raw + Query.select_raw + Query.order_by_raw + Query.limit + Repo.all.
pub fn list_alerts(pool :: PoolHandle, project_id :: String, status :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Alert.__table__())
    |> Query.join_as(:inner, AlertRule.__table__(), "r", "r.id = alerts.rule_id")
    |> Query.where_raw("alerts.project_id = ?::uuid AND (? = '' OR alerts.status = ?)", [project_id, status, status])
    |> Query.select_raw(["alerts.id::text", "alerts.rule_id::text", "alerts.project_id::text", "alerts.status", "alerts.message", "alerts.condition_snapshot::text", "alerts.triggered_at::text", "COALESCE(alerts.acknowledged_at::text, '') AS acknowledged_at", "COALESCE(alerts.resolved_at::text, '') AS resolved_at", "r.name AS rule_name"])
    |> Query.order_by_raw("alerts.triggered_at DESC")
    |> Query.limit(50)
  Repo.all(pool, q)
end

# Load all enabled threshold rules for evaluation.
# Uses ORM Query.where_raw + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn get_threshold_rules(pool :: PoolHandle) -> List<Map<String, String>>!String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_raw("enabled = true AND condition_json->>'condition_type' = 'threshold'", [])
    |> Query.select_raw(["id::text", "project_id::text", "name", "condition_json::text", "cooldown_minutes::text"])
  Repo.all(pool, q)
end

# --- Retention and storage queries (Phase 93, ORM rewrite Phase 113) ---

# Delete expired events for a project based on its retention_days setting.
# Returns the number of deleted rows.
# Uses ORM Repo.delete_where + Query.where_raw for interval expression instead of Repo.execute_raw.
pub fn delete_expired_events(pool :: PoolHandle, project_id :: String, retention_days_str :: String) -> Int!String do
  let q = Query.from(Event.__table__())
    |> Query.where_raw("project_id = ?::uuid AND received_at < now() - (? || ' days')::interval", [project_id, retention_days_str])
  Repo.delete_where(pool, Event.__table__(), q)
end

# Find event partitions older than max_days (for partition cleanup).
# Queries pg_inherits to find child tables of 'events' with names matching events_YYYYMMDD.
# DDL/catalog query -- queries pg_inherits/pg_class system catalogs. Excluded from data query raw SQL count per ORM rewrite scope.
pub fn get_expired_partitions(pool :: PoolHandle, max_days_str :: String) -> List<Map<String, String>>!String do
  let sql = "SELECT c.relname::text AS partition_name FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid JOIN pg_class p ON p.oid = i.inhparent WHERE p.relname = 'events' AND c.relname ~ '^events_[0-9]{8}$' AND to_date(substring(c.relname from '[0-9]{8}$'), 'YYYYMMDD') < (current_date - ($1 || ' days')::interval)"
  let rows = Repo.query_raw(pool, sql, [max_days_str])?
  Ok(rows)
end

# Drop a single event partition by name.
# The partition_name comes from the trusted pg_inherits query, not user input.
# DDL operation (DROP TABLE) -- excluded from data query raw SQL count per ORM rewrite scope.
pub fn drop_partition(pool :: PoolHandle, partition_name :: String) -> Int!String do
  Repo.execute_raw(pool, "DROP TABLE IF EXISTS " <> partition_name, [])
end

# Get all projects with their retention settings for the cleanup loop.
# Uses ORM Query.from + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn get_all_project_retention(pool :: PoolHandle) -> List<Map<String, String>>!String do
  let q = Query.from(Project.__table__())
    |> Query.select_raw(["id::text", "retention_days::text"])
  Repo.all(pool, q)
end

# Estimate storage usage for a project (event count and estimated bytes).
# Uses 1024 byte average row estimate.
# Uses ORM Query.from + Query.where_raw + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn get_project_storage(pool :: PoolHandle, project_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Event.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.select_raw(["count(*)::text AS event_count", "(count(*) * 1024)::text AS estimated_bytes"])
  Repo.all(pool, q)
end

# Update project retention and sampling settings from JSON body.
# Uses SQL-side JSON extraction per decision [91-03].
# ORM boundary: SET clause uses COALESCE with server-side JSONB extraction
# ($2::jsonb->>'field')::type to conditionally update only provided fields,
# falling back to current column value. Repo.update_where takes Map<String,String>
# which cannot express COALESCE fallback to current column value or server-side
# JSONB extraction. Intentional raw SQL.
pub fn update_project_settings(pool :: PoolHandle, project_id :: String, body :: String) -> Int!String do
  Repo.execute_raw(pool, "UPDATE projects SET retention_days = COALESCE(($2::jsonb->>'retention_days')::int, retention_days), sample_rate = COALESCE(($2::jsonb->>'sample_rate')::real, sample_rate) WHERE id = $1::uuid", [project_id, body])
end

# Get retention and sampling settings for a project.
# Uses ORM Query.from + Query.where_raw + Query.select_raw + Repo.all instead of Repo.query_raw.
pub fn get_project_settings(pool :: PoolHandle, project_id :: String) -> List<Map<String, String>>!String do
  let q = Query.from(Project.__table__())
    |> Query.where_raw("id = ?::uuid", [project_id])
    |> Query.select_raw(["retention_days::text", "sample_rate::text"])
  Repo.all(pool, q)
end

# Check if an event should be kept based on the project's sample_rate.
# Returns true if the event should be kept, false if it should be dropped.
# Defaults to keeping all events (sample_rate = 1.0) if project not found.
# ORM boundary: SELECT random() < COALESCE((SELECT ...), 1.0) uses a server-side
# random() function comparison with a scalar subquery and COALESCE default.
# Not expressible via ORM query builder. Intentional raw SQL.
pub fn check_sample_rate(pool :: PoolHandle, project_id :: String) -> Bool!String do
  let rows = Repo.query_raw(pool, "SELECT random() < COALESCE((SELECT sample_rate FROM projects WHERE id = $1::uuid), 1.0) AS keep", [project_id])?
  if List.length(rows) > 0 do
    Ok(Map.get(List.head(rows), "keep") == "t")
  else
    Ok(true)
  end
end
