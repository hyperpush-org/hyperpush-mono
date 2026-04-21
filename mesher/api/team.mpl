# Team membership management and API token lifecycle HTTP handlers.
# Provides ORG-04 (list/add/update-role/remove members) and ORG-05
# (list/create/revoke API keys) endpoints.
# All handlers follow the PipelineRegistry pattern for pool lookup.
# POST used for all mutation operations per decision [89-02].

from Ingestion.Pipeline import PipelineRegistry
from Storage.Queries import (
  get_members_with_users,
  add_member,
  update_member_role,
  remove_member,
  list_api_keys,
  create_api_key,
  revoke_api_key
)
from Types.User import User, OrgMembership
from Types.Project import ApiKey
from Api.Helpers import query_or_default, to_json_array, require_param, get_registry, resolve_org_id, resolve_project_id

# --- Shared helpers (leaf functions first, per define-before-use requirement) ---
# Serialize a member row (with user info) to JSON.
# Fields: id, user_id, email, display_name, role, joined_at

fn member_row_to_membership(row) -> OrgMembership do
  OrgMembership {
    id : Map.get(row, "id"),
    user_id : Map.get(row, "user_id"),
    org_id : "",
    role : Map.get(row, "role"),
    joined_at : Map.get(row, "joined_at")
  }
end

fn member_row_to_user(row) -> User do
  User {
    id : Map.get(row, "user_id"),
    email : Map.get(row, "email"),
    display_name : Map.get(row, "display_name"),
    created_at : ""
  }
end

fn member_to_json(row) -> String do
  let membership_json = Json.encode(member_row_to_membership(row))
  let user_json = Json.encode(member_row_to_user(row))
  let id = Json.get(membership_json, "id")
  let user_id = Json.get(membership_json, "user_id")
  let email = Json.get(user_json, "email")
  let display_name = Json.get(user_json, "display_name")
  let role = Json.get(membership_json, "role")
  let joined_at = Json.get(membership_json, "joined_at")
  """{"id":"#{id}","user_id":"#{user_id}","email":"#{email}","display_name":"#{display_name}","role":"#{role}","joined_at":"#{joined_at}"}"""
end

# Serialize an API key row to JSON.
# Fields: id, project_id, key_value, label, created_at, revoked_at
# Uses Json.encode + Json.get so string fields remain truthful values.

fn api_key_row_to_api_key(row) -> ApiKey do
  ApiKey {
    id : Map.get(row, "id"),
    project_id : Map.get(row, "project_id"),
    key_value : Map.get(row, "key_value"),
    label : Map.get(row, "label"),
    created_at : Map.get(row, "created_at"),
    revoked_at : if String.length(Map.get(row, "revoked_at")) == 0 do
      None
    else
      Some(Map.get(row, "revoked_at"))
    end
  }
end

fn api_key_to_json(row) -> String do
  let api_key_json = Json.encode(api_key_row_to_api_key(row))
  let id = Json.get(api_key_json, "id")
  let project_id = Json.get(api_key_json, "project_id")
  let key_value = Json.get(api_key_json, "key_value")
  let label = Json.get(api_key_json, "label")
  let created_at = Json.get(api_key_json, "created_at")
  let revoked_at_raw = Json.get(api_key_json, "revoked_at")
  let revoked_at = if revoked_at_raw == "" or revoked_at_raw == "null" do
    "null"
  else
    "\"#{revoked_at_raw}\""
  end
  """{"id":"#{id}","project_id":"#{project_id}","key_value":"#{key_value}","label":"#{label}","created_at":"#{created_at}","revoked_at":#{revoked_at}}"""
end

# Extract a field from a JSON body using Mesh-native Json.get (no DB roundtrip).
# Returns the value or empty string if field is missing/null.

fn extract_json_field(pool :: PoolHandle, body :: String, field :: String) -> String ! String do
  Ok(Json.get(body, field))
end

# Helper: resolve org path identifiers (UUID or slug) and fail explicitly when unknown.

fn resolve_team_org_id(pool :: PoolHandle, raw_id :: String) -> String do
  resolve_org_id(pool, raw_id)
end

# --- Team membership helper functions for case arm extraction (ORG-04) ---
# Helper: handle successful add_member result.

fn add_member_success(membership_id :: String) do
  HTTP.response(201, json { id : membership_id })
end

# Helper: handle successful update_member_role result.

fn update_role_success() do
  HTTP.response(200, json { status : "ok" })
end

# Helper: handle successful remove_member result.

fn remove_success() do
  HTTP.response(200, json { status : "ok" })
end

# --- API token helper functions for case arm extraction (ORG-05) ---
# Helper: handle successful create_api_key result.

fn create_key_success(key_value :: String) do
  HTTP.response(201, json { key_value : key_value })
end

# Helper: handle successful revoke_api_key result.

fn revoke_key_success() do
  HTTP.response(200, json { status : "ok" })
end

# --- Add member helper chain ---
# Helper: perform add_member after extracting user_id and role from body.

fn do_add_member(pool :: PoolHandle, org_id :: String, user_id :: String, role :: String) do
  let result = add_member(pool, user_id, org_id, role)
  case result do
    Ok( id) -> add_member_success(id)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Helper: extract role and dispatch add_member.

fn add_member_with_role(pool :: PoolHandle, org_id :: String, user_id :: String, body :: String) do
  let role_result = extract_json_field(pool, body, "role")
  case role_result do
    Ok( role) -> do_add_member(pool,
    org_id,
    user_id,
    if String.length(role) == 0 do
      "member"
    else
      role
    end)
    Err( e) -> HTTP.response(400, json { error : "invalid json" })
  end
