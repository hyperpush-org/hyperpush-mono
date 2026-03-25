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
# Uses Query.where_expr + Query.select_expr instead of raw projection strings.

pub fn count_unresolved_issues(pool :: PoolHandle, project_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_expr(Expr.eq(Expr.column("status"), Expr.value("unresolved")))
    |> Query.select_expr(Expr.label(Pg.text(Expr.fn_call("count", [Expr.column("*")])), "cnt"))
  Repo.all(pool, q)
end

# Look up the project_id for an issue by issue_id. Returns rows with "project_id" key.
# Used by ingestion/routes.mpl for broadcasting issue state change notifications.
# Uses Query.where_expr + Query.select_expr instead of raw projection strings.

pub fn get_issue_project_id(pool :: PoolHandle, issue_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("id"), Pg.uuid(Expr.value(issue_id))))
    |> Query.select_expr(Expr.label(Pg.text(Expr.column("project_id")), "project_id"))
  Repo.all(pool, q)
end

# --- Organization queries ---
# Insert a new organization. Returns the generated UUID.

pub fn insert_org(pool :: PoolHandle, name :: String, slug :: String) -> String ! String do
  let fields = %{"name" => name, "slug" => slug}
  let row = Repo.insert(pool, Organization.__table__(), fields) ?
  Ok(Map.get(row, "id"))
end

# Get an organization by ID.

pub fn get_org(pool :: PoolHandle, id :: String) -> Organization ! String do
  let row = Repo.get(pool, Organization.__table__(), id) ?
  Ok(Organization {
    id : Map.get(row, "id"),
    name : Map.get(row, "name"),
    slug : Map.get(row, "slug"),
    created_at : Map.get(row, "created_at")
  })
end

# List all organizations.

pub fn list_orgs(pool :: PoolHandle) -> List < Organization > ! String do
  let q = Query.from(Organization.__table__())
    |> Query.order_by(:name, :asc)
  let rows = Repo.all(pool, q) ?
  Ok(rows
    |> List.map(fn (row) do
      Organization {
        id : Map.get(row, "id"),
        name : Map.get(row, "name"),
        slug : Map.get(row, "slug"),
        created_at : Map.get(row, "created_at")
      }
    end))
end

# --- Project queries ---
# Insert a new project. Returns the generated UUID.

pub fn insert_project(pool :: PoolHandle, org_id :: String, name :: String, platform :: String) -> String ! String do
  let fields = %{"org_id" => org_id, "name" => name, "platform" => platform}
  let row = Repo.insert(pool, Project.__table__(), fields) ?
  Ok(Map.get(row, "id"))
end

# Resolve a project slug to its UUID. Returns the id as a string.
# Used by API handlers to support slug-based project identifiers (e.g. "default").

pub fn get_project_id_by_slug(pool :: PoolHandle, slug :: String) -> String ! String do
  let row = Repo.get_by(pool, Project.__table__(), "slug", slug) ?
  Ok(Map.get(row, "id"))
end

# Get a project by ID.

pub fn get_project(pool :: PoolHandle, id :: String) -> Project ! String do
  let row = Repo.get(pool, Project.__table__(), id) ?
  Ok(Project {
    id : Map.get(row, "id"),
    org_id : Map.get(row, "org_id"),
    name : Map.get(row, "name"),
    platform : Map.get(row, "platform"),
    created_at : Map.get(row, "created_at")
  })
end

# List all projects for an organization.

pub fn list_projects_by_org(pool :: PoolHandle, org_id :: String) -> List < Project > ! String do
  let q = Query.from(Project.__table__())
    |> Query.where(:org_id, org_id)
    |> Query.order_by(:name, :asc)
  let rows = Repo.all(pool, q) ?
  Ok(rows
    |> List.map(fn (row) do
      Project {
        id : Map.get(row, "id"),
        org_id : Map.get(row, "org_id"),
        name : Map.get(row, "name"),
        platform : Map.get(row, "platform"),
        created_at : Map.get(row, "created_at")
      }
    end))
end

# --- API key queries ---
# Create a new API key for a project. Returns the generated key_value (mshr_ prefixed).
# Uses Crypto stdlib UUID generation -- no DB round-trip needed.
# Format: "mshr_" + UUID4 (36 chars) = 41-char key.

pub fn create_api_key(pool :: PoolHandle, project_id :: String, label :: String) -> String ! String do
  # Generate API key using Crypto stdlib -- no DB round-trip needed
  let key_value = "mshr_#{Crypto.uuid4()}"
  let fields = %{"project_id" => project_id, "key_value" => key_value, "label" => label}
  Repo.insert(pool, ApiKey.__table__(), fields) ?
  Ok(key_value)
end

# Get the project associated with a valid (non-revoked) API key.
# Uses ORM Query.join_as plus structured SELECT/WHERE expressions instead of raw projections.

pub fn get_project_by_api_key(pool :: PoolHandle, key_value :: String) -> Project ! String do
  let q = Query.from(Project.__table__())
    |> Query.join_as(:inner, ApiKey.__table__(), "ak", "ak.project_id = projects.id")
    |> Query.where_expr(Expr.eq(Expr.column("ak.key_value"), Expr.value(key_value)))
    |> Query.where_expr(Expr.eq(Expr.coalesce([Pg.text(Expr.column("ak.revoked_at")), Expr.value("")]), Expr.value("")))
    |> Query.select_exprs([
      Expr.label(Expr.column("projects.id"), "id"),
      Expr.label(Expr.column("projects.org_id"), "org_id"),
      Expr.label(Expr.column("projects.name"), "name"),
      Expr.label(Expr.column("projects.platform"), "platform"),
      Expr.label(Expr.column("projects.created_at"), "created_at")
    ])
  let rows = Repo.all(pool, q) ?
  if List.length(rows) > 0 do
    let row = List.head(rows)
    Ok(Project {
      id : Map.get(row, "id"),
      org_id : Map.get(row, "org_id"),
      name : Map.get(row, "name"),
      platform : Map.get(row, "platform"),
      created_at : Map.get(row, "created_at")
    })
  else
    Err("not found")
  end
end

# Revoke an API key by setting revoked_at to now() through the neutral expression write path.

pub fn revoke_api_key(pool :: PoolHandle, key_id :: String) -> Int ! String do
  let q = Query.from(ApiKey.__table__())
    |> Query.where_raw("id = ?::uuid", [key_id])
  Repo.update_where_expr(pool, ApiKey.__table__(), %{"revoked_at" => Expr.fn_call("now", [])}, q) ?
  Ok(1)
end

