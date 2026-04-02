mod support;

use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use support::m046_route_free as route_free;

const BUILD_TIMEOUT: Duration = Duration::from_secs(90);
const TEST_TIMEOUT: Duration = Duration::from_secs(90);
const RUN_TIMEOUT: Duration = Duration::from_secs(15);
const DEFAULT_ENTRYPOINT: &str = "main.mpl";

#[derive(Debug)]
struct FixtureProject {
    _tempdir: tempfile::TempDir,
    project_dir: PathBuf,
    tests_dir: PathBuf,
    test_file: Option<PathBuf>,
    binary_path: PathBuf,
}

struct FixtureSpec<'a> {
    manifest: Option<&'a str>,
    expected_entrypoint: &'a str,
    files: &'a [(&'a str, &'a str)],
    test_file: Option<&'a str>,
}

struct TimedRunError {
    message: String,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    status_code: Option<i32>,
}

fn artifact_dir(test_name: &str) -> PathBuf {
    route_free::artifact_dir("m048-s01", test_name)
}

fn write_file(path: &Path, contents: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!("File '{}' is missing a parent directory", path.display()));
    };
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create '{}': {error}", parent.display()))?;
    fs::write(path, contents).map_err(|error| format!("Failed to write '{}': {error}", path.display()))
}

fn validate_relative_fixture_path(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Err(format!(
            "Fixture path '{}' must stay relative to the temp project root",
            path.display()
        ));
    }

    for component in path.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "Fixture path '{}' escapes the temp project root",
                    path.display()
                ));
            }
            Component::CurDir | Component::Normal(_) => {}
        }
    }

    Ok(())
}

fn ensure_clean_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        if !path.is_dir() {
            return Err(format!(
                "Refusing to reuse invalid retained artifact state at '{}': expected a directory, found a file.",
                path.display()
            ));
        }
        fs::remove_dir_all(path)
            .map_err(|error| format!("Failed to clear '{}': {error}", path.display()))?;
    }
    fs::create_dir_all(path).map_err(|error| format!("Failed to create '{}': {error}", path.display()))
}

fn write_fixture_project(
    name: &str,
    spec: &FixtureSpec<'_>,
    artifacts: &Path,
) -> Result<FixtureProject, String> {
    let tempdir = tempfile::tempdir().map_err(|error| format!("Failed to create temp dir: {error}"))?;
    let project_dir = tempdir.path().join(name);
    fs::create_dir_all(&project_dir)
        .map_err(|error| format!("Failed to create '{}': {error}", project_dir.display()))?;

    if let Some(manifest) = spec.manifest {
        write_file(&project_dir.join("mesh.toml"), manifest)?;
    }

    for (relative_path, contents) in spec.files {
        validate_relative_fixture_path(relative_path)?;
        write_file(&project_dir.join(relative_path), contents)?;
    }

    let expected_entry = project_dir.join(spec.expected_entrypoint);
    if !expected_entry.is_file() {
        return Err(format!(
            "Fixture '{}' is missing expected executable entry '{}'; refusing to run a malformed acceptance project.",
            name,
            spec.expected_entrypoint
        ));
    }

    let project_artifacts = artifacts.join("project");
    ensure_clean_dir(&project_artifacts)?;
    route_free::archive_directory_tree(&project_dir, &project_artifacts);

    let tests_dir = project_dir.join("tests");
    let binary_path = tempdir.path().join("out").join(name);
    let test_file = spec.test_file.map(|relative| project_dir.join(relative));

    Ok(FixtureProject {
        _tempdir: tempdir,
        project_dir,
        tests_dir,
        test_file,
        binary_path,
    })
}

fn package_manifest(name: &str, entrypoint: &str) -> String {
    format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nentrypoint = \"{entrypoint}\"\n"
    )
}

fn meshc_command() -> Command {
    let mut command = Command::new(route_free::meshc_bin());
    command.current_dir(route_free::repo_root());
    command
}

