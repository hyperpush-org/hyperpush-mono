use super::m046_route_free as route_free;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const RETAINED_FIXTURE_ROOT_RELATIVE: &str = "scripts/fixtures/backend/reference-backend";
pub const RETAINED_FIXTURE_API_RELATIVE: &str =
    "scripts/fixtures/backend/reference-backend/api";
pub const RETAINED_FIXTURE_HEALTH_RELATIVE: &str =
    "scripts/fixtures/backend/reference-backend/api/health.mpl";
pub const RETAINED_FIXTURE_JOBS_RELATIVE: &str =
    "scripts/fixtures/backend/reference-backend/api/jobs.mpl";
pub const RETAINED_FIXTURE_JOB_TYPE_RELATIVE: &str =
    "scripts/fixtures/backend/reference-backend/types/job.mpl";
pub const RETAINED_FIXTURE_TESTS_RELATIVE: &str =
    "scripts/fixtures/backend/reference-backend/tests";
pub const RETAINED_FIXTURE_CONFIG_TEST_RELATIVE: &str =
    "scripts/fixtures/backend/reference-backend/tests/config.test.mpl";
pub const RETAINED_FIXTURE_FIXTURE_TEST_RELATIVE: &str =
    "scripts/fixtures/backend/reference-backend/tests/fixture.test.mpl";
pub const RETAINED_FORMATTER_DIR_RELATIVE: &str = RETAINED_FIXTURE_API_RELATIVE;
pub const LEGACY_COMPAT_ROOT_RELATIVE: &str = "reference-backend";
pub const PACKAGE_NAME: &str = "reference-backend";
pub const STAGED_BUNDLE_FILES: &[&str] = &[
    "reference-backend",
    "reference-backend.up.sql",
    "apply-deploy-migrations.sh",
    "deploy-smoke.sh",
];
pub const REQUIRED_FIXTURE_FILES: &[&str] = &[
    "mesh.toml",
    "main.mpl",
    "README.md",
    "scripts/stage-deploy.sh",
    "scripts/smoke.sh",
    "scripts/apply-deploy-migrations.sh",
    "scripts/deploy-smoke.sh",
    "deploy/reference-backend.up.sql",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildOutputMetadata {
    pub source_package_dir: PathBuf,
    pub binary_path: PathBuf,
}

pub fn repo_root() -> PathBuf {
    route_free::repo_root()
}

pub fn meshc_bin() -> PathBuf {
    route_free::meshc_bin()
}

pub fn ensure_mesh_rt_staticlib() {
    route_free::ensure_mesh_rt_staticlib();
}

pub fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m051-s02", test_name)
}

pub fn write_artifact(path: &Path, contents: impl AsRef<str>) {
    route_free::write_artifact(path, contents.as_ref());
}

pub fn write_json_artifact(path: &Path, value: &impl Serialize) {
    route_free::write_json_artifact(path, value);
}

pub fn command_output_text(output: &Output) -> String {
    route_free::command_output_text(output)
}

pub fn read_source_file(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

pub fn validate_retained_fixture_root(root: &Path) -> Result<(), String> {
    if !root.is_dir() {
        return Err(format!(
            "reference-backend fixture root {} is missing; expected {}",
            root.display(),
            RETAINED_FIXTURE_ROOT_RELATIVE
        ));
    }

    let missing_files = REQUIRED_FIXTURE_FILES
        .iter()
        .copied()
        .filter(|relative_path| !root.join(relative_path).is_file())
        .collect::<Vec<_>>();
    if !missing_files.is_empty() {
        return Err(format!(
            "reference-backend fixture root {} is missing required files: {}",
            root.display(),
            missing_files.join(", ")
        ));
    }

    let manifest_path = root.join("mesh.toml");
    let manifest = fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "failed to read reference-backend fixture manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let expected_package_line = format!("name = \"{PACKAGE_NAME}\"");
    if !manifest.contains(&expected_package_line) {
        return Err(format!(
            "reference-backend fixture root {} is not the expected package directory; {} does not contain {}",
            root.display(),
            manifest_path.display(),
            expected_package_line
        ));
    }

    Ok(())
}

pub fn retained_fixture_root() -> PathBuf {
    let root = repo_root().join(RETAINED_FIXTURE_ROOT_RELATIVE);
    validate_retained_fixture_root(&root).unwrap_or_else(|message| panic!("{message}"));
    root
}

pub fn retained_api_dir() -> PathBuf {
    retained_fixture_root().join("api")
}

pub fn retained_health_path() -> PathBuf {
    retained_fixture_root().join("api/health.mpl")
}

pub fn retained_jobs_path() -> PathBuf {
    retained_fixture_root().join("api/jobs.mpl")
}

pub fn retained_job_type_path() -> PathBuf {
    retained_fixture_root().join("types/job.mpl")
}

pub fn retained_tests_dir() -> PathBuf {
    retained_fixture_root().join("tests")
}

pub fn retained_config_test_path() -> PathBuf {
    retained_fixture_root().join("tests/config.test.mpl")
}

pub fn retained_fixture_test_path() -> PathBuf {
    retained_fixture_root().join("tests/fixture.test.mpl")
}

pub fn retained_formatter_dir() -> PathBuf {
    retained_fixture_root().join("api")
}

pub fn retained_manifest_path() -> PathBuf {
    retained_fixture_root().join("mesh.toml")
}

pub fn retained_readme_path() -> PathBuf {
    retained_fixture_root().join("README.md")
}

