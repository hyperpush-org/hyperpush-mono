# Project service module.
# Thin actor wrapper for project CRUD and API key management.
# Delegates to query functions in Storage.Queries.

from Storage.Queries import (
  insert_project,
  get_project,
  list_projects_by_org,
  create_api_key,
  get_project_by_api_key,
  revoke_api_key
)
from Types.Project import Project

service ProjectService do
  fn init(pool :: PoolHandle) -> PoolHandle do
    pool
  end

  call CreateProject(org_id :: String, name :: String, platform :: String) do |pool|
    let result = insert_project(pool, org_id, name, platform)
    (pool, result)
  end

  call GetProject(id :: String) do |pool|
    let result = get_project(pool, id)
    (pool, result)
  end

  call ListProjectsByOrg(org_id :: String) do |pool|
    let result = list_projects_by_org(pool, org_id)
    (pool, result)
  end

  call CreateApiKey(project_id :: String, label :: String) do |pool|
    let result = create_api_key(pool, project_id, label)
    (pool, result)
  end

  call GetProjectByApiKey(key_value :: String) do |pool|
    let result = get_project_by_api_key(pool, key_value)
    (pool, result)
  end

  call RevokeApiKey(key_id :: String) do |pool|
    let result = revoke_api_key(pool, key_id)
    (pool, result)
  end
end