fn describe_command(command: &Command, description: &str, timeout: Duration) -> String {
    let cwd = command
        .get_current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<inherit>".to_string());
    let program = command.get_program().to_string_lossy();
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        "description: {description}\ntimeout: {timeout:?}\ncwd: {cwd}\ncommand: {program} {args}\n"
    )
}

fn read_child_pipes(child: &mut Child) -> (Vec<u8>, Vec<u8>) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    if let Some(mut out) = child.stdout.take() {
        out.read_to_end(&mut stdout).ok();
    }
    if let Some(mut err) = child.stderr.take() {
        err.read_to_end(&mut stderr).ok();
    }

    (stdout, stderr)
}

fn wait_with_timeout(
    mut child: Child,
    timeout: Duration,
    description: &str,
) -> Result<Output, TimedRunError> {
    let start = Instant::now();
    let poll_interval = Duration::from_millis(50);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let (stdout, stderr) = read_child_pipes(&mut child);
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let status = child.wait().ok();
                    let (stdout, stderr) = read_child_pipes(&mut child);
                    return Err(TimedRunError {
                        message: format!("{description} timed out after {timeout:?}"),
                        stdout,
                        stderr,
                        status_code: status.and_then(|status| status.code()),
                    });
                }
                sleep(poll_interval);
            }
            Err(error) => {
                let (stdout, stderr) = read_child_pipes(&mut child);
                return Err(TimedRunError {
                    message: format!("Failed to wait on {description}: {error}"),
                    stdout,
                    stderr,
                    status_code: None,
                });
            }
        }
    }
}

fn archive_command_output(artifacts: &Path, label: &str, output: &Output) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = route_free::command_output_text(output);

    route_free::write_artifact(&artifacts.join(format!("{label}.stdout.log")), &stdout);
    route_free::write_artifact(&artifacts.join(format!("{label}.stderr.log")), &stderr);
    route_free::write_artifact(&artifacts.join(format!("{label}.combined.log")), &combined);
    route_free::write_artifact(
        &artifacts.join(format!("{label}.status.txt")),
        format!("{:?}\n", output.status.code()),
    );
}

fn run_command_and_archive(
    command: &mut Command,
    artifacts: &Path,
    label: &str,
    timeout: Duration,
    description: &str,
) -> Output {
    route_free::write_artifact(
        &artifacts.join(format!("{label}.command.txt")),
        describe_command(command, description, timeout),
    );

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let child = command
        .spawn()
        .unwrap_or_else(|error| panic!("failed to spawn {description}: {error}"));

    match wait_with_timeout(child, timeout, description) {
        Ok(output) => {
            archive_command_output(artifacts, label, &output);
            output
        }
        Err(error) => {
            let stdout = String::from_utf8_lossy(&error.stdout);
            let stderr = String::from_utf8_lossy(&error.stderr);
            route_free::write_artifact(&artifacts.join(format!("{label}.stdout.log")), &stdout);
            route_free::write_artifact(&artifacts.join(format!("{label}.stderr.log")), &stderr);
            route_free::write_artifact(
                &artifacts.join(format!("{label}.timeout.txt")),
                format!(
                    "{}\nstatus: {:?}\nstdout:\n{}\nstderr:\n{}",
                    error.message, error.status_code, stdout, stderr
                ),
            );
            panic!("{description} timed out; artifacts: {}", artifacts.display());
        }
    }
}

fn build_fixture_binary(project: &FixtureProject, artifacts: &Path, label: &str) -> Output {
    route_free::ensure_mesh_rt_staticlib();
    let output_parent = project
        .binary_path
        .parent()
        .expect("binary path should have a parent directory");
    fs::create_dir_all(output_parent)
        .unwrap_or_else(|error| panic!("failed to create '{}': {error}", output_parent.display()));

    let mut command = meshc_command();
    command
        .arg("build")
        .arg(&project.project_dir)
        .arg("--output")
        .arg(&project.binary_path);

    run_command_and_archive(
        &mut command,
        artifacts,
        label,
        BUILD_TIMEOUT,
        &format!("meshc build {}", project.project_dir.display()),
    )
}