pub fn retained_stage_deploy_script_path() -> PathBuf {
    retained_fixture_root().join("scripts/stage-deploy.sh")
}

pub fn retained_smoke_script_path() -> PathBuf {
    retained_fixture_root().join("scripts/smoke.sh")
}

pub fn legacy_repo_root_binary_path() -> PathBuf {
    repo_root()
        .join(LEGACY_COMPAT_ROOT_RELATIVE)
        .join(PACKAGE_NAME)
}

pub fn runtime_build_dir() -> PathBuf {
    repo_root().join(".tmp").join("m051-s02").join("reference-backend-runtime")
}

pub fn runtime_binary_path() -> PathBuf {
    runtime_build_dir().join(PACKAGE_NAME)
}

pub fn runtime_build_metadata_path() -> PathBuf {
    runtime_build_dir().join("build-output.json")
}

pub fn build_reference_backend() -> Output {
    ensure_mesh_rt_staticlib();

    let build_dir = runtime_build_dir();
    fs::create_dir_all(&build_dir)
        .unwrap_or_else(|error| panic!("failed to create {}: {error}", build_dir.display()));

    let binary_path = runtime_binary_path();
    let _ = fs::remove_file(&binary_path);

    let output = Command::new(meshc_bin())
        .current_dir(repo_root())
        .arg("build")
        .arg(RETAINED_FIXTURE_ROOT_RELATIVE)
        .arg("--output")
        .arg(&binary_path)
        .output()
        .expect("failed to invoke meshc build for retained reference-backend fixture");

    if output.status.success() {
        assert!(
            binary_path.is_file(),
            "meshc build reported success but runtime binary is missing at {}",
            binary_path.display()
        );
        write_json_artifact(
            &runtime_build_metadata_path(),
            &BuildOutputMetadata {
                source_package_dir: retained_fixture_root(),
                binary_path: binary_path.clone(),
            },
        );
    }

    output
}

pub fn run_reference_backend_migration(database_url: &str, command: &str) -> Output {
    Command::new(meshc_bin())
        .current_dir(repo_root())
        .env("DATABASE_URL", database_url)
        .arg("migrate")
        .arg(RETAINED_FIXTURE_ROOT_RELATIVE)
        .arg(command)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "failed to invoke meshc migrate {} {}: {}",
                RETAINED_FIXTURE_ROOT_RELATIVE,
                command,
                error
            )
        })
}

pub fn run_reference_backend_smoke_script(
    database_url: &str,
    port: u16,
    job_poll_ms: u64,
) -> Output {
    Command::new("bash")
        .current_dir(repo_root())
        .arg(retained_smoke_script_path())
        .env("DATABASE_URL", database_url)
        .env("PORT", port.to_string())
        .env("JOB_POLL_MS", job_poll_ms.to_string())
        .output()
        .expect("failed to invoke retained reference-backend smoke script")
}

pub fn run_reference_backend_stage_deploy_script(bundle_dir: &Path) -> Output {
    Command::new("bash")
        .current_dir(repo_root())
        .arg(retained_stage_deploy_script_path())
        .arg(bundle_dir)
        .output()
        .expect("failed to invoke retained reference-backend stage-deploy script")
}

pub fn assert_command_success(output: &Output, description: &str) {
    assert!(
        output.status.success(),
        "{description} failed:\n{}",
        command_output_text(output)
    );
}

pub fn assert_is_executable(path: &Path) {
    let metadata =
        fs::metadata(path).unwrap_or_else(|error| panic!("failed to stat {}: {error}", path.display()));
    assert!(metadata.is_file(), "expected file at {}", path.display());
    assert!(
        metadata.permissions().mode() & 0o111 != 0,
        "expected executable permissions at {}",
        path.display()
    );
}

pub fn assert_staged_bundle_dir_outside_repo_root(bundle_dir: &Path) {
    let bundle_dir = fs::canonicalize(bundle_dir)
        .unwrap_or_else(|error| panic!("failed to canonicalize {}: {error}", bundle_dir.display()));
    let repo_root = fs::canonicalize(repo_root()).expect("failed to canonicalize repo root");
    assert!(
        !bundle_dir.starts_with(&repo_root),
        "staged bundle dir must live outside the repo root; bundle_dir={} repo_root={}",
        bundle_dir.display(),
        repo_root.display()
    );
}

pub fn assert_runtime_binary_outside_legacy_tree(binary_path: &Path) {
    let expected_root = runtime_build_dir();
    assert!(
        binary_path.starts_with(&expected_root),
        "retained runtime binary should live under {} but was {}",
        expected_root.display(),
        binary_path.display()
    );
    assert_ne!(
        binary_path,
        legacy_repo_root_binary_path().as_path(),
        "retained runtime binary must not reuse the repo-root compatibility binary path"
    );
}

pub fn publish_bundle_pointer(artifacts: &Path, bundle_dir: &Path) -> PathBuf {
    let pointer_path = artifacts.join("latest-proof-bundle.txt");
    write_artifact(&pointer_path, format!("{}\n", bundle_dir.display()));
    pointer_path
}

pub fn write_bundle_manifest(bundle_dir: &Path, manifest_path: &Path) {
    let manifest = STAGED_BUNDLE_FILES
        .iter()
        .map(|relative_path| bundle_dir.join(relative_path).display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    write_artifact(manifest_path, format!("{manifest}\n"));
}