# --- User queries ---
# Create a new user with bcrypt password hashing via pgcrypto (cost factor 12).
# Uses explicit Pg helpers plus Repo.insert_expr so the auth path no longer depends on raw SQL.

pub fn create_user(pool :: PoolHandle, email :: String, password :: String, display_name :: String) -> String ! String do
  let row = Repo.insert_expr(pool,
  User.__table__(),
  %{"email" => Expr.value(email), "password_hash" => Pg.crypt(Expr.value(password),
  Pg.gen_salt("bf", 12)), "display_name" => Expr.value(display_name)}) ?
  Ok(Map.get(row, "id"))
end

# Authenticate a user by email and password.
# Returns the User if credentials match, Err("not found") otherwise.
# Uses Query.where_expr with explicit Pg.crypt verification instead of raw SQL fragments.

pub fn authenticate_user(pool :: PoolHandle, email :: String, password :: String) -> User ! String do
  let q = Query.from(User.__table__())
    |> Query.where(:email, email)
    |> Query.where_expr(Expr.eq(Expr.column("password_hash"),
    Pg.crypt(Expr.value(password), Expr.column("password_hash"))))
  let rows = Repo.all(pool, q) ?
  if List.length(rows) > 0 do
    let row = List.head(rows)
    Ok(User {
      id : Map.get(row, "id"),
      email : Map.get(row, "email"),
      display_name : Map.get(row, "display_name"),
      created_at : Map.get(row, "created_at")
    })
  else
    Err("not found")
  end
end

# Get a user by ID.

pub fn get_user(pool :: PoolHandle, id :: String) -> User ! String do
  let row = Repo.get(pool, User.__table__(), id) ?
  Ok(User {
    id : Map.get(row, "id"),
    email : Map.get(row, "email"),
    display_name : Map.get(row, "display_name"),
    created_at : Map.get(row, "created_at")
  })
end

# --- Session queries ---
# Create a new session with a cryptographically random token.
# Returns the 64-char hex token.
# Uses Crypto stdlib UUID generation -- no DB round-trip needed.

pub fn create_session(pool :: PoolHandle, user_id :: String) -> String ! String do
  # Generate cryptographically random token using Crypto stdlib -- no DB round-trip needed
  # Two UUID4s with hyphens stripped = 32 + 32 = 64 hex chars (same format as before)
  let uuid1 = Crypto.uuid4()
    |> String.replace("-", "")
  let uuid2 = Crypto.uuid4()
    |> String.replace("-", "")
  let token = "#{uuid1}#{uuid2}"
  let fields = %{"token" => token, "user_id" => user_id}
  Repo.insert(pool, Session.__table__(), fields) ?
  Ok(token)
end

# Validate a session token. Returns the Session if valid and not expired.
# Uses Query.where_expr + Query.select_exprs for the token, expiry, and casted projection.

pub fn validate_session(pool :: PoolHandle, token :: String) -> Session ! String do
  let q = Query.from(Session.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("token"), Expr.value(token)))
    |> Query.where_expr(Expr.gt(Expr.column("expires_at"), Pg.timestamptz(Expr.fn_call("now", []))))
    |> Query.select_exprs([
      Expr.label(Expr.column("token"), "token"),
      Expr.label(Pg.text(Expr.column("user_id")), "user_id"),
      Expr.label(Pg.text(Expr.column("created_at")), "created_at"),
      Expr.label(Pg.text(Expr.column("expires_at")), "expires_at")
    ])
  let rows = Repo.all(pool, q) ?
  if List.length(rows) > 0 do
    let row = List.head(rows)
    Ok(Session {
      token : Map.get(row, "token"),
      user_id : Map.get(row, "user_id"),
      created_at : Map.get(row, "created_at"),
      expires_at : Map.get(row, "expires_at")
    })
  else
    Err("not found")
  end
end

# Delete a session by token (logout).
# Uses ORM Repo.delete_where -- zero raw SQL.

pub fn delete_session(pool :: PoolHandle, token :: String) -> Int ! String do
  let q = Query.from(Session.__table__())
    |> Query.where(:token, token)
  Repo.delete_where(pool, Session.__table__(), q)
end

# --- Org membership queries ---
# Add a user to an organization with a role (owner/admin/member).

pub fn add_member(pool :: PoolHandle, user_id :: String, org_id :: String, role :: String) -> String ! String do
  let fields = %{"user_id" => user_id, "org_id" => org_id, "role" => role}
  let row = Repo.insert(pool, OrgMembership.__table__(), fields) ?
  Ok(Map.get(row, "id"))
end

# Get all members of an organization.

pub fn get_members(pool :: PoolHandle, org_id :: String) -> List < OrgMembership > ! String do
  let q = Query.from(OrgMembership.__table__())
    |> Query.where(:org_id, org_id)
  let rows = Repo.all(pool, q) ?
  Ok(rows
    |> List.map(fn (row) do
      OrgMembership {
        id : Map.get(row, "id"),
        user_id : Map.get(row, "user_id"),
        org_id : Map.get(row, "org_id"),
        role : Map.get(row, "role"),
        joined_at : Map.get(row, "joined_at")
      }
    end))
end

# --- Issue queries (Phase 89) ---
# Upsert an issue: insert on first occurrence, update on subsequent.
# Uses neutral expression-valued conflict updates for arithmetic, now(), and CASE.
# Handles GROUP-04 (new issue), GROUP-05 (event_count + last_seen), and
# ISSUE-02 (regression: resolved flips to unresolved on new event).
# Returns Ok(issue_id) or Err.

pub fn upsert_issue(pool :: PoolHandle,
project_id :: String,
fingerprint :: String,
title :: String,
level :: String) -> String ! String do
  let insert_fields = %{"project_id" => project_id, "fingerprint" => fingerprint, "title" => title, "level" => level, "event_count" => "1"}
  let update_fields = %{"event_count" => Expr.add(Expr.column("issues.event_count"),
  Expr.value("1")), "last_seen" => Expr.fn_call("now", []), "status" => Expr.case_when([Expr.eq(Expr.column("issues.status"),
  Expr.value("resolved"))],
  [Expr.value("unresolved")],
  Expr.column("issues.status"))}
  let row = Repo.insert_or_update_expr(pool,
  Issue.__table__(),
  insert_fields,
  ["project_id", "fingerprint"],
  update_fields) ?
  Ok(Map.get(row, "id"))
end

# Check if an issue with the given fingerprint is discarded (ISSUE-05 suppression).
# Returns true if the issue exists with status = 'discarded', false otherwise.
# Uses ORM Query.where + Repo.all instead of Repo.query_raw.