fn run_fixture_binary(project: &FixtureProject, artifacts: &Path, label: &str) -> Output {
    assert!(
        project.binary_path.exists(),
        "expected built binary at {} before execution; artifacts: {}",
        project.binary_path.display(),
        artifacts.display()
    );

    let mut command = Command::new(&project.binary_path);
    command.current_dir(&project.project_dir);

    run_command_and_archive(
        &mut command,
        artifacts,
        label,
        RUN_TIMEOUT,
        &format!("run {}", project.binary_path.display()),
    )
}

fn run_meshc_test(target: &Path, artifacts: &Path, label: &str) -> Output {
    let mut command = meshc_command();
    command.arg("test").arg(target);

    run_command_and_archive(
        &mut command,
        artifacts,
        label,
        TEST_TIMEOUT,
        &format!("meshc test {}", target.display()),
    )
}

fn assert_success(output: &Output, context: &str, artifacts: &Path) {
    assert!(
        output.status.success(),
        "{context} failed; artifacts: {}\n{}",
        artifacts.display(),
        route_free::command_output_text(output)
    );
}

fn assert_stdout_contains(output: &Output, expected: &str, context: &str, artifacts: &Path) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(expected),
        "{context} missing stdout marker {expected:?}; artifacts: {}\n{}",
        artifacts.display(),
        route_free::command_output_text(output)
    );
}

fn assert_stdout_not_contains(output: &Output, unexpected: &str, context: &str, artifacts: &Path) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains(unexpected),
        "{context} unexpectedly contained stdout marker {unexpected:?}; artifacts: {}\n{}",
        artifacts.display(),
        route_free::command_output_text(output)
    );
}

fn default_control_fixture(artifacts: &Path) -> FixtureProject {
    let spec = FixtureSpec {
        manifest: None,
        expected_entrypoint: DEFAULT_ENTRYPOINT,
        files: &[
            (
                "main.mpl",
                "from Support import label\n\nfn main() do\n  println(\"proof=default-control label=#{label()}\")\nend\n",
            ),
            (
                "support.mpl",
                "pub fn label() -> String do\n  \"default-support\"\nend\n",
            ),
        ],
        test_file: None,
    };

    write_fixture_project("default-control-project", &spec, artifacts).unwrap_or_else(|error| {
        route_free::write_artifact(&artifacts.join("setup.error.txt"), &error);
        panic!("default control fixture should materialize: {error}");
    })
}

fn override_precedence_fixture(artifacts: &Path) -> FixtureProject {
    let manifest = package_manifest("override-precedence", "lib/start.mpl");
    let spec = FixtureSpec {
        manifest: Some(&manifest),
        expected_entrypoint: "lib/start.mpl",
        files: &[
            (
                "main.mpl",
                "fn main() do\n  println(\"proof=root-main-should-not-run\")\nend\n",
            ),
            (
                "lib/start.mpl",
                "from App import label\n\nfn main() do\n  println(\"proof=override-wins label=#{label()}\")\nend\n",
            ),
            (
                "app.mpl",
                "pub fn label() -> String do\n  \"override-app\"\nend\n",
            ),
        ],
        test_file: None,
    };

    write_fixture_project("override-precedence-project", &spec, artifacts).unwrap_or_else(
        |error| {
            route_free::write_artifact(&artifacts.join("setup.error.txt"), &error);
            panic!("override precedence fixture should materialize: {error}");
        },
    )
}

