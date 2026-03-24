# User service module.
# Thin actor wrapper for user auth, sessions, and org membership.
# Delegates to query functions in Storage.Queries.

from Storage.Queries import (
  create_user,
  authenticate_user,
  get_user,
  create_session,
  validate_session,
  delete_session,
  add_member,
  get_members
)
from Types.User import User, Session, OrgMembership

# Helper function for two-step login: authenticate then create session.
# Extracted from UserService.Login handler to avoid complex case expressions
# inside service call bodies (LLVM codegen limitation with Result pattern
# matching inside service dispatch handlers).
fn login_user(pool :: PoolHandle, email :: String, password :: String) -> String!String do
  let auth_result = authenticate_user(pool, email, password)
  case auth_result do
    Ok(user) -> create_session(pool, user.id)
    Err(_) -> Err("authentication failed")
  end
end

service UserService do
  fn init(pool :: PoolHandle) -> PoolHandle do
    pool
  end

  call Register(email :: String, password :: String, display_name :: String) do |pool|
    let result = create_user(pool, email, password, display_name)
    (pool, result)
  end

  call Login(email :: String, password :: String) do |pool|
    let result = login_user(pool, email, password)
    (pool, result)
  end

  call ValidateSession(token :: String) do |pool|
    let result = validate_session(pool, token)
    (pool, result)
  end

  call Logout(token :: String) do |pool|
    let result = delete_session(pool, token)
    (pool, result)
  end

  call GetUser(id :: String) do |pool|
    let result = get_user(pool, id)
    (pool, result)
  end

  call AddMember(user_id :: String, org_id :: String, role :: String) do |pool|
    let result = add_member(pool, user_id, org_id, role)
    (pool, result)
  end

  call GetMembers(org_id :: String) do |pool|
    let result = get_members(pool, org_id)
    (pool, result)
  end
end