end

# Helper: check user_id is non-empty.

fn check_user_id(pool :: PoolHandle, org_id :: String, user_id :: String, body :: String) do
  if String.length(user_id) == 0 do
    HTTP.response(400, json { error : "user_id is required" })
  else
    add_member_with_role(pool, org_id, user_id, body)
  end
end

# Helper: validate user_id and dispatch add member.

fn validate_add_member(pool :: PoolHandle, org_id :: String, body :: String) do
  let uid_result = extract_json_field(pool, body, "user_id")
  case uid_result do
    Ok( user_id) -> check_user_id(pool, org_id, user_id, body)
    Err( e) -> HTTP.response(400, json { error : "invalid json" })
  end
end

# --- Update member role helper chain ---
# Helper: perform the actual role update.

fn perform_role_update(pool :: PoolHandle, membership_id :: String, role :: String) do
  let result = update_member_role(pool, membership_id, role)
  case result do
    Ok( count) -> if count > 0 do
      update_role_success()
    else
      HTTP.response(400, json { error : "invalid role or membership not found" })
    end
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Helper: extract role and perform update.

fn do_update_role(pool :: PoolHandle, membership_id :: String, body :: String) do
  let role_result = extract_json_field(pool, body, "role")
  case role_result do
    Ok( role) -> perform_role_update(pool, membership_id, role)
    Err( e) -> HTTP.response(400, json { error : "invalid json" })
  end
end

# --- Create API key helper chain ---
# Helper: perform the actual key creation.

fn perform_create_key(pool :: PoolHandle, project_id :: String, label :: String) do
  let result = create_api_key(pool, project_id, label)
  case result do
    Ok( key_value) -> create_key_success(key_value)
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Helper: extract label and perform key creation.

fn do_create_key(pool :: PoolHandle, project_id :: String, body :: String) do
  let label_result = extract_json_field(pool, body, "label")
  case label_result do
    Ok( label) -> perform_create_key(pool,
    project_id,
    if String.length(label) == 0 do
      "default"
    else
      label
    end)
    Err( e) -> HTTP.response(400, json { error : "invalid json" })
  end
end

# --- Handler functions (pub, defined after all helpers) ---
# Handle GET /api/v1/orgs/:org_id/members
# Lists all members of an organization with user info (email, display_name).

pub fn handle_list_members(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "org_id")
  let org_id = resolve_team_org_id(pool, raw_id)
  if String.length(org_id) == 0 do
    HTTP.response(404, json { error : "organization not found" })
  else
    let result = get_members_with_users(pool, org_id)
    case result do
      Ok( rows) -> HTTP.response(200,
      rows
        |> List.map(fn (row) do member_to_json(row) end)
        |> to_json_array())
      Err( e) -> HTTP.response(500, json { error : e })
    end
  end
end

# Handle POST /api/v1/orgs/:org_id/members
# Adds a member to an organization. Body: {"user_id":"...","role":"member"}
# role defaults to "member" if omitted.

pub fn handle_add_member(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "org_id")
  let org_id = resolve_team_org_id(pool, raw_id)
  let body = Request.body(request)
  if String.length(org_id) == 0 do
    HTTP.response(404, json { error : "organization not found" })
  else
    validate_add_member(pool, org_id, body)
  end
end

# Handle POST /api/v1/orgs/:org_id/members/:membership_id/role
# Updates a member's role. Body: {"role":"admin"}

pub fn handle_update_member_role(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "org_id")
  let org_id = resolve_team_org_id(pool, raw_id)
  let membership_id = require_param(request, "membership_id")
  let body = Request.body(request)
  if String.length(org_id) == 0 do
    HTTP.response(404, json { error : "organization not found" })
  else
    do_update_role(pool, membership_id, body)
  end
end

# Handle POST /api/v1/orgs/:org_id/members/:membership_id/remove
# Removes a member from an organization.

pub fn handle_remove_member(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "org_id")
  let org_id = resolve_team_org_id(pool, raw_id)
  let membership_id = require_param(request, "membership_id")
  if String.length(org_id) == 0 do
    HTTP.response(404, json { error : "organization not found" })
  else
    let result = remove_member(pool, membership_id)
    case result do
      Ok( count) -> if count > 0 do
        remove_success()
      else
        HTTP.response(404, json { error : "membership not found" })
      end
      Err( e) -> HTTP.response(500, json { error : e })
    end
  end
end

# Handle GET /api/v1/projects/:project_id/api-keys
# Lists all API keys for a project with revocation status.

pub fn handle_list_api_keys(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let result = list_api_keys(pool, project_id)
  case result do
    Ok( rows) -> HTTP.response(200,
    rows
      |> List.map(fn (row) do api_key_to_json(row) end)
      |> to_json_array())
    Err( e) -> HTTP.response(500, json { error : e })
  end
end

# Handle POST /api/v1/projects/:project_id/api-keys
# Creates a new API key for a project. Body: {"label":"my-key"}
# label defaults to "default" if omitted.

pub fn handle_create_api_key(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let raw_id = require_param(request, "project_id")
  let project_id = resolve_project_id(pool, raw_id)
  let body = Request.body(request)
  do_create_key(pool, project_id, body)
end

# Handle POST /api/v1/api-keys/:key_id/revoke
# Revokes an API key by setting revoked_at to now().

pub fn handle_revoke_api_key(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let key_id = require_param(request, "key_id")
  let result = revoke_api_key(pool, key_id)
  case result do
    Ok -> revoke_key_success()
    Err( e) -> HTTP.response(500, json { error : e })
  end
end