fn override_only_build_fixture(artifacts: &Path) -> FixtureProject {
    let manifest = package_manifest("override-only-build", "lib/start.mpl");
    let spec = FixtureSpec {
        manifest: Some(&manifest),
        expected_entrypoint: "lib/start.mpl",
        files: &[
            (
                "lib/start.mpl",
                "from Lib.Support import label\n\nfn main() do\n  println(\"proof=override-only-build label=#{label()}\")\nend\n",
            ),
            (
                "lib/support.mpl",
                "pub fn label() -> String do\n  \"nested-support\"\nend\n",
            ),
        ],
        test_file: None,
    };

    write_fixture_project("override-only-build-project", &spec, artifacts).unwrap_or_else(
        |error| {
            route_free::write_artifact(&artifacts.join("setup.error.txt"), &error);
            panic!("override-only build fixture should materialize: {error}");
        },
    )
}

fn override_test_fixture(artifacts: &Path) -> FixtureProject {
    let manifest = package_manifest("override-entry-tests", "lib/start.mpl");
    let test_relative_path = "tests/override_entry.test.mpl";
    let spec = FixtureSpec {
        manifest: Some(&manifest),
        expected_entrypoint: "lib/start.mpl",
        files: &[
            (
                "lib/start.mpl",
                "from App import answer\n\nfn main() do\n  println(\"proof=app-entry answer=#{answer()}\")\nend\n",
            ),
            (
                "app.mpl",
                "pub fn answer() -> Int do\n  42\nend\n",
            ),
            (
                "tests/support.mpl",
                "pub fn label() -> String do\n  \"override-tests-support\"\nend\n",
            ),
            (
                test_relative_path,
                "from App import answer\nfrom Tests.Support import label\n\ntest(\"override entry roots meshc test targets\") do\n  println(\"proof=override-test answer=#{answer()} label=#{label()}\")\n  assert(answer() == 42)\n  assert(label() == \"override-tests-support\")\nend\n",
            ),
        ],
        test_file: Some(test_relative_path),
    };

    write_fixture_project("override-test-project", &spec, artifacts).unwrap_or_else(|error| {
        route_free::write_artifact(&artifacts.join("setup.error.txt"), &error);
        panic!("override test fixture should materialize: {error}");
    })
}

#[test]
fn m048_s01_fixture_writer_rejects_missing_override_entry_file() {
    let artifacts = artifact_dir("fixture-writer-rejects-missing-entry");
    let manifest = package_manifest("broken-override-entry", "lib/start.mpl");
    let spec = FixtureSpec {
        manifest: Some(&manifest),
        expected_entrypoint: "lib/start.mpl",
        files: &[("app.mpl", "pub fn answer() -> Int do\n  42\nend\n")],
        test_file: None,
    };

    let error = write_fixture_project("broken-override-entry", &spec, &artifacts)
        .expect_err("fixture writer should reject a missing override entry file");
    route_free::write_artifact(&artifacts.join("setup.error.txt"), &error);

    assert!(error.contains("lib/start.mpl"), "unexpected error: {error}");
    assert!(
        error.contains("malformed acceptance project"),
        "unexpected error: {error}"
    );
}

#[test]
fn m048_s01_artifact_writer_rejects_invalid_retained_artifact_state() {
    let tempdir = tempfile::tempdir().unwrap();
    let colliding_path = tempdir.path().join("project");
    fs::write(&colliding_path, "not-a-directory").unwrap();

    let error = ensure_clean_dir(&colliding_path)
        .expect_err("artifact setup should reject file collisions instead of reusing them");

    assert!(
        error.contains("invalid retained artifact state"),
        "unexpected error: {error}"
    );
}

#[test]
fn m048_s01_default_control_build_and_run_keep_root_main_behavior() {
    let artifacts = artifact_dir("default-control-build-and-run");
    let project = default_control_fixture(&artifacts);

    let build = build_fixture_binary(&project, &artifacts, "build");
    assert_success(
        &build,
        "default-control meshc build should succeed",
        &artifacts,
    );

    let run = run_fixture_binary(&project, &artifacts, "run");
    assert_success(&run, "default-control binary should run", &artifacts);
    assert_stdout_contains(
        &run,
        "proof=default-control label=default-support",
        "default-control binary should print the root-main proof marker",
        &artifacts,
    );
}

