use std::sync::Arc;
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use crate::{db, error::AppError, state::AppState};

#[derive(Serialize)]
pub struct VersionListItem {
    pub version: String,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub download_count: i64,
    pub size_bytes: i64,
}

/// GET /api/v1/packages/{name}/versions
/// Returns all versions for a package ordered newest first.
pub async fn versions_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<VersionListItem>>, AppError> {
    // Ensure package exists (returns 404 if not)
    db::packages::get_package(&state.pool, &name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;
    let versions = db::packages::list_versions(&state.pool, &name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(versions.into_iter().map(|v| VersionListItem {
        version: v.version,
        published_at: v.published_at,
        download_count: v.download_count,
        size_bytes: v.size_bytes,
    }).collect()))
}

#[derive(Serialize)]
pub struct VersionMeta {
    pub sha256: String,
}

#[derive(Serialize)]
pub struct LatestVersion {
    pub version: String,
    pub sha256: String,
}

/// GET /api/v1/packages/{name}/{version}
/// Returns {"sha256": "..."} — used by meshpkg install to verify
pub async fn version_handler(
    State(state): State<Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
) -> Result<Json<VersionMeta>, AppError> {
    let ver = db::packages::get_version(&state.pool, &name, &version)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;
    Ok(Json(VersionMeta { sha256: ver.sha256 }))
}

/// GET /api/v1/packages/{name}
/// Returns {latest: {version, sha256}, readme, description, owner, download_count}
/// meshpkg install <name> uses .latest.version and .latest.sha256
/// Website PackagePage.vue uses .readme for README rendering (REG-04)
pub async fn package_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let pkg = db::packages::get_package(&state.pool, &name)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;

    // Fetch the latest version record (for sha256 AND readme)
    let (latest, readme) = if let Some(ref latest_ver) = pkg.latest_version {
        let ver = db::packages::get_version(&state.pool, &name, latest_ver)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        match ver {
            Some(v) => {
                let latest_json = serde_json::json!({
                    "version": v.version,
                    "sha256": v.sha256,
                });
                let readme = v.readme;
                (Some(latest_json), readme)
            }
            None => (None, None),
        }
    } else {
        (None, None)
    };

    Ok(Json(serde_json::json!({
        "name": pkg.name,
        "description": pkg.description,
        "owner": pkg.owner_login,
        "download_count": pkg.download_count,
        "latest": latest,
        "readme": readme,   // Option<String>: null if no README was in tarball
    })))
}
