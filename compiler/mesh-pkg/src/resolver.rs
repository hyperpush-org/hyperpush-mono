//! Dependency resolver with DFS traversal, conflict detection, and cycle detection.
//!
//! Resolves dependencies declared in a mesh.toml manifest, handling both
//! git-based and path-based dependencies. Transitive dependencies are resolved
//! recursively, with diamond conflicts and cycles reported as errors.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use crate::lockfile::{LockedPackage, Lockfile};
use crate::manifest::{Dependency, Manifest};

/// A resolved dependency with its source and local checkout path.
#[derive(Debug, Clone)]
pub struct ResolvedDep {
    /// Package name.
    pub name: String,
    /// Where this dependency comes from.
    pub source: DepSource,
    /// Resolved git commit SHA, or "local" for path deps.
    pub revision: String,
    /// Local filesystem path where the dependency is checked out.
    pub path: PathBuf,
}

/// Source of a dependency -- either a git repository or a local path.
#[derive(Debug, Clone, PartialEq)]
pub enum DepSource {
    Git { url: String, rev: String },
    Path { path: PathBuf },
}

impl DepSource {
    /// Return a canonical string key for conflict detection.
    /// Two deps with the same name but different source keys are conflicting.
    fn key(&self) -> String {
        match self {
            DepSource::Git { url, .. } => format!("git:{}", url),
            DepSource::Path { path } => format!("path:{}", path.display()),
        }
    }
}

/// Internal state for the DFS resolver.
struct ResolveCtx {
    /// Resolved dependencies indexed by name.
    resolved: BTreeMap<String, ResolvedDep>,
    /// Source keys indexed by dep name, for conflict detection.
    source_keys: BTreeMap<String, String>,
    /// Currently in the DFS stack -- for cycle detection.
    visiting: HashSet<String>,
}

/// Resolve all dependencies from a manifest.
///
/// Performs DFS traversal over the dependency graph, resolving git and path
/// dependencies. Detects diamond conflicts (same name, different sources)
/// and cycles (A -> B -> A).
///
/// Git dependencies are cloned/fetched into `project_dir/.mesh/deps/<name>/`.
/// Path dependencies are resolved relative to the manifest's directory.
pub fn resolve(manifest: &Manifest, project_dir: &Path) -> Result<Vec<ResolvedDep>, String> {
    let mut ctx = ResolveCtx {
        resolved: BTreeMap::new(),
        source_keys: BTreeMap::new(),
        visiting: HashSet::new(),
    };

    resolve_deps(&manifest.dependencies, project_dir, project_dir, &mut ctx)?;

    Ok(ctx.resolved.into_values().collect())
}

/// Recursively resolve a set of dependencies.
///
/// `base_dir` is the directory containing the current manifest (used for
/// resolving relative path dependencies). `project_dir` is the top-level
/// project directory (used for the .mesh/deps checkout location).
fn resolve_deps(
    deps: &BTreeMap<String, Dependency>,
    base_dir: &Path,
    project_dir: &Path,
    ctx: &mut ResolveCtx,
) -> Result<(), String> {
    for (name, dep) in deps {
        // Cycle detection
        if ctx.visiting.contains(name) {
            return Err(format!(
                "Dependency cycle detected: `{}` is already being resolved",
                name
            ));
        }

        // Determine source and resolve
        let (source, revision, dep_path) = match dep {
            Dependency::RegistryShorthand(version) | Dependency::Registry { version } => {
                return Err(format!(
                    "Registry dependency `{}@{}` must be installed via `meshpkg install` before building",
                    name, version
                ));
            }
            Dependency::Git {
                git: url,
                rev,
                branch,
                tag,
            } => {
                let dest = project_dir.join(".mesh").join("deps").join(name);
                let resolved_rev = fetch_git_dep(
                    url,
                    &dest,
                    rev.as_deref(),
                    branch.as_deref(),
                    tag.as_deref(),
                )?;
                let source = DepSource::Git {
                    url: url.clone(),
                    rev: resolved_rev.clone(),
                };
                (source, resolved_rev, dest)
            }
            Dependency::Path { path } => {
                let resolved_path = if Path::new(path).is_absolute() {
                    PathBuf::from(path)
                } else {
                    base_dir.join(path).canonicalize().map_err(|e| {
                        format!(
                            "Failed to resolve path dependency `{}` ({}): {}",
                            name, path, e
                        )
                    })?
                };
                let source = DepSource::Path {
                    path: resolved_path.clone(),
                };
                (source, "local".to_string(), resolved_path)
            }
        };

        // Conflict detection: same name, different source
        let source_key = source.key();
        if let Some(existing_key) = ctx.source_keys.get(name) {
            if *existing_key != source_key {
                return Err(format!(
                    "Dependency conflict: `{}` required from two different sources",
                    name
                ));
            }
            // Same source -- already resolved, skip
            continue;
        }

        // Record this dep
        ctx.source_keys.insert(name.clone(), source_key);
        ctx.resolved.insert(
            name.clone(),
            ResolvedDep {
                name: name.clone(),
                source,
                revision,
                path: dep_path.clone(),
            },
        );

        // Check for transitive dependencies
        let sub_manifest_path = dep_path.join("mesh.toml");
        if sub_manifest_path.exists() {
            ctx.visiting.insert(name.clone());
            let sub_manifest = Manifest::from_file(&sub_manifest_path)?;
            resolve_deps(&sub_manifest.dependencies, &dep_path, project_dir, ctx)?;
            ctx.visiting.remove(name);
        }
    }

    Ok(())
}

