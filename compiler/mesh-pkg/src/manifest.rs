use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Represents a parsed mesh.toml manifest file.
#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
}

/// Package metadata from the [package] section of mesh.toml.
#[derive(Debug, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
}

/// A dependency specification -- registry, git-based, or path-based.
///
/// Serde uses `untagged` deserialization, so variants are tried in declaration
/// order. RegistryShorthand MUST be first so a bare string "1.0.0" matches it
/// before Git or Path are attempted.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Dependency {
    /// Bare string shorthand: `foo = "1.0.0"`
    RegistryShorthand(String),
    /// Table form: `foo = { version = "1.0.0" }`
    Registry {
        version: String,
    },
    /// Git source: `foo = { git = "https://...", ... }`
    Git {
        git: String,
        #[serde(default)]
        rev: Option<String>,
        #[serde(default)]
        branch: Option<String>,
        #[serde(default)]
        tag: Option<String>,
    },
    /// Local path: `foo = { path = "../foo" }`
    Path {
        path: String,
    },
}

impl Dependency {
    /// Returns the version string if this is a registry dependency.
    pub fn registry_version(&self) -> Option<&str> {
        match self {
            Dependency::RegistryShorthand(v) => Some(v),
            Dependency::Registry { version } => Some(version),
            _ => None,
        }
    }

    /// Returns true if this is a registry (not git or path) dependency.
    pub fn is_registry(&self) -> bool {
        self.registry_version().is_some()
    }
}