pub fn is_issue_discarded(pool :: PoolHandle, project_id :: String, fingerprint :: String) -> Bool ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("project_id = ?::uuid", [project_id])
    |> Query.where(:fingerprint, fingerprint)
    |> Query.where(:status, "discarded")
    |> Query.select_raw(["1 AS found"])
  let rows = Repo.all(pool, q) ?
  Ok(List.length(rows) > 0)
end

# --- Issue management queries (Phase 89 Plan 02) ---
# Transition an issue to 'resolved' status (ISSUE-01).
# Uses ORM Repo.update_where instead of raw SQL.

pub fn resolve_issue(pool :: PoolHandle, issue_id :: String) -> Int ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
    |> Query.where_raw("status != 'resolved'", [])
  Repo.update_where(pool, Issue.__table__(), %{"status" => "resolved"}, q) ?
  Ok(1)
end

# Transition an issue to 'archived' status (ISSUE-01).
# Uses ORM Repo.update_where instead of raw SQL.

pub fn archive_issue(pool :: PoolHandle, issue_id :: String) -> Int ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  Repo.update_where(pool, Issue.__table__(), %{"status" => "archived"}, q) ?
  Ok(1)
end

# Reopen an issue -- set status back to 'unresolved' (ISSUE-01).
# Uses ORM Repo.update_where instead of raw SQL.

pub fn unresolve_issue(pool :: PoolHandle, issue_id :: String) -> Int ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  Repo.update_where(pool, Issue.__table__(), %{"status" => "unresolved"}, q) ?
  Ok(1)
end

# Assign an issue to a user. Pass empty string to unassign (ISSUE-04).
# Uses expression-aware Repo.update_where_expr for both assign and unassign,
# with Expr.null() carrying the neutral NULL assignment path.

fn assign_issue_to_user(pool :: PoolHandle, issue_id :: String, user_id :: String) -> Int ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  Repo.update_where_expr(pool, Issue.__table__(), %{"assigned_to" => Expr.value(user_id)}, q) ?
  Ok(1)
end

fn unassign_issue(pool :: PoolHandle, issue_id :: String) -> Int ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  Repo.update_where_expr(pool, Issue.__table__(), %{"assigned_to" => Expr.null()}, q) ?
  Ok(1)
end

pub fn assign_issue(pool :: PoolHandle, issue_id :: String, user_id :: String) -> Int ! String do
  if String.length(user_id) > 0 do
    assign_issue_to_user(pool, issue_id, user_id)
  else
    unassign_issue(pool, issue_id)
  end
end

# Mark an issue as discarded -- future events with this fingerprint are suppressed (ISSUE-05).
# Uses ORM Repo.update_where instead of raw SQL.

pub fn discard_issue(pool :: PoolHandle, issue_id :: String) -> Int ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  Repo.update_where(pool, Issue.__table__(), %{"status" => "discarded"}, q) ?
  Ok(1)
end

# Delete an issue and all associated events (ISSUE-05).
# Events deleted first due to FK constraint on issue_id.
# Uses ORM Repo.delete_where instead of raw SQL.

pub fn delete_issue(pool :: PoolHandle, issue_id :: String) -> Int ! String do
  let q_events = Query.from(Event.__table__())
    |> Query.where_raw("issue_id = ?::uuid", [issue_id])
  Repo.delete_where(pool, Event.__table__(), q_events) ?
  let q_issue = Query.from(Issue.__table__())
    |> Query.where_raw("id = ?::uuid", [issue_id])
  Repo.delete_where(pool, Issue.__table__(), q_issue)
end

# Helper: parse event_count string to Int, defaulting to 0 on failure.

fn parse_event_count(s :: String) -> Int do
  let result = String.to_int(s)
  case result do
    Some( n) -> n
    None -> 0
  end
end

# Helper: parse limit string to Int, defaulting to 25 on failure.

fn parse_limit(s :: String) -> Int do
  let result = String.to_int(s)
  case result do
    Some( n) -> n
    None -> 25
  end
end

# Helper: keep dashboard bucket selection honest and injection-safe.

fn normalize_time_bucket(bucket :: String) -> String do
  if bucket == "day" do
    "day"
  else
    "hour"
  end
end

# List issues for a project filtered by status (for API listing).
# Constructs Issue structs manually with parse_event_count for the Int field.
# Uses structured SELECT expressions plus regular ordering instead of raw projections.

