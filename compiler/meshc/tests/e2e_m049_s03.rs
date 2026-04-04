mod support;

use serde_json::json;
use std::fs;

use support::m049_todo_examples as todo;

#[test]
fn m049_s03_committed_examples_match_public_cli_output_and_preserve_mode_specific_file_sets() {
    let artifacts = todo::artifact_dir("todo-examples-parity");
    todo::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "examples_root": todo::examples_root(),
            "checks": [
                "public materializer check reproduces the committed todo-sqlite and todo-postgres trees exactly",
                "retained artifacts include generated trees, manifests, and target snapshots under .tmp/m049-s03",
                "SQLite/Postgres divergent file sets stay intentional rather than accidental drift"
            ]
        }),
    );

    let summary = todo::materialize_examples_check(&todo::examples_root(), &artifacts);
    todo::assert_expected_example_set(&summary);

    let sqlite = todo::example_entry(&summary, todo::SQLITE_EXAMPLE_NAME);
    assert_eq!(sqlite.example.db, "sqlite");
    assert!(
        sqlite.generated_manifest.file_count >= 15,
        "expected SQLite generated manifest to retain the full scaffold tree, got {:?}",
        sqlite.generated_manifest
    );
    let postgres = todo::example_entry(&summary, todo::POSTGRES_EXAMPLE_NAME);
    assert_eq!(postgres.example.db, "postgres");
    assert!(
        postgres.generated_manifest.file_count >= 16,
        "expected Postgres generated manifest to retain the full scaffold tree, got {:?}",
        postgres.generated_manifest
    );

    todo::assert_sqlite_example_shape(&todo::sqlite_example_dir());
    todo::assert_postgres_example_shape(&todo::postgres_example_dir());
}

#[test]
fn m049_s03_materializer_check_fails_closed_when_a_committed_example_root_is_missing() {
    let artifacts = todo::artifact_dir("todo-examples-missing-root");
    let temp_examples_root = artifacts.join("mutated/examples");
    todo::clone_examples_root(&temp_examples_root);
    fs::remove_dir_all(temp_examples_root.join(todo::SQLITE_EXAMPLE_NAME)).unwrap_or_else(
        |error| {
            panic!(
                "failed to remove {}: {error}",
                temp_examples_root.join(todo::SQLITE_EXAMPLE_NAME).display()
            )
        },
    );

    todo::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "examples_root": temp_examples_root,
            "checks": [
                "materializer check fails before any parity claim when todo-sqlite is missing",
                "failure artifacts preserve the mutated examples root and retained temp session path"
            ]
        }),
    );

    let run = todo::materialize_examples_check_expect_failure(&temp_examples_root, &artifacts);
    assert!(
        run.stderr.contains("[m049-s03] validation failed"),
        "expected validation failure output, got:\n{}",
        run.combined
    );
    assert!(
        run.stderr.contains("target todo-sqlite is missing"),
        "expected missing-root error in materializer output, got:\n{}",
        run.combined
    );
}

#[test]
fn m049_s03_materializer_check_names_missing_extra_and_changed_files_in_mutated_examples() {
    let artifacts = todo::artifact_dir("todo-examples-drift-report");
    let temp_examples_root = artifacts.join("mutated/examples");
    todo::clone_examples_root(&temp_examples_root);

    let sqlite_mesh_toml = temp_examples_root
        .join(todo::SQLITE_EXAMPLE_NAME)
        .join("mesh.toml");
    let sqlite_mesh_toml_original = fs::read_to_string(&sqlite_mesh_toml)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", sqlite_mesh_toml.display()));
    fs::write(
        &sqlite_mesh_toml,
        sqlite_mesh_toml_original.replace(
            "name = \"todo-sqlite\"",
            "name = \"todo-sqlite-hand-edited\"",
        ),
    )
    .unwrap_or_else(|error| panic!("failed to write {}: {error}", sqlite_mesh_toml.display()));
    fs::remove_file(
        temp_examples_root
            .join(todo::SQLITE_EXAMPLE_NAME)
            .join(todo::SQLITE_STORAGE_TEST_RELATIVE_PATH),
    )
    .unwrap_or_else(|error| {
        panic!(
            "failed to remove SQLite storage test from {}: {error}",
            temp_examples_root.display()
        )
    });

    let postgres_root = temp_examples_root.join(todo::POSTGRES_EXAMPLE_NAME);
    fs::remove_file(postgres_root.join(todo::POSTGRES_MIGRATION_RELATIVE_PATH)).unwrap_or_else(
        |error| {
            panic!(
                "failed to remove Postgres migration from {}: {error}",
                postgres_root.display()
            )
        },
    );
    fs::write(postgres_root.join("HAND_EDITED.txt"), "drift\n").unwrap_or_else(|error| {
        panic!(
            "failed to write hand-edited drift marker under {}: {error}",
            postgres_root.display()
        )
    });

    todo::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "examples_root": temp_examples_root,
            "checks": [
                "materializer check reports changed mesh.toml when the project name drifts",
                "materializer check reports missing SQLite storage and Postgres migration files",
                "materializer check reports extra hand-edited files instead of normalizing them away"
            ]
        }),
    );

    let run = todo::materialize_examples_check_expect_failure(&temp_examples_root, &artifacts);
    assert!(
        run.stderr.contains("example=todo-sqlite"),
        "expected SQLite drift report, got:\n{}",
        run.combined
    );
    assert!(
        run.stderr.contains("missing=tests/storage.test.mpl"),
        "expected SQLite missing storage-test report, got:\n{}",
        run.combined
    );
    assert!(
        run.stderr.contains("changed=mesh.toml"),
        "expected SQLite mesh.toml drift report, got:\n{}",
        run.combined
    );
    assert!(
        run.stderr.contains("example=todo-postgres"),
        "expected Postgres drift report, got:\n{}",
        run.combined
    );
    assert!(
        run.stderr
            .contains("missing=migrations/20260402120000_create_todos.mpl"),
        "expected Postgres missing migration report, got:\n{}",
        run.combined
    );
    assert!(
        run.stderr.contains("extra=HAND_EDITED.txt"),
        "expected Postgres extra-file drift report, got:\n{}",
        run.combined
    );
}

#[test]
fn m049_s03_sqlite_example_meshc_test_and_build_output_stay_green_and_out_of_tree() {
    let artifacts = todo::artifact_dir("todo-sqlite-test-build");
    todo::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "project_dir": todo::sqlite_example_dir(),
            "checks": [
                "meshc test on examples/todo-sqlite keeps the generated config/storage tests green",
                "meshc build --output writes the binary under .tmp/m049-s03 instead of polluting the tracked example tree",
                "build metadata and binary paths stay retained for later slice debugging"
            ]
        }),
    );

    todo::verify_sqlite_example_meshc_test_and_build(&todo::sqlite_example_dir(), &artifacts);
}

#[test]
fn m049_s03_postgres_example_meshc_test_and_build_output_stay_green_and_out_of_tree() {
    let artifacts = todo::artifact_dir("todo-postgres-test-build");
    todo::write_json_artifact(
        &artifacts.join("scenario-meta.json"),
        &json!({
            "project_dir": todo::postgres_example_dir(),
            "checks": [
                "meshc test on examples/todo-postgres keeps the generated config tests green",
                "meshc test output preserves the runtime-owned startup record marker for Work.sync_todos",
                "meshc build --output writes the binary under .tmp/m049-s03 instead of polluting the tracked example tree"
            ]
        }),
    );

    todo::verify_postgres_example_meshc_test_and_build(&todo::postgres_example_dir(), &artifacts);
}
