use std::path::Path;
use std::time::Duration;

use colored::Colorize;
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use tar::Builder;

use mesh_pkg::Manifest;

pub fn run(project_dir: &Path, registry: &str, json_mode: bool) -> Result<(), String> {
    // Read manifest
    let manifest_path = project_dir.join("mesh.toml");
    let manifest = Manifest::from_file(&manifest_path)?;

    let name = &manifest.package.name;
    let version = &manifest.package.version;

    // Create tarball in memory
    let (tarball_bytes, sha256) = create_tarball(project_dir, &manifest)?;

    // Upload with spinner
    let msg = format!("Publishing {}@{}...", name, version);
    with_spinner(&msg, json_mode, || {
        upload_tarball(&tarball_bytes, &sha256, name, version, registry)
    })?;

    if json_mode {
        println!("{{\"status\": \"ok\", \"name\": \"{}\", \"version\": \"{}\", \"sha256\": \"{}\"}}",
            name, version, sha256);
    } else {
        println!("{} Published {}@{}", "✓".green().bold(), name, version);
        println!("  SHA-256: {}", sha256);
    }

    Ok(())
}

fn create_tarball(project_dir: &Path, _manifest: &Manifest) -> Result<(Vec<u8>, String), String> {
    let mut buf = Vec::new();
    {
        let enc = GzEncoder::new(&mut buf, Compression::default());
        let mut archive = Builder::new(enc);

        // Add mesh.toml at archive root (no prefix directory)
        let mesh_toml = project_dir.join("mesh.toml");
        archive.append_path_with_name(&mesh_toml, "mesh.toml")
            .map_err(|e| format!("Failed to add mesh.toml to tarball: {}", e))?;

        // Add root-level .mpl files (package source — e.g. slug.mpl, main.mpl)
        // Exclude *.test.mpl files (test-only, not needed by consumers)
        for entry in std::fs::read_dir(project_dir)
            .map_err(|e| format!("Failed to read project dir: {}", e))?
        {
            let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "mpl" {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if !name.ends_with(".test.mpl") {
                            archive.append_path_with_name(&path, name)
                                .map_err(|e| format!("Failed to add {} to tarball: {}", name, e))?;
                        }
                    }
                }
            }
        }

        // Add src/ directory at archive root
        let src_dir = project_dir.join("src");
        if src_dir.exists() {
            archive.append_dir_all("src", &src_dir)
                .map_err(|e| format!("Failed to add src/ to tarball: {}", e))?;
        }

        archive.into_inner()
            .map_err(|e| format!("Failed to finalize tarball: {}", e))?
            .finish()
            .map_err(|e| format!("Failed to flush gzip stream: {}", e))?;
    }

    // Compute SHA-256
    let hash_bytes = Sha256::digest(&buf);
    let sha256: String = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect();

    Ok((buf, sha256))
}

fn upload_tarball(
    tarball: &[u8],
    sha256: &str,
    name: &str,
    version: &str,
    registry: &str,
) -> Result<(), String> {
    // Read auth token
    let token = crate::auth::read_token()?;

    let agent = ureq::Agent::new_with_defaults();
    let url = format!("{}/api/v1/packages", registry);

    let response = agent
        .post(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Type", "application/octet-stream")
        .header("X-Package-Name", name)
        .header("X-Package-Version", version)
        .header("X-Package-SHA256", sha256)
        .send(tarball)
        .map_err(|e| format!("Failed to connect to registry: {}", e))?;

    match response.status().as_u16() {
        200 | 201 => Ok(()),
        409 => Err(format!("{}@{} already exists in registry. Versions are immutable.", name, version)),
        401 => Err("Unauthorized. Run `meshpkg login` to authenticate.".to_string()),
        status => Err(format!("Registry returned HTTP {}", status)),
    }
}

pub(crate) fn with_spinner<T, F: FnOnce() -> Result<T, String>>(
    msg: &str,
    json_mode: bool,
    f: F,
) -> Result<T, String> {
    if json_mode {
        return f();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", ""]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    let result = f();
    pb.finish_and_clear();
    result
}