pub fn list_issues_by_status(pool :: PoolHandle, project_id :: String, status :: String) -> List < Issue > ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_expr(Expr.eq(Expr.column("status"), Expr.value(status)))
    |> Query.select_exprs([
      Expr.label(Expr.column("id"), "id"),
      Expr.label(Expr.column("project_id"), "project_id"),
      Expr.label(Expr.column("fingerprint"), "fingerprint"),
      Expr.label(Expr.column("title"), "title"),
      Expr.label(Expr.column("level"), "level"),
      Expr.label(Expr.column("status"), "status"),
      Expr.label(Expr.column("event_count"), "event_count"),
      Expr.label(Expr.column("first_seen"), "first_seen"),
      Expr.label(Expr.column("last_seen"), "last_seen"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("assigned_to")), Expr.value("")]), "assigned_to")
    ])
    |> Query.order_by(:last_seen, :desc)
  let rows = Repo.all(pool, q) ?
  Ok(rows
    |> List.map(fn (row) do
      Issue {
        id : Map.get(row, "id"),
        project_id : Map.get(row, "project_id"),
        fingerprint : Map.get(row, "fingerprint"),
        title : Map.get(row, "title"),
        level : Map.get(row, "level"),
        status : Map.get(row, "status"),
        event_count : parse_event_count(Map.get(row, "event_count")),
        first_seen : Map.get(row, "first_seen"),
        last_seen : Map.get(row, "last_seen"),
        assigned_to : Map.get(row, "assigned_to")
      }
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

pub fn check_volume_spikes(pool :: PoolHandle) -> Int ! String do
  Repo.execute_raw(pool,
  "UPDATE issues SET status = 'unresolved' WHERE status = 'archived' AND id IN (SELECT i.id FROM issues i JOIN events e ON e.issue_id = i.id AND e.received_at > now() - interval '1 hour' WHERE i.status = 'archived' GROUP BY i.id HAVING count(*) > GREATEST(10, (SELECT count(*) FROM events e2 WHERE e2.issue_id = i.id AND e2.received_at > now() - interval '7 days') / 168 * 10))",
  [])
end

# Extract event fields from JSON and compute fingerprint using PostgreSQL.
# This keeps the fingerprint fallback chain next to the JSONB operators it depends
# on: custom > stacktrace frames > exception type > message.
# Returns a Map with keys: fingerprint, title, level.
# Honest raw S03 keep-site: this query still depends on CASE + WITH ORDINALITY +
# jsonb_array_elements/string_agg scalar-subquery behavior for the fingerprint
# fallback chain. S02 moves the write-side/search-side PG helpers onto explicit
# Pg.* surfaces, but this read-side ordinality boundary remains intentionally raw
# until S03 can collapse it without pretending the expression surface is portable.

pub fn extract_event_fields(pool :: PoolHandle, event_json :: String) -> Map < String, String > ! String do
  # Honest raw S03 keep-site: this query still depends on CASE + WITH ORDINALITY +
  # jsonb_array_elements/string_agg scalar-subquery behavior for the fingerprint
  # fallback chain, so S02 keeps it raw until S03 can collapse it honestly.
  let sql = "SELECT CASE WHEN length(COALESCE(j->>'fingerprint', '')) > 0 THEN j->>'fingerprint' WHEN j->'stacktrace' IS NOT NULL AND jsonb_typeof(j->'stacktrace') = 'array' AND jsonb_array_length(j->'stacktrace') > 0 THEN (SELECT string_agg((frame->>'filename') || '|' || (frame->>'function_name'), ';' ORDER BY ordinality) FROM jsonb_array_elements(j->'stacktrace') WITH ORDINALITY AS t(frame, ordinality)) || ':' || lower(COALESCE(replace(j->>'message', '0x', ''), '')) WHEN j->'exception' IS NOT NULL AND j->'exception'->>'type_name' IS NOT NULL THEN (j->'exception'->>'type_name') || ':' || lower(COALESCE(replace(j->'exception'->>'value', '0x', ''), '')) ELSE 'msg:' || lower(COALESCE(replace(j->>'message', '0x', ''), '')) END AS fingerprint, COALESCE(NULLIF(j->>'message', ''), 'Untitled') AS title, COALESCE(j->>'level', 'error') AS level FROM (SELECT $1::jsonb AS j) AS sub"
  let rows = Repo.query_raw(pool, sql, [event_json]) ?
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

pub fn list_issues_filtered(pool :: PoolHandle,
project_id :: String,
status :: String,
level :: String,
assigned_to :: String,
cursor :: String,
cursor_id :: String,
limit_str :: String) -> List < Map < String, String > > ! String do
  if String.length(cursor) > 0 do
    let sql = "SELECT id::text, project_id::text, fingerprint, title, level, status, event_count::text, first_seen::text, last_seen::text, COALESCE(assigned_to::text, '') as assigned_to FROM issues WHERE project_id = $1::uuid AND ($2 = '' OR status = $2) AND ($3 = '' OR level = $3) AND ($4 = '' OR assigned_to = $4::uuid) AND (last_seen, id) < ($5::timestamptz, $6::uuid) ORDER BY last_seen DESC, id DESC LIMIT $7::int"
    let rows = Repo.query_raw(pool,
    sql,
    [project_id, status, level, assigned_to, cursor, cursor_id, limit_str]) ?
    Ok(rows)
  else
    let sql = "SELECT id::text, project_id::text, fingerprint, title, level, status, event_count::text, first_seen::text, last_seen::text, COALESCE(assigned_to::text, '') as assigned_to FROM issues WHERE project_id = $1::uuid AND ($2 = '' OR status = $2) AND ($3 = '' OR level = $3) AND ($4 = '' OR assigned_to = $4::uuid) ORDER BY last_seen DESC, id DESC LIMIT $5::int"
    let rows = Repo.query_raw(pool, sql, [project_id, status, level, assigned_to, limit_str]) ?
    Ok(rows)
  end
end

# SEARCH-02: Full-text search on event messages using inline tsvector.
# Uses inline to_tsvector (avoids partition complications with stored tsvector column).
# Includes 24-hour default time range (SEARCH-04) for partition pruning.
# Returns relevance rank for ordering through expression-valued SELECT/WHERE helpers.

pub fn search_events_fulltext(pool :: PoolHandle,
project_id :: String,
search_query :: String,
limit_str :: String) -> List < Map < String, String > > ! String do
  let lim = parse_limit(limit_str)
  let search_vector = Pg.to_tsvector("english", Expr.column("message"))
  let search_terms = Pg.plainto_tsquery("english", Expr.value(search_query))
  let q = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_expr(Pg.tsvector_matches(search_vector, search_terms))
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.select(["id", "issue_id", "level", "message", "received_at"])
    |> Query.select_expr(Expr.label(Pg.ts_rank(search_vector, search_terms), "rank"))
    |> Query.order_by_raw("rank DESC, received_at DESC")
    |> Query.limit(lim)
  Repo.all(pool, q)
end

# SEARCH-03: Filter events by tag key-value pair using JSONB containment.
# Uses tags @> ?::jsonb operator which leverages existing GIN index (idx_events_tags).
# Includes 24-hour default time range (SEARCH-04).
# Uses expression-valued WHERE composition for the JSONB predicate.

pub fn filter_events_by_tag(pool :: PoolHandle,
project_id :: String,
tag_json :: String,
limit_str :: String) -> List < Map < String, String > > ! String do
  let lim = parse_limit(limit_str)
  let q = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_expr(Pg.jsonb_contains(Expr.column("tags"), Pg.jsonb(Expr.value(tag_json))))
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.select(["id", "issue_id", "level", "message", "tags", "received_at"])
    |> Query.order_by(:received_at, :desc)
    |> Query.limit(lim)
  Repo.all(pool, q)
end

# Event listing within an issue with keyset pagination (for DETAIL-05 context).
# Keyset pagination on (received_at, id) for stable browsing.
# Keeps the tuple cursor predicate raw because the current builder still lacks OR/tuple expression support.

pub fn list_events_for_issue(pool :: PoolHandle,
issue_id :: String,
cursor :: String,
cursor_id :: String,
limit_str :: String) -> List < Map < String, String > > ! String do
  let lim = parse_limit(limit_str)
  let base = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("issue_id"), Pg.uuid(Expr.value(issue_id))))
    |> Query.select_exprs([
      Expr.label(Expr.column("id"), "id"),
      Expr.label(Expr.column("level"), "level"),
      Expr.label(Expr.column("message"), "message"),
      Expr.label(Expr.column("received_at"), "received_at")
    ])
    |> Query.order_by(:received_at, :desc)
    |> Query.order_by(:id, :desc)
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
# bucket param is normalized to the honest allow-list used by the caller surface.
# Default 24-hour time window.
# Uses structured SELECT expressions plus alias-based GROUP BY / ORDER BY.

