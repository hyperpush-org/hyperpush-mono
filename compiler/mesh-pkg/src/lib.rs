pub mod lockfile;
pub mod manifest;
pub mod resolver;
pub mod scaffold;

// Re-export key types for convenience.
pub use lockfile::{LockedPackage, Lockfile};
pub use manifest::{Dependency, Manifest, Package};
pub use resolver::resolve_dependencies;
pub use scaffold::scaffold_project;
