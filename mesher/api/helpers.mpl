# Shared helper functions for API modules.
# Provides common utilities used across search, dashboard, and team handlers.
# All functions are pub for cross-module import.

from Storage.Queries import get_project_id_by_slug, get_org_id_by_slug

# Cluster-aware registry lookup.
# In cluster mode (Node.self returns non-empty), uses Global.whereis for
# cross-node discovery. In standalone mode, uses Process.whereis (zero overhead).
# Both return Pid<()>; the runtime representation is u64 in either case.

pub fn get_registry() do
  let node_name = Node.self()
  if node_name != "" do
    Global.whereis("mesher_registry")
  else
    Process.whereis("mesher_registry")
  end
end

# Resolve a project identifier to a UUID.
# If the identifier is 36 chars (UUID format), returns it directly.
# Otherwise, treats it as a slug and looks up the project UUID from the database.
# Returns the UUID string on success, or an empty string if slug not found.

pub fn resolve_project_id(pool :: PoolHandle, raw_id :: String) -> String do
  if String.length(raw_id) == 36 do
    raw_id
  else
    let result = get_project_id_by_slug(pool, raw_id)
    case result do
      Ok( uuid) -> uuid
      Err -> ""
    end
  end
end

# Resolve an org identifier to a UUID.
# If the identifier is 36 chars (UUID format), returns it directly.
# Otherwise, treats it as a slug and looks up the org UUID from the database.
# Returns the UUID string on success, or an empty string if slug not found.

pub fn resolve_org_id(pool :: PoolHandle, raw_id :: String) -> String do
  if String.length(raw_id) == 36 do
    raw_id
  else
    let result = get_org_id_by_slug(pool, raw_id)
    case result do
      Ok( uuid) -> uuid
      Err -> ""
    end
  end
end

# Extract optional query parameter with a default value.
# Request.query returns Option<String>; case match to Some/None.

pub fn query_or_default(request, param :: String, default :: String) -> String do
  let opt = Request.query(request, param)
  case opt do
    Some( v) -> v
    None -> default
  end
end

# Extract a required path parameter.
# Request.param returns Option<String>; route matching guarantees existence.

pub fn require_param(request, name :: String) -> String do
  let opt = Request.param(request, name)
  case opt do
    Some( v) -> v
    None -> ""
  end
end

# Convert a list of JSON strings to a JSON array.
# Replaces the old recursive json_array_loop pattern with String.join.

pub fn to_json_array(items) -> String do
  "[#{String.join(items, ",")}]"
end
