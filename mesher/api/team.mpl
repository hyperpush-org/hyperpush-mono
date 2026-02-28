# Team membership management and API token lifecycle HTTP handlers.
# Provides ORG-04 (list/add/update-role/remove members) and ORG-05
# (list/create/revoke API keys) endpoints.
# All handlers follow the PipelineRegistry pattern for pool lookup.
# POST used for all mutation operations per decision [89-02].

from Ingestion.Pipeline import PipelineRegistry
from Storage.Queries import get_members_with_users, add_member, update_member_role, remove_member, list_api_keys, create_api_key, revoke_api_key
from Api.Helpers import query_or_default, to_json_array, require_param, get_registry, resolve_project_id

# --- Shared helpers (leaf functions first, per define-before-use requirement) ---

# Serialize a member row (with user info) to JSON.
# Fields: id, user_id, email, display_name, role, joined_at
fn member_to_json(row) -> String do
  let id = Map.get(row, "id")
  let user_id = Map.get(row, "user_id")
  let email = Map.get(row, "email")
  let display_name = Map.get(row, "display_name")
  let role = Map.get(row, "role")
  let joined_at = Map.get(row, "joined_at")
  "{\"id\":\"" <> id <> "\",\"user_id\":\"" <> user_id <> "\",\"email\":\"" <> email <> "\",\"display_name\":\"" <> display_name <> "\",\"role\":\"" <> role <> "\",\"joined_at\":\"" <> joined_at <> "\"}"
end

# Serialize an API key row to JSON.
# Fields: id, project_id, key_value, label, created_at, revoked_at
# If revoked_at is empty string, emit null instead of quoted empty string.
fn api_key_to_json(row) -> String do
  let id = Map.get(row, "id")
  let project_id = Map.get(row, "project_id")
  let key_value = Map.get(row, "key_value")
  let label = Map.get(row, "label")
  let created_at = Map.get(row, "created_at")
  let revoked_at = Map.get(row, "revoked_at")
  let revoked_str = if String.length(revoked_at) == 0 do "null" else "\"" <> revoked_at <> "\"" end
  "{\"id\":\"" <> id <> "\",\"project_id\":\"" <> project_id <> "\",\"key_value\":\"" <> key_value <> "\",\"label\":\"" <> label <> "\",\"created_at\":\"" <> created_at <> "\",\"revoked_at\":" <> revoked_str <> "}"
end


# Extract a field from a JSON body using Mesh-native Json.get (no DB roundtrip).
# Returns the value or empty string if field is missing/null.
fn extract_json_field(pool :: PoolHandle, body :: String, field :: String) -> String!String do
  Ok(Json.get(body, field))
end

# --- Team membership helper functions for case arm extraction (ORG-04) ---

# Helper: handle successful add_member result.
fn add_member_success(membership_id :: String) do
  HTTP.response(201, json { id: membership_id })
end

# Helper: handle successful update_member_role result.
fn update_role_success(n :: Int) do
  HTTP.response(200, json { status: "ok", affected: n })
end

# Helper: handle successful remove_member result.
fn remove_success(n :: Int) do
  HTTP.response(200, json { status: "ok", affected: n })
end

# --- API token helper functions for case arm extraction (ORG-05) ---

# Helper: handle successful create_api_key result.
fn create_key_success(key_value :: String) do
  HTTP.response(201, json { key_value: key_value })
end

# Helper: handle successful revoke_api_key result.
fn revoke_key_success(n :: Int) do
  HTTP.response(200, json { status: "ok", affected: n })
end

# --- Add member helper chain ---

# Helper: perform add_member after extracting user_id and role from body.
fn do_add_member(pool :: PoolHandle, org_id :: String, user_id :: String, role :: String) do
  let result = add_member(pool, user_id, org_id, role)
  case result do
    Ok(id) -> add_member_success(id)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Helper: extract role and dispatch add_member.
fn add_member_with_role(pool :: PoolHandle, org_id :: String, user_id :: String, body :: String) do
  let role_result = extract_json_field(pool, body, "role")
  case role_result do
    Ok(role) -> do_add_member(pool, org_id, user_id, if String.length(role) == 0 do "member" else role end)
    Err(e) -> HTTP.response(400, json { error: "invalid json" })
  end
