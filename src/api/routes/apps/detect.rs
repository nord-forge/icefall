use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::TeamCtx;
use crate::api::AppState;
use crate::build::git::{clone_repo, GitCloneOptions};
use crate::github::token::get_valid_installation_token;

#[derive(Deserialize)]
pub(super) struct DetectRequest {
    git_repo: String,
    git_branch: Option<String>,
    github_installation_id: Option<String>,
    base_directory: Option<String>,
}

/// POST /apps/detect — clone a repo shallowly and report how it would deploy:
/// framework + suggested build settings, plus repo-shape hints (Dockerfile
/// variants, compose files, monorepo) and foreign-platform coupling (AC5).
pub(super) async fn detect_repo(
    State(state): State<AppState>,
    _ctx: TeamCtx,
    Json(body): Json<DetectRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let repo = body.git_repo.trim();
    if repo.is_empty() {
        return Err(ApiError::BadRequest("git_repo is required".into()));
    }

    // Private repos: resolve an installation token so the clone authenticates.
    let token = match &body.github_installation_id {
        Some(id) => Some(
            get_valid_installation_token(&state.db, id)
                .await
                .map_err(ApiError::BadRequest)?
                .token,
        ),
        None => None,
    };

    let tmp = tempfile::tempdir()
        .map_err(|e| ApiError::Internal(Box::new(std::io::Error::other(e.to_string()))))?;
    let work_dir = tmp.path().join("repo");

    let opts = GitCloneOptions {
        repo_url: repo.to_string(),
        branch: body.git_branch.clone(),
        sha: None,
        ssh_key_path: None,
        token,
        submodules: false,
        lfs: false,
        shallow: true,
    };

    // Clone failure is the reachability signal — surface it as a 400 the wizard
    // turns into "repo not found / not accessible".
    if let Err(e) = clone_repo(&opts, &work_dir).await {
        return Err(ApiError::BadRequest(format!(
            "Repository not accessible: {e}"
        )));
    }

    let project_dir = match body.base_directory.as_ref().map(|d| d.trim()) {
        Some(d) if !d.is_empty() => work_dir.join(d),
        _ => work_dir.clone(),
    };

    let detection = crate::build::detect::detect(&project_dir, None)
        .map_err(|e| ApiError::BadRequest(format!("Detection failed: {e}")))?;
    let hints = crate::build::detect::detect_repo_hints(&project_dir, &detection);

    let coupling = first_compose_coupling(&project_dir, &hints.compose_files);

    Ok(Json(serde_json::json!({
        "data": {
            "detection": detection,
            "hints": hints,
            "foreign_coupling": coupling,
        }
    })))
}

/// Audit the first compose file (if any) for foreign-platform coupling. The
/// wizard re-audits the specific file the user picks; this is the at-a-glance
/// signal for the resolution step.
fn first_compose_coupling(
    project_dir: &std::path::Path,
    compose_files: &[String],
) -> Option<serde_json::Value> {
    let name = compose_files.first()?;
    let yaml = std::fs::read_to_string(project_dir.join(name)).ok()?;
    let coupling = crate::build::compose_audit::analyze_foreign_coupling(&yaml).ok()?;
    if coupling.is_empty() {
        return None;
    }
    Some(serde_json::json!({ "file": name, "coupling": coupling }))
}