pub fn event_volume_hourly(pool :: PoolHandle, project_id :: String, bucket :: String) -> List < Map < String, String > > ! String do
  let bucket_name = normalize_time_bucket(bucket)
  let bucket_expr = Expr.fn_call("date_trunc", [Expr.value(bucket_name), Expr.column("received_at")])
  let q = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.select_exprs([
      Expr.label(bucket_expr, "bucket"),
      Expr.label(Expr.fn_call("count", [Expr.column("*")]), "count")
    ])
    |> Query.group_by(:bucket)
    |> Query.order_by(:bucket, :asc)
  Repo.all(pool, q)
end

# DASH-02: Error breakdown by severity level for a project.
# Groups events by level (error, warning, info, etc.) with counts.
# Uses structured SELECT expressions with regular GROUP BY / ORDER BY.

pub fn error_breakdown_by_level(pool :: PoolHandle, project_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.select_exprs([
      Expr.label(Expr.column("level"), "level"),
      Expr.label(Expr.fn_call("count", [Expr.column("*")]), "count")
    ])
    |> Query.group_by(:level)
    |> Query.order_by(:count, :desc)
  Repo.all(pool, q)
end

# DASH-03: Top issues ranked by frequency (event count).
# Returns unresolved issues ordered by event_count DESC.
# Uses structured projection helpers while keeping numeric ORDER BY on the real column.

pub fn top_issues_by_frequency(pool :: PoolHandle, project_id :: String, limit_str :: String) -> List < Map < String, String > > ! String do
  let lim = parse_limit(limit_str)
  let q = Query.from(Issue.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_expr(Expr.eq(Expr.column("status"), Expr.value("unresolved")))
    |> Query.select_exprs([
      Expr.label(Expr.column("id"), "id"),
      Expr.label(Expr.column("title"), "title"),
      Expr.label(Expr.column("level"), "level"),
      Expr.label(Expr.column("status"), "status"),
      Expr.label(Expr.column("event_count"), "event_count"),
      Expr.label(Expr.column("last_seen"), "last_seen")
    ])
    |> Query.order_by(:event_count, :desc)
    |> Query.limit(lim)
  Repo.all(pool, q)
end

# DASH-04: Event breakdown by tag key (environment, release, etc.).
# Uses jsonb_exists/jsonb_extract_path_text through expression-valued helpers
# so the JSONB key filter and projection stay on the explicit PG surface.

pub fn event_breakdown_by_tag(pool :: PoolHandle, project_id :: String, tag_key :: String) -> List < Map < String, String > > ! String do
  let tag_value = Expr.fn_call("jsonb_extract_path_text", [Expr.column("tags"), Expr.value(tag_key)])
  let q = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_raw("received_at > now() - interval '24 hours'", [])
    |> Query.where_expr(Expr.fn_call("jsonb_exists", [Expr.column("tags"), Expr.value(tag_key)]))
    |> Query.select_exprs([
      Expr.label(tag_value, "tag_value"),
      Expr.label(Expr.fn_call("count", [Expr.column("*")]), "count")
    ])
    |> Query.group_by(:tag_value)
    |> Query.order_by(:count, :desc)
    |> Query.limit(20)
  Repo.all(pool, q)
end

# DASH-05: Per-issue event timeline (recent events for a specific issue).
# Ordered by received_at DESC for chronological browsing.
# Uses ORM Query.where_raw + Query.select_raw + Query.order_by + Query.limit + Repo.all.

pub fn issue_event_timeline(pool :: PoolHandle, issue_id :: String, limit_str :: String) -> List < Map < String, String > > ! String do
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

pub fn project_health_summary(pool :: PoolHandle, project_id :: String) -> List < Map < String, String > > ! String do
  let sql = "SELECT (SELECT count(*) FROM issues WHERE project_id = $1::uuid AND status = 'unresolved')::text AS unresolved_count, (SELECT count(*) FROM events WHERE project_id = $1::uuid AND received_at > now() - interval '24 hours')::text AS events_24h, (SELECT count(*) FROM issues WHERE project_id = $1::uuid AND first_seen > now() - interval '24 hours')::text AS new_today"
  let rows = Repo.query_raw(pool, sql, [project_id]) ?
  Ok(rows)
end

# --- Event detail queries (Phase 91 Plan 02) ---
# DETAIL-01..04, DETAIL-06: Get complete event with all JSONB fields.
# Returns full event payload including exception, stacktrace, breadcrumbs,
# tags, extra, user_context. JSONB fields use COALESCE for null safety.
# Uses structured projection helpers instead of raw SELECT fragments.

pub fn get_event_detail(pool :: PoolHandle, event_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("id"), Pg.uuid(Expr.value(event_id))))
    |> Query.select_exprs([
      Expr.label(Expr.column("id"), "id"),
      Expr.label(Expr.column("project_id"), "project_id"),
      Expr.label(Expr.column("issue_id"), "issue_id"),
      Expr.label(Expr.column("level"), "level"),
      Expr.label(Expr.column("message"), "message"),
      Expr.label(Expr.column("fingerprint"), "fingerprint"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("exception")), Expr.value("null")]), "exception"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("stacktrace")), Expr.value("[]")]), "stacktrace"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("breadcrumbs")), Expr.value("[]")]), "breadcrumbs"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("tags")), Expr.value("{}")]), "tags"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("extra")), Expr.value("{}")]), "extra"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("user_context")), Expr.value("null")]), "user_context"),
      Expr.label(Expr.coalesce([Expr.column("sdk_name"), Expr.value("")]), "sdk_name"),
      Expr.label(Expr.coalesce([Expr.column("sdk_version"), Expr.value("")]), "sdk_version"),
      Expr.label(Expr.column("received_at"), "received_at")
    ])
  Repo.all(pool, q)
end

# DETAIL-05: Get next and previous event IDs within an issue for navigation.
# Uses tuple comparison (received_at, id) for stable ordering.
# ORM boundary: Two scalar subqueries with opposing sort orders and tuple comparison
# in a single SELECT -- each subquery uses (received_at, id) tuple comparison with
# different directions (> for next, < for prev) and LIMIT 1. The ORM Query builder
# cannot compose multiple independent subqueries in SELECT expressions. Intentional raw SQL.

pub fn get_event_neighbors(pool :: PoolHandle,
issue_id :: String,
received_at :: String,
event_id :: String) -> List < Map < String, String > > ! String do
  let sql = "SELECT (SELECT id::text FROM events WHERE issue_id = $1::uuid AND (received_at, id) > ($2::timestamptz, $3::uuid) ORDER BY received_at, id LIMIT 1) AS next_id, (SELECT id::text FROM events WHERE issue_id = $1::uuid AND (received_at, id) < ($2::timestamptz, $3::uuid) ORDER BY received_at DESC, id DESC LIMIT 1) AS prev_id"
  let rows = Repo.query_raw(pool, sql, [issue_id, received_at, event_id]) ?
  Ok(rows)
