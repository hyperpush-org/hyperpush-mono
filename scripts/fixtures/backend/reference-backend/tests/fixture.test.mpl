import File

fn read_required_file(repo_relative :: String, package_relative :: String, tests_relative :: String) -> String do
  case File.read(repo_relative) do
    Ok(contents) -> contents
    Err( _) -> do
      case File.read(package_relative) do
        Ok(contents) -> contents
        Err( _) -> do
          case File.read(tests_relative) do
            Ok(contents) -> contents
            Err(message) -> do
              assert(false)
              message
            end
          end
        end
      end
    end
  end
end

fn assert_contains(haystack :: String, needle :: String) do
  assert(String.contains(haystack, needle))
end

fn assert_not_contains(haystack :: String, needle :: String) do
  assert(String.contains(haystack, needle) == false)
end

fn file_missing(repo_relative :: String, package_relative :: String, tests_relative :: String) -> Bool do
  let repo_missing = case File.read(repo_relative) do
    Ok( _) -> false
    Err( _) -> true
  end
  let package_missing = case File.read(package_relative) do
    Ok( _) -> false
    Err( _) -> true
  end
  let tests_missing = case File.read(tests_relative) do
    Ok( _) -> false
    Err( _) -> true
  end
  if repo_missing do
    if package_missing do
      tests_missing
    else
      false
    end
  else
    false
  end
end

describe("retained reference-backend fixture") do
  test("manifest and copied backend assets preserve package identity") do
    let manifest = read_required_file("scripts/fixtures/backend/reference-backend/mesh.toml", "mesh.toml", "../mesh.toml")
    let config_tests = read_required_file(
      "scripts/fixtures/backend/reference-backend/tests/config.test.mpl",
      "tests/config.test.mpl",
      "config.test.mpl"
    )
    let migration = read_required_file(
      "scripts/fixtures/backend/reference-backend/migrations/20260323010000_create_jobs.mpl",
      "migrations/20260323010000_create_jobs.mpl",
      "../migrations/20260323010000_create_jobs.mpl"
    )
    let deploy_sql = read_required_file(
      "scripts/fixtures/backend/reference-backend/deploy/reference-backend.up.sql",
      "deploy/reference-backend.up.sql",
      "../deploy/reference-backend.up.sql"
    )

    assert_contains(manifest, "name = \"reference-backend\"")
    assert_contains(config_tests, "database_url_key()")
    assert_contains(migration, "CREATE TABLE IF NOT EXISTS jobs")
    assert_contains(deploy_sql, "_mesh_migrations")
    assert_contains(deploy_sql, "20260323010000")
    assert_contains(deploy_sql, "scripts/fixtures/backend/reference-backend/migrations/20260323010000_create_jobs.mpl")
  end

  test("readme marks the fixture as internal and post-deletion authoritative") do
    let readme = read_required_file("scripts/fixtures/backend/reference-backend/README.md", "README.md", "../README.md")

    assert_contains(readme, "canonical maintainer runbook")
    assert_contains(readme, "maintainer-only/internal fixture")
    assert_contains(readme, "sole in-repo backend-only proof surface")
    assert_contains(readme, "repo-root `reference-backend/` compatibility tree was deleted")
    assert_contains(readme, "cargo run -q -p meshc -- test scripts/fixtures/backend/reference-backend/tests")
    assert_contains(readme, "scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh")
    assert_contains(readme, "bash scripts/verify-production-proof-surface.sh")
    assert_not_contains(readme, "reference-backend/README.md")
    assert_not_contains(readme, "Do not delete or retarget the repo-root compatibility path in this slice")
    assert_not_contains(readme, "## Compatibility boundary")
  end

  test("stage and smoke scripts build from the fixture into external artifacts") do
    let stage_deploy = read_required_file(
      "scripts/fixtures/backend/reference-backend/scripts/stage-deploy.sh",
      "scripts/stage-deploy.sh",
      "../scripts/stage-deploy.sh"
    )
    let smoke = read_required_file(
      "scripts/fixtures/backend/reference-backend/scripts/smoke.sh",
      "scripts/smoke.sh",
      "../scripts/smoke.sh"
    )

    assert_contains(stage_deploy, "PACKAGE_REL=\"scripts/fixtures/backend/reference-backend\"")
    assert_contains(stage_deploy, "--output \"$TARGET_BINARY\"")
    assert_contains(stage_deploy, "ensure_source_tree_clean")
    assert_not_contains(stage_deploy, "SOURCE_BINARY=\"$ROOT/reference-backend/reference-backend\"")
    assert_not_contains(stage_deploy, "cargo run -p meshc -- build reference-backend")

    assert_contains(smoke, "ARTIFACT_DIR=\"$ROOT/.tmp/m051-s02/fixture-smoke\"")
    assert_contains(smoke, "--output \"$BINARY_PATH\"")
    assert_contains(smoke, "bash $PACKAGE_REL/scripts/apply-deploy-migrations.sh $PACKAGE_REL/deploy/reference-backend.up.sql")
    assert_not_contains(smoke, "./reference-backend/reference-backend")
    assert_not_contains(smoke, "bash \"$ROOT/reference-backend/scripts/deploy-smoke.sh\"")
  end

  test("repo-root compatibility leftovers and in-place binaries stay absent") do
    assert(file_missing(
      "reference-backend/README.md",
      "reference-backend/README.md",
      "../reference-backend/README.md"
    ))
    assert(file_missing(
      "scripts/fixtures/backend/reference-backend/reference-backend",
      "reference-backend",
      "../reference-backend"
    ))
    assert(file_missing(
      "scripts/fixtures/backend/reference-backend/scripts/verify-production-proof-surface.sh",
      "scripts/fixtures/backend/reference-backend/scripts/verify-production-proof-surface.sh",
      "../scripts/verify-production-proof-surface.sh"
    ))
  end
end