#[test]
fn m048_s01_override_precedence_build_and_run_prefers_manifest_entrypoint() {
    let artifacts = artifact_dir("override-precedence-build-and-run");
    let project = override_precedence_fixture(&artifacts);

    let build = build_fixture_binary(&project, &artifacts, "build");
    assert_success(
        &build,
        "override-precedence meshc build should succeed",
        &artifacts,
    );

    let run = run_fixture_binary(&project, &artifacts, "run");
    assert_success(&run, "override-precedence binary should run", &artifacts);
    assert_stdout_contains(
        &run,
        "proof=override-wins label=override-app",
        "override-precedence binary should execute the manifest entrypoint",
        &artifacts,
    );
    assert_stdout_not_contains(
        &run,
        "proof=root-main-should-not-run",
        "override-precedence binary should not execute the legacy root main",
        &artifacts,
    );
}

#[test]
fn m048_s01_override_only_build_and_run_succeeds_without_root_main() {
    let artifacts = artifact_dir("override-only-build-and-run");
    let project = override_only_build_fixture(&artifacts);

    let build = build_fixture_binary(&project, &artifacts, "build");
    assert_success(
        &build,
        "override-only meshc build should succeed without a root main.mpl",
        &artifacts,
    );

    let run = run_fixture_binary(&project, &artifacts, "run");
    assert_success(&run, "override-only binary should run", &artifacts);
    assert_stdout_contains(
        &run,
        "proof=override-only-build label=nested-support",
        "override-only binary should execute the nested override entrypoint",
        &artifacts,
    );
}

#[test]
fn m048_s01_meshc_test_project_dir_target_honors_override_entrypoint_contract() {
    let artifacts = artifact_dir("meshc-test-project-dir");
    let project = override_test_fixture(&artifacts);

    let output = run_meshc_test(&project.project_dir, &artifacts, "meshc-test");
    assert_success(
        &output,
        "meshc test <project-dir> should succeed for an override-entry project",
        &artifacts,
    );
    assert_stdout_contains(
        &output,
        "proof=override-test answer=42 label=override-tests-support",
        "meshc test <project-dir> should execute the fixture test instead of a zero-proof run",
        &artifacts,
    );
    assert_stdout_contains(
        &output,
        "1 passed",
        "meshc test <project-dir> should report one passing test file",
        &artifacts,
    );
}

#[test]
fn m048_s01_meshc_test_tests_dir_target_honors_override_entrypoint_contract() {
    let artifacts = artifact_dir("meshc-test-tests-dir");
    let project = override_test_fixture(&artifacts);

    let output = run_meshc_test(&project.tests_dir, &artifacts, "meshc-test");
    assert_success(
        &output,
        "meshc test <tests-dir> should succeed for an override-entry project",
        &artifacts,
    );
    assert_stdout_contains(
        &output,
        "proof=override-test answer=42 label=override-tests-support",
        "meshc test <tests-dir> should execute the fixture test instead of drifting to repo sources",
        &artifacts,
    );
    assert_stdout_contains(
        &output,
        "1 passed",
        "meshc test <tests-dir> should report one passing test file",
        &artifacts,
    );
}

#[test]
fn m048_s01_meshc_test_specific_file_target_honors_override_entrypoint_contract() {
    let artifacts = artifact_dir("meshc-test-specific-file");
    let project = override_test_fixture(&artifacts);
    let test_file = project
        .test_file
        .as_ref()
        .expect("override test fixture should expose a specific test file")
        .clone();

    let output = run_meshc_test(&test_file, &artifacts, "meshc-test");
    assert_success(
        &output,
        "meshc test <specific-file> should succeed for an override-entry project",
        &artifacts,
    );
    assert_stdout_contains(
        &output,
        "proof=override-test answer=42 label=override-tests-support",
        "meshc test <specific-file> should execute the fixture test instead of falling back to repo CWD",
        &artifacts,
    );
    assert_stdout_contains(
        &output,
        "1 passed",
        "meshc test <specific-file> should report one passing test file",
        &artifacts,
    );
}