end

# Helper: check user_id is non-empty.
fn check_user_id(pool :: PoolHandle, org_id :: String, user_id :: String, body :: String) do
  if String.length(user_id) == 0 do
    HTTP.response(400, json { error: "user_id is required" })
  else
    add_member_with_role(pool, org_id, user_id, body)
  end
end

# Helper: validate user_id and dispatch add member.
fn validate_add_member(pool :: PoolHandle, org_id :: String, body :: String) do
  let uid_result = extract_json_field(pool, body, "user_id")
  case uid_result do
    Ok(user_id) -> check_user_id(pool, org_id, user_id, body)
    Err(e) -> HTTP.response(400, json { error: "invalid json" })
  end
end

# --- Update member role helper chain ---

# Helper: perform the actual role update.
fn perform_role_update(pool :: PoolHandle, membership_id :: String, role :: String) do
  let result = update_member_role(pool, membership_id, role)
  case result do
    Ok(n) -> update_role_success(n)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Helper: extract role and perform update.
fn do_update_role(pool :: PoolHandle, membership_id :: String, body :: String) do
  let role_result = extract_json_field(pool, body, "role")
  case role_result do
    Ok(role) -> perform_role_update(pool, membership_id, role)
    Err(e) -> HTTP.response(400, json { error: "invalid json" })
  end
end

# --- Create API key helper chain ---

# Helper: perform the actual key creation.
fn perform_create_key(pool :: PoolHandle, project_id :: String, label :: String) do
  let result = create_api_key(pool, project_id, label)
  case result do
    Ok(key_value) -> create_key_success(key_value)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Helper: extract label and perform key creation.
fn do_create_key(pool :: PoolHandle, project_id :: String, body :: String) do
  let label_result = extract_json_field(pool, body, "label")
  case label_result do
    Ok(label) -> perform_create_key(pool, project_id, if String.length(label) == 0 do "default" else label end)
    Err(e) -> HTTP.response(400, json { error: "invalid json" })
  end
end

# --- Handler functions (pub, defined after all helpers) ---

# Handle GET /api/v1/orgs/:org_id/members
# Lists all members of an organization with user info (email, display_name).
pub fn handle_list_members(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let org_id = require_param(request, "org_id")
  let result = get_members_with_users(pool, org_id)
  case result do
    Ok(rows) -> HTTP.response(200, rows |> List.map(fn(row) do member_to_json(row) end) |> to_json_array())
    Err(e) -> HTTP.response(500, json { error: e })
  end
end

# Handle POST /api/v1/orgs/:org_id/members
# Adds a member to an organization. Body: {"user_id":"...","role":"member"}
# role defaults to "member" if omitted.
pub fn handle_add_member(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let org_id = require_param(request, "org_id")
  let body = Request.body(request)
  validate_add_member(pool, org_id, body)
end

# Handle POST /api/v1/orgs/:org_id/members/:membership_id/role
# Updates a member's role. Body: {"role":"admin"}
pub fn handle_update_member_role(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let membership_id = require_param(request, "membership_id")
  let body = Request.body(request)
  do_update_role(pool, membership_id, body)
end

# Handle POST /api/v1/orgs/:org_id/members/:membership_id/remove
# Removes a member from an organization.
pub fn handle_remove_member(request) do
  let reg_pid = get_registry()
  let pool = PipelineRegistry.get_pool(reg_pid)
  let membership_id = require_param(request, "membership_id")
  let result = remove_member(pool, membership_id)
  case result do
    Ok(n) -> remove_success(n)
    Err(e) -> HTTP.response(500, json { error: e })
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
    Ok(rows) -> HTTP.response(200, rows |> List.map(fn(row) do api_key_to_json(row) end) |> to_json_array())
    Err(e) -> HTTP.response(500, json { error: e })
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
    Ok(n) -> revoke_key_success(n)
    Err(e) -> HTTP.response(500, json { error: e })
  end
end