/// Clone or fetch a git repository and check out the specified revision.
///
/// If `dest` does not exist, clones from `url`. If it exists, opens and fetches.
/// Then checks out the specified revision (rev, tag, or branch). If none specified,
/// uses the default branch HEAD.
///
/// Returns the resolved commit SHA.
pub fn fetch_git_dep(
    url: &str,
    dest: &Path,
    rev: Option<&str>,
    branch: Option<&str>,
    tag: Option<&str>,
) -> Result<String, String> {
    let repo = if dest.exists() {
        // Open existing repo and fetch
        let repo = git2::Repository::open(dest)
            .map_err(|e| format!("Failed to open git repo at {}: {}", dest.display(), e))?;

        // Fetch from origin (scoped to drop Remote before moving repo)
        {
            let mut remote = repo
                .find_remote("origin")
                .map_err(|e| format!("Failed to find remote 'origin': {}", e))?;
            remote
                .fetch(&[] as &[&str], None, None)
                .map_err(|e| format!("Failed to fetch from {}: {}", url, e))?;
        }

        repo
    } else {
        // Clone fresh
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!("Failed to create directory {}: {}", parent.display(), e)
            })?;
        }
        git2::Repository::clone(url, dest)
            .map_err(|e| format!("Failed to clone {}: {}", url, e))?
    };

    // Resolve the target commit
    let oid = if let Some(rev) = rev {
        // Exact revision
        git2::Oid::from_str(rev)
            .map_err(|e| format!("Invalid revision '{}': {}", rev, e))?
    } else if let Some(tag_name) = tag {
        // Tag -> resolve to commit
        let ref_name = format!("refs/tags/{}", tag_name);
        let reference = repo
            .find_reference(&ref_name)
            .map_err(|e| format!("Failed to find tag '{}': {}", tag_name, e))?;
        let commit = reference
            .peel_to_commit()
            .map_err(|e| format!("Failed to resolve tag '{}' to commit: {}", tag_name, e))?;
        commit.id()
    } else if let Some(branch_name) = branch {
        // Branch -> resolve HEAD of branch
        let ref_name = format!("refs/remotes/origin/{}", branch_name);
        let reference = repo.find_reference(&ref_name).or_else(|_| {
            // Try local branch
            let local_ref = format!("refs/heads/{}", branch_name);
            repo.find_reference(&local_ref)
        }).map_err(|e| format!("Failed to find branch '{}': {}", branch_name, e))?;
        let commit = reference
            .peel_to_commit()
            .map_err(|e| format!("Failed to resolve branch '{}' to commit: {}", branch_name, e))?;
        commit.id()
    } else {
        // Default branch HEAD
        let head = repo
            .head()
            .map_err(|e| format!("Failed to get HEAD of {}: {}", url, e))?;
        let commit = head
            .peel_to_commit()
            .map_err(|e| format!("Failed to resolve HEAD to commit: {}", e))?;
        commit.id()
    };

    // Checkout the resolved commit (detached HEAD)
    let obj = repo
        .find_object(oid, None)
        .map_err(|e| format!("Failed to find object {}: {}", oid, e))?;
    repo.checkout_tree(&obj, Some(git2::build::CheckoutBuilder::new().force()))
        .map_err(|e| format!("Failed to checkout {}: {}", oid, e))?;
    repo.set_head_detached(oid)
        .map_err(|e| format!("Failed to detach HEAD at {}: {}", oid, e))?;

    Ok(oid.to_string())
}