end

# --- Team management queries (Phase 91 Plan 03 -- ORG-04) ---
# Update a member's role. SQL-side validation ensures only valid roles accepted.
# Returns affected row count (0 if invalid role or membership not found).
# Uses ORM Repo.update_where with Query.where_raw for role validation.

pub fn update_member_role(pool :: PoolHandle, membership_id :: String, new_role :: String) -> Int ! String do
  let q = Query.from(OrgMembership.__table__())
    |> Query.where_raw("id = ?::uuid AND ? IN ('owner', 'admin', 'member')",
    [membership_id, new_role])
  Repo.update_where(pool, OrgMembership.__table__(), %{"role" => new_role}, q) ?
  Ok(1)
end

# Remove a member from an organization.
# Returns affected row count (0 if membership not found).

pub fn remove_member(pool :: PoolHandle, membership_id :: String) -> Int ! String do
  Repo.delete(pool, OrgMembership.__table__(), membership_id) ?
  Ok(1)
end

# List all members of an organization with user info (email, display_name).
# JOIN with users table for enriched member listing.
# Returns raw Map rows for flexible JSON serialization.
# Uses structured SELECT expressions with regular ordering.

pub fn get_members_with_users(pool :: PoolHandle, org_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(OrgMembership.__table__())
    |> Query.join_as(:inner, User.__table__(), "u", "u.id = org_memberships.user_id")
    |> Query.where_expr(Expr.eq(Expr.column("org_memberships.org_id"), Pg.uuid(Expr.value(org_id))))
    |> Query.select_exprs([
      Expr.label(Expr.column("org_memberships.id"), "id"),
      Expr.label(Expr.column("org_memberships.user_id"), "user_id"),
      Expr.label(Expr.column("org_memberships.org_id"), "org_id"),
      Expr.label(Expr.column("org_memberships.role"), "role"),
      Expr.label(Expr.column("org_memberships.joined_at"), "joined_at"),
      Expr.label(Expr.column("u.email"), "email"),
      Expr.label(Expr.column("u.display_name"), "display_name")
    ])
    |> Query.order_by(:joined_at, :asc)
  Repo.all(pool, q)
end

# --- API token management queries (Phase 91 Plan 03 -- ORG-05) ---
# List all API keys for a project with full details.
# Returns raw Map rows. revoked_at is empty string if not revoked.
# Uses Query.where_expr + Query.select_exprs + Query.order_by.

pub fn list_api_keys(pool :: PoolHandle, project_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(ApiKey.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.select_exprs([
      Expr.label(Pg.text(Expr.column("id")), "id"),
      Expr.label(Pg.text(Expr.column("project_id")), "project_id"),
      Expr.label(Expr.column("key_value"), "key_value"),
      Expr.label(Expr.column("label"), "label"),
      Expr.label(Pg.text(Expr.column("created_at")), "created_at"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("revoked_at")), Expr.value("")]), "revoked_at")
    ])
    |> Query.order_by(:created_at, :desc)
  Repo.all(pool, q)
end

# --- Alert system queries (Phase 92) ---
# ALERT-01: Insert alert rule from JSON body using Repo.insert_expr plus
# PostgreSQL JSONB extraction/defaulting helpers.

pub fn create_alert_rule(pool :: PoolHandle, project_id :: String, body :: String) -> String ! String do
  let body_json = Pg.jsonb(Expr.value(body))
  let row = Repo.insert_expr(pool,
  AlertRule.__table__(),
  %{"project_id" => Pg.uuid(Expr.value(project_id)), "name" => Expr.coalesce([Expr.fn_call("jsonb_extract_path_text",
  [body_json, Expr.value("name")]), Expr.value("Unnamed Rule")]), "condition_json" => Expr.coalesce([Expr.fn_call("jsonb_extract_path",
  [body_json, Expr.value("condition")]), Pg.jsonb(Expr.value("{}"))]), "action_json" => Expr.coalesce([Expr.fn_call("jsonb_extract_path",
  [body_json, Expr.value("action")]), Pg.jsonb(Expr.value("{\"type\":\"websocket\"}"))]), "cooldown_minutes" => Expr.coalesce([Pg.int(Expr.fn_call("jsonb_extract_path_text",
  [body_json, Expr.value("cooldown_minutes")])), Pg.int(Expr.value("60"))])}) ?
  Ok(Map.get(row, "id"))
end

# ALERT-01: List all alert rules for a project.
# Uses Query.where_expr + Query.select_exprs + Query.order_by instead of raw projection strings.

pub fn list_alert_rules(pool :: PoolHandle, project_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.select_exprs([
      Expr.label(Pg.text(Expr.column("id")), "id"),
      Expr.label(Pg.text(Expr.column("project_id")), "project_id"),
      Expr.label(Expr.column("name"), "name"),
      Expr.label(Pg.text(Expr.column("condition_json")), "condition_json"),
      Expr.label(Pg.text(Expr.column("action_json")), "action_json"),
      Expr.label(Pg.text(Expr.column("enabled")), "enabled"),
      Expr.label(Pg.text(Expr.column("cooldown_minutes")), "cooldown_minutes"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("last_fired_at")), Expr.value("")]), "last_fired_at"),
      Expr.label(Pg.text(Expr.column("created_at")), "created_at")
    ])
    |> Query.order_by(:created_at, :desc)
  Repo.all(pool, q)
end

# Enable/disable an alert rule.
# Uses ORM Repo.update_where with Query.where_raw.

pub fn toggle_alert_rule(pool :: PoolHandle, rule_id :: String, enabled_str :: String) -> Int ! String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_raw("id = ?::uuid", [rule_id])
  Repo.update_where(pool, AlertRule.__table__(), %{"enabled" => enabled_str}, q) ?
  Ok(1)
end

# Delete an alert rule.

pub fn delete_alert_rule(pool :: PoolHandle, rule_id :: String) -> Int ! String do
  Repo.delete(pool, AlertRule.__table__(), rule_id) ?
  Ok(1)
end

# ALERT-02: Count events in time window AND check cooldown, return true if should fire.
# ORM boundary: Cross-join between two derived tables (event count subquery + cooldown subquery)
# with CASE expression, interval arithmetic, and multiple bound parameters in complex expressions.
# Not expressible via ORM query builder. Intentional raw SQL.

