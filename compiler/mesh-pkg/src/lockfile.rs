use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents the contents of a mesh.lock file.
///
/// The lockfile captures the exact resolved state of all dependencies,
/// ensuring deterministic builds. Packages are always sorted by name.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Lockfile {
    /// Lockfile format version. Always 1 for now.
    pub version: u32,
    /// Resolved packages, sorted by name for deterministic output.
    pub packages: Vec<LockedPackage>,
}

/// A single resolved package entry in the lockfile.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct LockedPackage {
    /// Package name.
    pub name: String,
    /// Explicit version string. Empty string for git/path deps.
    #[serde(default)]
    pub version: String,
    /// Source location (registry download URL, git URL, or local path).
    pub source: String,
    /// Resolved revision (git commit SHA, "local" for path deps, or version string for registry).
    pub revision: String,
    /// Hex SHA-256 of the downloaded tarball. None for git/path deps.
    #[serde(default)]
    pub sha256: Option<String>,
}

impl Lockfile {
    /// Create a new lockfile with the given packages.
    /// Packages are sorted by name for deterministic output.
    pub fn new(mut packages: Vec<LockedPackage>) -> Self {
        packages.sort_by(|a, b| a.name.cmp(&b.name));
        Lockfile {
            version: 1,
            packages,
        }
    }

    /// Serialize and write the lockfile to the given path.
    pub fn write(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize lockfile: {}", e))?;
        std::fs::write(path, content)
            .map_err(|e| format!("Failed to write {}: {}", path.display(), e))
    }

    /// Read and deserialize a lockfile from the given path.
    pub fn read(path: &Path) -> Result<Lockfile, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse lockfile: {}", e))
    }

    /// Serialize the lockfile to a TOML string.
    pub fn to_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize lockfile: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn lockfile_round_trip() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("mesh.lock");

        let lockfile = Lockfile::new(vec![
            LockedPackage {
                name: "beta-lib".to_string(),
                version: String::new(),
                source: "https://github.com/example/beta.git".to_string(),
                revision: "def456".to_string(),
                sha256: None,
            },
            LockedPackage {
                name: "alpha-lib".to_string(),
                version: String::new(),
                source: "https://github.com/example/alpha.git".to_string(),
                revision: "abc123".to_string(),
                sha256: None,
            },
        ]);

        // Packages should be sorted by name
        assert_eq!(lockfile.packages[0].name, "alpha-lib");
        assert_eq!(lockfile.packages[1].name, "beta-lib");

        // Write and read back
        lockfile.write(&lock_path).unwrap();
        let read_back = Lockfile::read(&lock_path).unwrap();
        assert_eq!(lockfile, read_back);
    }

    #[test]
    fn lockfile_determinism() {
        // Same inputs must produce byte-identical output
        let packages1 = vec![
            LockedPackage {
                name: "zlib".to_string(),
                version: String::new(),
                source: "/path/to/zlib".to_string(),
                revision: "local".to_string(),
                sha256: None,
            },
            LockedPackage {
                name: "alib".to_string(),
                version: String::new(),
                source: "https://example.com/alib.git".to_string(),
                revision: "aaa111".to_string(),
                sha256: None,
            },
        ];

        let packages2 = vec![
            LockedPackage {
                name: "alib".to_string(),
                version: String::new(),
                source: "https://example.com/alib.git".to_string(),
                revision: "aaa111".to_string(),
                sha256: None,
            },
            LockedPackage {
                name: "zlib".to_string(),
                version: String::new(),
                source: "/path/to/zlib".to_string(),
                revision: "local".to_string(),
                sha256: None,
            },
        ];

        let lf1 = Lockfile::new(packages1);
        let lf2 = Lockfile::new(packages2);

        let s1 = lf1.to_string().unwrap();
        let s2 = lf2.to_string().unwrap();
        assert_eq!(s1, s2, "Same packages in different order must produce identical output");
    }

    #[test]
    fn lockfile_empty() {
        let dir = TempDir::new().unwrap();
        let lock_path = dir.path().join("mesh.lock");

        let lockfile = Lockfile::new(vec![]);
        assert_eq!(lockfile.version, 1);
        assert!(lockfile.packages.is_empty());

        lockfile.write(&lock_path).unwrap();
        let read_back = Lockfile::read(&lock_path).unwrap();
        assert_eq!(lockfile, read_back);
    }

    #[test]
    fn lockfile_with_path_dep() {
        let lockfile = Lockfile::new(vec![
            LockedPackage {
                name: "local-dep".to_string(),
                version: String::new(),
                source: "../local-dep".to_string(),
                revision: "local".to_string(),
                sha256: None,
            },
        ]);

        let s = lockfile.to_string().unwrap();
        assert!(s.contains("local-dep"));
        assert!(s.contains("local"));
    }

    #[test]
    fn lockfile_registry_package_with_sha256() {
        // Registry packages have version and sha256 populated
        let lockfile = Lockfile::new(vec![
            LockedPackage {
                name: "foo".to_string(),
                version: "1.0.0".to_string(),
                source: "https://registry.example.com/packages/foo-1.0.0.tar.gz".to_string(),
                revision: "1.0.0".to_string(),
                sha256: Some("abc123def456".to_string()),
            },
        ]);

        let s = lockfile.to_string().unwrap();
        assert!(s.contains("sha256"));
        assert!(s.contains("abc123def456"));
        assert!(s.contains("1.0.0"));

        // Round-trip
        let lf2: Lockfile = toml::from_str(&s).unwrap();
        assert_eq!(lf2.packages[0].sha256.as_deref(), Some("abc123def456"));
        assert_eq!(lf2.packages[0].version, "1.0.0");
    }

    #[test]
    fn lockfile_backward_compat_no_sha256() {
        // Old lockfiles without sha256 and version fields should still parse
        let old_format = r#"
version = 1

[[packages]]
name = "some-dep"
source = "https://github.com/example/some-dep.git"
revision = "abc123"
"#;
        let lf: Lockfile = toml::from_str(old_format).unwrap();
        assert_eq!(lf.packages.len(), 1);
        assert_eq!(lf.packages[0].name, "some-dep");
        assert!(lf.packages[0].sha256.is_none());
        assert_eq!(lf.packages[0].version, "");
    }
}