/// High-level API: resolve dependencies from a project directory.
///
/// Reads mesh.toml from `project_dir`, resolves all dependencies, and produces
/// a lockfile. Returns the resolved dependencies and lockfile.
pub fn resolve_dependencies(
    project_dir: &Path,
) -> Result<(Vec<ResolvedDep>, Lockfile), String> {
    let manifest_path = project_dir.join("mesh.toml");
    let manifest = Manifest::from_file(&manifest_path)?;

    let resolved = resolve(&manifest, project_dir)?;

    let locked_packages: Vec<LockedPackage> = resolved
        .iter()
        .map(|dep| LockedPackage {
            name: dep.name.clone(),
            version: String::new(), // git/path deps have no version number
            source: match &dep.source {
                DepSource::Git { url, .. } => url.clone(),
                DepSource::Path { path } => path.display().to_string(),
            },
            revision: dep.revision.clone(),
            sha256: None, // git/path deps have no tarball hash
        })
        .collect();

    let lockfile = Lockfile::new(locked_packages);

    Ok((resolved, lockfile))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper: create a minimal mesh.toml in the given directory.
    fn write_manifest(dir: &Path, name: &str, deps: &str) {
        let content = format!(
            r#"[package]
name = "{}"
version = "0.1.0"

{}
"#,
            name, deps
        );
        std::fs::write(dir.join("mesh.toml"), content).unwrap();
    }

    #[test]
    fn resolve_path_dependency() {
        let root = TempDir::new().unwrap();
        let dep_dir = root.path().join("libs").join("math");
        std::fs::create_dir_all(&dep_dir).unwrap();

        // Create the dependency's manifest
        write_manifest(&dep_dir, "math", "");

        // Create the root manifest with a path dependency
        let deps = format!(
            "[dependencies]\nmath = {{ path = \"{}\" }}",
            dep_dir.display()
        );
        write_manifest(root.path(), "my-project", &deps);

        let manifest = Manifest::from_file(&root.path().join("mesh.toml")).unwrap();
        let resolved = resolve(&manifest, root.path()).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "math");
        assert_eq!(resolved[0].revision, "local");
        match &resolved[0].source {
            DepSource::Path { path } => {
                assert_eq!(path.canonicalize().unwrap(), dep_dir.canonicalize().unwrap());
            }
            _ => panic!("Expected path dependency"),
        }
    }

    #[test]
    fn resolve_transitive_path_deps() {
        let root = TempDir::new().unwrap();

        // Create dep-b (leaf)
        let dep_b = root.path().join("dep-b");
        std::fs::create_dir_all(&dep_b).unwrap();
        write_manifest(&dep_b, "dep-b", "");

        // Create dep-a that depends on dep-b
        let dep_a = root.path().join("dep-a");
        std::fs::create_dir_all(&dep_a).unwrap();
        let deps_a = format!(
            "[dependencies]\ndep-b = {{ path = \"{}\" }}",
            dep_b.display()
        );
        write_manifest(&dep_a, "dep-a", &deps_a);

        // Root depends on dep-a
        let deps_root = format!(
            "[dependencies]\ndep-a = {{ path = \"{}\" }}",
            dep_a.display()
        );
        write_manifest(root.path(), "root", &deps_root);

        let manifest = Manifest::from_file(&root.path().join("mesh.toml")).unwrap();
        let resolved = resolve(&manifest, root.path()).unwrap();

        let names: HashSet<&str> = resolved.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains("dep-a"), "Should resolve dep-a");
        assert!(names.contains("dep-b"), "Should resolve dep-b transitively");
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn detect_diamond_conflict() {
        let root = TempDir::new().unwrap();

        // Create shared-dep with two different paths
        let shared_v1 = root.path().join("shared-v1");
        std::fs::create_dir_all(&shared_v1).unwrap();
        write_manifest(&shared_v1, "shared", "");

        let shared_v2 = root.path().join("shared-v2");
        std::fs::create_dir_all(&shared_v2).unwrap();
        write_manifest(&shared_v2, "shared", "");

        // dep-a depends on shared (at v1 path)
        let dep_a = root.path().join("dep-a");
        std::fs::create_dir_all(&dep_a).unwrap();
        let deps_a = format!(
            "[dependencies]\nshared = {{ path = \"{}\" }}",
            shared_v1.display()
        );
        write_manifest(&dep_a, "dep-a", &deps_a);

        // dep-b depends on shared (at v2 path -- different source!)
        let dep_b = root.path().join("dep-b");
        std::fs::create_dir_all(&dep_b).unwrap();
        let deps_b = format!(
            "[dependencies]\nshared = {{ path = \"{}\" }}",
            shared_v2.display()
        );
        write_manifest(&dep_b, "dep-b", &deps_b);

        // Root depends on both dep-a and dep-b
        let deps_root = format!(
            "[dependencies]\ndep-a = {{ path = \"{}\" }}\ndep-b = {{ path = \"{}\" }}",
            dep_a.display(),
            dep_b.display()
        );
        write_manifest(root.path(), "root", &deps_root);

        let manifest = Manifest::from_file(&root.path().join("mesh.toml")).unwrap();
        let result = resolve(&manifest, root.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Dependency conflict") && err.contains("shared"),
            "Expected conflict error for 'shared', got: {}",
            err
        );
    }

    #[test]
    fn detect_cycle() {
        let root = TempDir::new().unwrap();

        // dep-a depends on dep-b
        let dep_a = root.path().join("dep-a");
        std::fs::create_dir_all(&dep_a).unwrap();

        let dep_b = root.path().join("dep-b");
        std::fs::create_dir_all(&dep_b).unwrap();

        // dep-a -> dep-b
        let deps_a = format!(
            "[dependencies]\ndep-b = {{ path = \"{}\" }}",
            dep_b.display()
        );
        write_manifest(&dep_a, "dep-a", &deps_a);

        // dep-b -> dep-a (cycle!)
        let deps_b = format!(
            "[dependencies]\ndep-a = {{ path = \"{}\" }}",
            dep_a.display()
        );
        write_manifest(&dep_b, "dep-b", &deps_b);

        // Root depends on dep-a
        let deps_root = format!(
            "[dependencies]\ndep-a = {{ path = \"{}\" }}",
            dep_a.display()
        );
        write_manifest(root.path(), "root", &deps_root);

        let manifest = Manifest::from_file(&root.path().join("mesh.toml")).unwrap();
        let result = resolve(&manifest, root.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("cycle") || err.contains("Cycle"),
            "Expected cycle error, got: {}",
            err
        );
    }

    #[test]
    fn resolve_git_dep_local_repo() {
        // Create a local git repo to use as a "git" dependency
        let root = TempDir::new().unwrap();
        let git_repo_dir = root.path().join("upstream-lib");
        std::fs::create_dir_all(&git_repo_dir).unwrap();

        // Initialize a git repo with a commit
        let repo = git2::Repository::init(&git_repo_dir).unwrap();
        let manifest_content = r#"[package]
name = "upstream-lib"
version = "1.0.0"
"#;
        std::fs::write(git_repo_dir.join("mesh.toml"), manifest_content).unwrap();

        // Stage and commit
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("mesh.toml")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let commit_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Create root project that depends on local git repo
        let project_dir = root.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let deps = format!(
            "[dependencies]\nupstream-lib = {{ git = \"{}\" }}",
            git_repo_dir.display()
        );
        write_manifest(&project_dir, "my-project", &deps);

        let manifest = Manifest::from_file(&project_dir.join("mesh.toml")).unwrap();
        let resolved = resolve(&manifest, &project_dir).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "upstream-lib");
        assert_eq!(resolved[0].revision, commit_oid.to_string());
        match &resolved[0].source {
            DepSource::Git { url, rev } => {
                assert_eq!(url, &git_repo_dir.display().to_string());
                assert_eq!(rev, &commit_oid.to_string());
            }
            _ => panic!("Expected git dependency"),
        }

        // Verify the dep was cloned into .mesh/deps/
        let checkout = project_dir.join(".mesh/deps/upstream-lib/mesh.toml");
        assert!(checkout.exists(), "Git dep should be cloned to .mesh/deps/");
    }

    #[test]
    fn resolve_git_dep_with_branch() {
        let root = TempDir::new().unwrap();
        let git_repo_dir = root.path().join("branch-lib");
        std::fs::create_dir_all(&git_repo_dir).unwrap();

        // Init repo with initial commit on main
        let repo = git2::Repository::init(&git_repo_dir).unwrap();
        std::fs::write(git_repo_dir.join("mesh.toml"), "[package]\nname = \"branch-lib\"\nversion = \"0.1.0\"\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("mesh.toml")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let initial_commit = repo.commit(Some("HEAD"), &sig, &sig, "Initial", &tree, &[]).unwrap();
        let initial = repo.find_commit(initial_commit).unwrap();

        // Create a "dev" branch with a new commit
        repo.branch("dev", &initial, false).unwrap();
        repo.set_head("refs/heads/dev").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();

        std::fs::write(git_repo_dir.join("extra.txt"), "dev content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("extra.txt")).unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let dev_commit_oid = repo.commit(Some("refs/heads/dev"), &sig, &sig, "Dev commit", &tree, &[&initial]).unwrap();

        // Go back to default branch
        repo.set_head("refs/heads/main").ok();

        // Project depends on the dev branch
        let project_dir = root.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();
        let deps = format!(
            "[dependencies]\nbranch-lib = {{ git = \"{}\", branch = \"dev\" }}",
            git_repo_dir.display()
        );
        write_manifest(&project_dir, "my-project", &deps);

        let manifest = Manifest::from_file(&project_dir.join("mesh.toml")).unwrap();
        let resolved = resolve(&manifest, &project_dir).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "branch-lib");
        assert_eq!(resolved[0].revision, dev_commit_oid.to_string());
    }

    #[test]
    fn resolve_dependencies_e2e() {
        let root = TempDir::new().unwrap();

        // Create a path dependency
        let lib_dir = root.path().join("lib");
        std::fs::create_dir_all(&lib_dir).unwrap();
        write_manifest(&lib_dir, "my-lib", "");

        // Root project
        let deps = format!(
            "[dependencies]\nmy-lib = {{ path = \"{}\" }}",
            lib_dir.display()
        );
        write_manifest(root.path(), "my-app", &deps);

        let (resolved, lockfile) = resolve_dependencies(root.path()).unwrap();

        assert_eq!(resolved.len(), 1);
        assert_eq!(lockfile.packages.len(), 1);
        assert_eq!(lockfile.packages[0].name, "my-lib");
        assert_eq!(lockfile.packages[0].revision, "local");
        assert_eq!(lockfile.version, 1);
    }

    #[test]
    fn no_deps_produces_empty_lockfile() {
        let root = TempDir::new().unwrap();
        write_manifest(root.path(), "no-deps", "");

        let (resolved, lockfile) = resolve_dependencies(root.path()).unwrap();
        assert!(resolved.is_empty());
        assert!(lockfile.packages.is_empty());
        assert_eq!(lockfile.version, 1);
    }

    #[test]
    fn diamond_same_source_ok() {
        // Diamond where both sides depend on same source -- should NOT conflict
        let root = TempDir::new().unwrap();

        let shared = root.path().join("shared");
        std::fs::create_dir_all(&shared).unwrap();
        write_manifest(&shared, "shared", "");

        let dep_a = root.path().join("dep-a");
        std::fs::create_dir_all(&dep_a).unwrap();
        let deps_a = format!(
            "[dependencies]\nshared = {{ path = \"{}\" }}",
            shared.display()
        );
        write_manifest(&dep_a, "dep-a", &deps_a);

        let dep_b = root.path().join("dep-b");
        std::fs::create_dir_all(&dep_b).unwrap();
        let deps_b = format!(
            "[dependencies]\nshared = {{ path = \"{}\" }}",
            shared.display()
        );
        write_manifest(&dep_b, "dep-b", &deps_b);

        let deps_root = format!(
            "[dependencies]\ndep-a = {{ path = \"{}\" }}\ndep-b = {{ path = \"{}\" }}",
            dep_a.display(),
            dep_b.display()
        );
        write_manifest(root.path(), "root", &deps_root);

        let manifest = Manifest::from_file(&root.path().join("mesh.toml")).unwrap();
        let resolved = resolve(&manifest, root.path()).unwrap();

        // shared should appear only once
        let names: Vec<&str> = resolved.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(
            names.iter().filter(|&&n| n == "shared").count(),
            1,
            "Diamond with same source should deduplicate"
        );
        assert_eq!(resolved.len(), 3); // dep-a, dep-b, shared
    }
}