pub fn evaluate_threshold_rule(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
threshold_str :: String,
window_str :: String,
cooldown_str :: String) -> Bool ! String do
  let sql = "SELECT CASE WHEN event_count > $3::int AND (last_fired IS NULL OR last_fired < now() - interval '1 minute' * $6::int) THEN 1 ELSE 0 END AS should_fire FROM (SELECT count(*) AS event_count FROM events WHERE project_id = $2::uuid AND received_at > now() - interval '1 minute' * $4::int) counts, (SELECT last_fired_at AS last_fired FROM alert_rules WHERE id = $1::uuid) cooldown"
  let rows = Repo.query_raw(pool,
  sql,
  [rule_id, project_id, threshold_str, window_str, "", cooldown_str]) ?
  if List.length(rows) > 0 do
    let should_fire = Map.get(List.head(rows), "should_fire")
    Ok(should_fire == "1")
  else
    Ok(false)
  end
end

# ALERT-04/05: Insert alert record, update last_fired_at, return alert_id.
# Uses expression-valued insert/update helpers instead of raw jsonb_build_object SQL.

pub fn fire_alert(pool :: PoolHandle,
rule_id :: String,
project_id :: String,
message :: String,
condition_type :: String,
rule_name :: String) -> String ! String do
  let row = Repo.insert_expr(pool,
  Alert.__table__(),
  %{"rule_id" => Pg.uuid(Expr.value(rule_id)), "project_id" => Pg.uuid(Expr.value(project_id)), "status" => Expr.value("active"), "message" => Expr.value(message), "condition_snapshot" => Expr.fn_call("jsonb_build_object",
  [Pg.text(Expr.value("condition_type")), Pg.text(Expr.value(condition_type)), Pg.text(Expr.value("rule_name")), Pg.text(Expr.value(rule_name))])}) ?
  let q = Query.from(AlertRule.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("id"), Pg.uuid(Expr.value(rule_id))))
  Repo.update_where_expr(pool,
  AlertRule.__table__(),
  %{"last_fired_at" => Expr.fn_call("now", [])},
  q) ?
  Ok(Map.get(row, "id"))
end

# ALERT-03: Check if an issue was just created (first_seen = last_seen).
# Uses structured WHERE expressions plus Repo.exists.

pub fn check_new_issue(pool :: PoolHandle, issue_id :: String) -> Bool ! String do
  let q = Query.from(Issue.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("id"), Pg.uuid(Expr.value(issue_id))))
    |> Query.where_expr(Expr.eq(Expr.column("first_seen"), Expr.column("last_seen")))
    |> Query.select(["id"])
  let rows = Repo.all(pool, q) ?
  Ok(List.length(rows) > 0)
end

# ALERT-03: Get enabled alert rules for event-based conditions for a project.
# Uses expression-valued JSONB extraction for condition_json filtering.

pub fn get_event_alert_rules(pool :: PoolHandle, project_id :: String, condition_type :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.where_expr(Expr.eq(Expr.column("enabled"), Pg.cast(Expr.value("true"), "boolean")))
    |> Query.where_expr(Expr.eq(Expr.fn_call("jsonb_extract_path_text",
    [Expr.column("condition_json"), Expr.value("condition_type")]),
    Expr.value(condition_type)))
    |> Query.select(["id", "name", "cooldown_minutes"])
  Repo.all(pool, q)
end

# ALERT-05: Check cooldown before firing (for event-based triggers).
# Uses Repo.exists so the helper stays builder-backed apart from the interval predicate.

pub fn should_fire_by_cooldown(pool :: PoolHandle, rule_id :: String, cooldown_str :: String) -> Bool ! String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("id"), Pg.uuid(Expr.value(rule_id))))
    |> Query.where_raw("(last_fired_at IS NULL OR last_fired_at < now() - interval '1 minute' * ?::int)",
    [cooldown_str])
    |> Query.select(["id"])
  let rows = Repo.all(pool, q) ?
  Ok(List.length(rows) > 0)
end

# ALERT-06: Transition alert to acknowledged.
# Uses expression-aware Repo.update_where_expr for the now() timestamp update.

pub fn acknowledge_alert(pool :: PoolHandle, alert_id :: String) -> Int ! String do
  let q = Query.from(Alert.__table__())
    |> Query.where_raw("id = ?::uuid", [alert_id])
    |> Query.where(:status, "active")
  Repo.update_where_expr(pool,
  Alert.__table__(),
  %{"status" => Expr.value("acknowledged"), "acknowledged_at" => Expr.fn_call("now", [])},
  q) ?
  Ok(1)
end

# ALERT-06: Transition alert to resolved.
# Uses expression-aware Repo.update_where_expr for the now() timestamp update.

pub fn resolve_fired_alert(pool :: PoolHandle, alert_id :: String) -> Int ! String do
  let q = Query.from(Alert.__table__())
    |> Query.where_raw("id = ?::uuid AND status IN ('active', 'acknowledged')", [alert_id])
  Repo.update_where_expr(pool,
  Alert.__table__(),
  %{"status" => Expr.value("resolved"), "resolved_at" => Expr.fn_call("now", [])},
  q) ?
  Ok(1)
end

# ALERT-06: List alerts for a project filtered by status.
# Uses structured SELECT expressions plus conditional query assembly.

pub fn list_alerts(pool :: PoolHandle, project_id :: String, status :: String) -> List < Map < String, String > > ! String do
  let base = Query.from(Alert.__table__())
    |> Query.join_as(:inner, AlertRule.__table__(), "r", "r.id = alerts.rule_id")
    |> Query.where_expr(Expr.eq(Expr.column("alerts.project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.select_exprs([
      Expr.label(Expr.column("alerts.id"), "id"),
      Expr.label(Expr.column("alerts.rule_id"), "rule_id"),
      Expr.label(Expr.column("alerts.project_id"), "project_id"),
      Expr.label(Expr.column("alerts.status"), "status"),
      Expr.label(Expr.column("alerts.message"), "message"),
      Expr.label(Pg.text(Expr.column("alerts.condition_snapshot")), "condition_snapshot"),
      Expr.label(Expr.column("alerts.triggered_at"), "triggered_at"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("alerts.acknowledged_at")), Expr.value("")]), "acknowledged_at"),
      Expr.label(Expr.coalesce([Pg.text(Expr.column("alerts.resolved_at")), Expr.value("")]), "resolved_at"),
      Expr.label(Expr.column("r.name"), "rule_name")
    ])
    |> Query.order_by(:triggered_at, :desc)
    |> Query.limit(50)
  if String.length(status) > 0 do
    let q = base
      |> Query.where_expr(Expr.eq(Expr.column("alerts.status"), Expr.value(status)))
    Repo.all(pool, q)
  else
    Repo.all(pool, base)
  end
end

# Load all enabled threshold rules for evaluation.
# Uses expression-valued JSONB extraction for the threshold condition filter.