impl Manifest {
    /// Read and parse a mesh.toml manifest from a file path.
    pub fn from_file(path: &Path) -> Result<Manifest, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        Self::from_str(&content)
    }

    /// Parse a mesh.toml manifest from a string.
    pub fn from_str(content: &str) -> Result<Manifest, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse manifest: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_manifest() {
        let toml = r#"
[package]
name = "my-project"
version = "0.1.0"
description = "A test project"
authors = ["Alice", "Bob"]

[dependencies]
json-lib = { git = "https://github.com/example/json-lib.git", tag = "v1.0" }
math-utils = { git = "https://github.com/example/math-utils.git", branch = "main" }
local-dep = { path = "../local-dep" }
"#;
        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "my-project");
        assert_eq!(manifest.package.version, "0.1.0");
        assert_eq!(manifest.package.description.as_deref(), Some("A test project"));
        assert_eq!(manifest.package.authors, vec!["Alice", "Bob"]);
        assert_eq!(manifest.dependencies.len(), 3);

        // BTreeMap is sorted by key
        let keys: Vec<&String> = manifest.dependencies.keys().collect();
        assert_eq!(keys, vec!["json-lib", "local-dep", "math-utils"]);

        match &manifest.dependencies["json-lib"] {
            Dependency::Git { git, tag, .. } => {
                assert_eq!(git, "https://github.com/example/json-lib.git");
                assert_eq!(tag.as_deref(), Some("v1.0"));
            }
            _ => panic!("Expected git dependency"),
        }

        match &manifest.dependencies["local-dep"] {
            Dependency::Path { path } => {
                assert_eq!(path, "../local-dep");
            }
            _ => panic!("Expected path dependency"),
        }

        match &manifest.dependencies["math-utils"] {
            Dependency::Git { git, branch, .. } => {
                assert_eq!(git, "https://github.com/example/math-utils.git");
                assert_eq!(branch.as_deref(), Some("main"));
            }
            _ => panic!("Expected git dependency"),
        }
    }

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
[package]
name = "minimal"
version = "0.0.1"
"#;
        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.package.name, "minimal");
        assert_eq!(manifest.package.version, "0.0.1");
        assert!(manifest.package.description.is_none());
        assert!(manifest.package.authors.is_empty());
        assert!(manifest.dependencies.is_empty());
    }

    #[test]
    fn reject_missing_package_section() {
        let toml = r#"
[dependencies]
foo = { path = "./foo" }
"#;
        let result = Manifest::from_str(toml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to parse manifest"), "Error: {}", err);
    }

    #[test]
    fn reject_missing_name() {
        let toml = r#"
[package]
version = "1.0.0"
"#;
        let result = Manifest::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn reject_missing_version() {
        let toml = r#"
[package]
name = "no-version"
"#;
        let result = Manifest::from_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn parse_git_dep_with_rev() {
        let toml = r#"
[package]
name = "rev-test"
version = "1.0.0"

[dependencies]
pinned = { git = "https://example.com/pinned.git", rev = "abc123" }
"#;
        let manifest = Manifest::from_str(toml).unwrap();
        match &manifest.dependencies["pinned"] {
            Dependency::Git { git, rev, .. } => {
                assert_eq!(git, "https://example.com/pinned.git");
                assert_eq!(rev.as_deref(), Some("abc123"));
            }
            _ => panic!("Expected git dependency"),
        }
    }

    #[test]
    fn parse_git_dep_bare() {
        let toml = r#"
[package]
name = "bare-git"
version = "1.0.0"

[dependencies]
lib = { git = "https://example.com/lib.git" }
"#;
        let manifest = Manifest::from_str(toml).unwrap();
        match &manifest.dependencies["lib"] {
            Dependency::Git { git, rev, branch, tag } => {
                assert_eq!(git, "https://example.com/lib.git");
                assert!(rev.is_none());
                assert!(branch.is_none());
                assert!(tag.is_none());
            }
            _ => panic!("Expected git dependency"),
        }
    }

    // --- New tests for registry dependencies and license field ---

    #[test]
    fn parse_registry_shorthand() {
        let toml = r#"
[package]
name = "uses-registry"
version = "0.1.0"

[dependencies]
foo = "1.0.0"
"#;
        let manifest = Manifest::from_str(toml).unwrap();
        match &manifest.dependencies["foo"] {
            Dependency::RegistryShorthand(v) => {
                assert_eq!(v, "1.0.0");
            }
            other => panic!("Expected RegistryShorthand, got: {:?}", other),
        }
        assert!(manifest.dependencies["foo"].is_registry());
        assert_eq!(manifest.dependencies["foo"].registry_version(), Some("1.0.0"));
    }

    #[test]
    fn parse_registry_table_form() {
        let toml = r#"
[package]
name = "uses-registry-table"
version = "0.1.0"

[dependencies]
foo = { version = "1.0.0" }
"#;
        let manifest = Manifest::from_str(toml).unwrap();
        match &manifest.dependencies["foo"] {
            Dependency::Registry { version } => {
                assert_eq!(version, "1.0.0");
            }
            other => panic!("Expected Registry, got: {:?}", other),
        }
        assert!(manifest.dependencies["foo"].is_registry());
        assert_eq!(manifest.dependencies["foo"].registry_version(), Some("1.0.0"));
    }

    #[test]
    fn parse_mixed_dependency_types() {
        let toml = r#"
[package]
name = "mixed-deps"
version = "0.1.0"

[dependencies]
registry-short = "2.3.4"
registry-table = { version = "1.0.0" }
git-dep = { git = "https://github.com/example/lib.git", tag = "v1.0" }
path-dep = { path = "../path-dep" }
"#;
        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.dependencies.len(), 4);

        match &manifest.dependencies["registry-short"] {
            Dependency::RegistryShorthand(v) => assert_eq!(v, "2.3.4"),
            other => panic!("Expected RegistryShorthand, got: {:?}", other),
        }

        match &manifest.dependencies["registry-table"] {
            Dependency::Registry { version } => assert_eq!(version, "1.0.0"),
            other => panic!("Expected Registry, got: {:?}", other),
        }

        match &manifest.dependencies["git-dep"] {
            Dependency::Git { git, tag, .. } => {
                assert_eq!(git, "https://github.com/example/lib.git");
                assert_eq!(tag.as_deref(), Some("v1.0"));
            }
            other => panic!("Expected Git, got: {:?}", other),
        }

        match &manifest.dependencies["path-dep"] {
            Dependency::Path { path } => assert_eq!(path, "../path-dep"),
            other => panic!("Expected Path, got: {:?}", other),
        }
    }

    #[test]
    fn parse_license_field() {
        // With license field
        let toml_with_license = r#"
[package]
name = "licensed"
version = "1.0.0"
license = "MIT"
"#;
        let manifest = Manifest::from_str(toml_with_license).unwrap();
        assert_eq!(manifest.package.license.as_deref(), Some("MIT"));

        // Without license field -- should still parse and default to None
        let toml_no_license = r#"
[package]
name = "unlicensed"
version = "1.0.0"
"#;
        let manifest = Manifest::from_str(toml_no_license).unwrap();
        assert!(manifest.package.license.is_none());
    }
}