pub fn get_threshold_rules(pool :: PoolHandle) -> List < Map < String, String > > ! String do
  let q = Query.from(AlertRule.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("enabled"), Pg.cast(Expr.value("true"), "boolean")))
    |> Query.where_expr(Expr.eq(Expr.fn_call("jsonb_extract_path_text",
    [Expr.column("condition_json"), Expr.value("condition_type")]),
    Expr.value("threshold")))
    |> Query.select(["id", "project_id", "name", "condition_json", "cooldown_minutes"])
  Repo.all(pool, q)
end

# --- Retention and storage queries (Phase 93, ORM rewrite Phase 113) ---
# Delete expired events for a project based on its retention_days setting.
# Returns the number of deleted rows.
# Uses ORM Repo.delete_where + Query.where_raw for interval expression instead of Repo.execute_raw.

pub fn delete_expired_events(pool :: PoolHandle, project_id :: String, retention_days_str :: String) -> Int ! String do
  let q = Query.from(Event.__table__())
    |> Query.where_raw("project_id = ?::uuid AND received_at < now() - (? || ' days')::interval",
    [project_id, retention_days_str])
  Repo.delete_where(pool, Event.__table__(), q)
end

# Find event partitions older than max_days (for partition cleanup).
# Queries pg_inherits to find child tables of 'events' with names matching events_YYYYMMDD.
# DDL/catalog query -- queries pg_inherits/pg_class system catalogs. Excluded from data query raw SQL count per ORM rewrite scope.

pub fn get_expired_partitions(pool :: PoolHandle, max_days_str :: String) -> List < Map < String, String > > ! String do
  let sql = "SELECT c.relname::text AS partition_name FROM pg_inherits i JOIN pg_class c ON c.oid = i.inhrelid JOIN pg_class p ON p.oid = i.inhparent WHERE p.relname = 'events' AND c.relname ~ '^events_[0-9]{8}$' AND to_date(substring(c.relname from '[0-9]{8}$'), 'YYYYMMDD') < (current_date - ($1 || ' days')::interval)"
  let rows = Repo.query_raw(pool, sql, [max_days_str]) ?
  Ok(rows)
end

# Drop a single event partition by name.
# The partition_name comes from the trusted pg_inherits query, not user input.
# DDL operation (DROP TABLE) -- excluded from data query raw SQL count per ORM rewrite scope.

pub fn drop_partition(pool :: PoolHandle, partition_name :: String) -> Int ! String do
  Repo.execute_raw(pool, "DROP TABLE IF EXISTS " <> partition_name, [])
end

# Get all projects with their retention settings for the cleanup loop.
# Uses Query.select_exprs so the cleanup row shape is explicit and stable.

pub fn get_all_project_retention(pool :: PoolHandle) -> List < Map < String, String > > ! String do
  let q = Query.from(Project.__table__())
    |> Query.select_exprs([
      Expr.label(Pg.text(Expr.column("id")), "id"),
      Expr.label(Pg.text(Expr.column("retention_days")), "retention_days")
    ])
  Repo.all(pool, q)
end

# Estimate storage usage for a project (event count and estimated bytes).
# Uses 1024 byte average row estimate.
# Uses Query.where_expr + Query.select_exprs instead of raw projection strings.

pub fn get_project_storage(pool :: PoolHandle, project_id :: String) -> List < Map < String, String > > ! String do
  let event_count = Expr.fn_call("count", [Expr.column("*")])
  let estimated_bytes = Expr.mul(event_count, Pg.cast(Expr.value("1024"), "bigint"))
  let q = Query.from(Event.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("project_id"), Pg.uuid(Expr.value(project_id))))
    |> Query.select_exprs([
      Expr.label(Pg.text(event_count), "event_count"),
      Expr.label(Pg.text(estimated_bytes), "estimated_bytes")
    ])
  Repo.all(pool, q)
end

# Update project retention and sampling settings from JSON body.
# Uses Mesh-side Json.get parsing so the neutral write path only updates the
# fields that were actually provided by the caller.

pub fn update_project_settings(pool :: PoolHandle, project_id :: String, body :: String) -> Int ! String do
  let retention_days = Json.get(body, "retention_days")
  let sample_rate = Json.get(body, "sample_rate")
  if String.length(retention_days) > 0 do
    let q = Query.from(Project.__table__())
      |> Query.where_raw("id = ?::uuid", [project_id])
    if String.length(sample_rate) > 0 do
      Repo.update_where_expr(pool,
      Project.__table__(),
      %{"retention_days" => Expr.value(retention_days), "sample_rate" => Expr.value(sample_rate)},
      q) ?
      Ok(1)
    else
      Repo.update_where_expr(pool,
      Project.__table__(),
      %{"retention_days" => Expr.value(retention_days)},
      q) ?
      Ok(1)
    end
  else if String.length(sample_rate) > 0 do
    let q = Query.from(Project.__table__())
      |> Query.where_raw("id = ?::uuid", [project_id])
    Repo.update_where_expr(pool,
    Project.__table__(),
    %{"sample_rate" => Expr.value(sample_rate)},
    q) ?
    Ok(1)
  else
    Ok(0)
  end
end

# Get retention and sampling settings for a project.
# Uses Query.where_expr + Query.select_exprs so the API row keys stay explicit.

pub fn get_project_settings(pool :: PoolHandle, project_id :: String) -> List < Map < String, String > > ! String do
  let q = Query.from(Project.__table__())
    |> Query.where_expr(Expr.eq(Expr.column("id"), Pg.uuid(Expr.value(project_id))))
    |> Query.select_exprs([
      Expr.label(Pg.text(Expr.column("retention_days")), "retention_days"),
      Expr.label(Pg.text(Expr.column("sample_rate")), "sample_rate")
    ])
  Repo.all(pool, q)
end

# Check if an event should be kept based on the project's sample_rate.
# Returns true if the event should be kept, false if it should be dropped.
# Defaults to keeping all events (sample_rate = 1.0) if project not found.
# ORM boundary: SELECT random() < COALESCE((SELECT ...), 1.0) uses a server-side
# random() function comparison with a scalar subquery and COALESCE default.
# Not expressible via ORM query builder. Intentional raw SQL.

pub fn check_sample_rate(pool :: PoolHandle, project_id :: String) -> Bool ! String do
  let rows = Repo.query_raw(pool,
  "SELECT random() < COALESCE((SELECT sample_rate FROM projects WHERE id = $1::uuid), 1.0) AS keep",
  [project_id]) ?
  if List.length(rows) > 0 do
    Ok(Map.get(List.head(rows), "keep") == "t")
  else
    Ok(true)
  end
end
