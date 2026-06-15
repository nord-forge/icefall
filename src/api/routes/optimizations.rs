//! Smart Resource Packer API (IF-191): surface right-sizing recommendations for
//! a server's apps and apply them.

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::{App, UpdateApp, CONTROL_PLANE_SERVER_ID};
use crate::optimize::{self, AppUsage, Recommendation, RecommendationKind, ServerCapacity};

/// Default analysis window. The packer right-sizes from the last week of usage.
const ANALYSIS_DAYS: i64 = 7;

/// Does this app run on the given server? Apps with no `server_id` are treated
/// as running on the control plane.
fn app_on_server(app: &App, server_id: &str) -> bool {
    match app.server_id.as_deref() {
        Some(sid) => sid == server_id,
        None => server_id == CONTROL_PLANE_SERVER_ID,
    }
}

/// Parse an app's stored `resource_limits` JSON for its memory limit (bytes).
fn app_memory_limit(app: &App) -> i64 {
    app.resource_limits
        .as_deref()
        .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
        .and_then(|v| v.get("memory_bytes").and_then(|m| m.as_i64()))
        .unwrap_or(0)
}

/// Build the engine inputs for one server's apps from persisted usage stats.
async fn build_app_usage(state: &AppState, server_id: &str) -> Result<Vec<AppUsage>, ApiError> {
    let apps = state.db.list_apps().await?;
    let usage = state.db.container_usage_stats(ANALYSIS_DAYS).await?;
    let usage_by_app: std::collections::HashMap<&str, _> =
        usage.iter().map(|u| (u.app_id.as_str(), u)).collect();

    Ok(apps
        .iter()
        .filter(|a| app_on_server(a, server_id))
        .filter_map(|a| {
            let u = usage_by_app.get(a.id.as_str())?;
            Some(AppUsage {
                app_id: a.id.clone(),
                app_name: a.name.clone(),
                current_memory_bytes: app_memory_limit(a),
                ghost_mode_enabled: a.ghost_mode_enabled,
                avg_cpu_percent: u.avg_cpu_percent,
                peak_memory_bytes: u.peak_memory_bytes,
                sample_count: u.sample_count,
            })
        })
        .collect())
}

/// Latest memory-used ratio per server, for co-location. Best-effort: servers
/// with no recent metric are reported at 0 utilization.
async fn server_capacities(state: &AppState) -> Result<Vec<ServerCapacity>, ApiError> {
    let servers = state.db.list_servers().await?;
    let apps = state.db.list_apps().await?;
    let now = crate::db::models::now_iso8601();
    let week_ago = (chrono::Utc::now() - chrono::Duration::days(7)).to_rfc3339();

    let mut caps = Vec::new();
    for s in &servers {
        let latest = state
            .db
            .query_server_metrics_history(&s.id, &week_ago, &now, 1)
            .await
            .unwrap_or_default();
        let ratio = latest
            .first()
            .and_then(|m| {
                let total = m.ram_total_bytes?;
                let used = m.ram_used_bytes?;
                (total > 0).then(|| used as f64 / total as f64)
            })
            .unwrap_or(0.0);
        let app_count = apps.iter().filter(|a| app_on_server(a, &s.id)).count();
        caps.push(ServerCapacity {
            server_id: s.id.clone(),
            server_name: s.name.clone(),
            memory_used_ratio: ratio,
            app_count,
        });
    }
    Ok(caps)
}

fn summarize(recs: &[Recommendation]) -> serde_json::Value {
    let ram_saved: i64 = recs.iter().map(|r| r.ram_saved_bytes).sum();
    let usd: f64 = recs.iter().map(|r| r.estimated_monthly_savings_usd).sum();
    serde_json::json!({
        "count": recs.len(),
        "ram_saved_bytes": ram_saved,
        "estimated_monthly_savings_usd": (usd * 100.0).round() / 100.0,
    })
}

pub async fn get_optimizations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let app_usage = build_app_usage(&state, &server_id).await?;
    let mut recs = optimize::analyze(&app_usage);
    recs.extend(optimize::colocation_recommendations(
        &server_capacities(&state).await?,
    ));

    Ok(Json(serde_json::json!({
        "data": {
            "recommendations": recs,
            "summary": summarize(&recs),
            "analysis_days": ANALYSIS_DAYS,
        }
    })))
}

#[derive(Deserialize)]
pub struct ApplyRequest {
    /// App to apply to. Required for single-app recommendations.
    pub app_id: String,
    /// The recommendation kind being applied — determines what changes.
    pub kind: RecommendationKind,
    /// New memory limit in bytes (for over/under-provisioned).
    pub memory_bytes: Option<i64>,
}

/// Apply a single recommendation to one app. Admins only — it mutates resource
/// limits / ghost mode. Returns the updated app.
pub async fn apply_optimization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    Json(body): Json<ApplyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }

    let app = state
        .db
        .get_app(&body.app_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("app {}", body.app_id)))?;
    if !app_on_server(&app, &server_id) {
        return Err(ApiError::BadRequest(
            "App does not run on this server".into(),
        ));
    }

    apply_one(&state, &app, body.kind, body.memory_bytes).await?;
    Ok(Json(serde_json::json!({ "message": "applied" })))
}

/// Apply all auto-applicable recommendations for a server at once. Skips the
/// advisory ones (under-provisioned, co-location). Admins only.
pub async fn apply_all_optimizations(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let caller = authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;
    if caller.role != "admin" {
        return Err(ApiError::Forbidden("Admin access required".into()));
    }

    let app_usage = build_app_usage(&state, &server_id).await?;
    let recs = optimize::analyze(&app_usage);
    let apps = state.db.list_apps().await?;
    let app_by_id: std::collections::HashMap<&str, &App> =
        apps.iter().map(|a| (a.id.as_str(), a)).collect();

    let mut applied = 0u32;
    for rec in recs.iter().filter(|r| r.auto_applicable) {
        if let Some(app) = app_by_id.get(rec.app_id.as_str()) {
            if apply_one(&state, app, rec.kind, rec.suggested_memory_bytes)
                .await
                .is_ok()
            {
                applied += 1;
            }
        }
    }

    Ok(Json(serde_json::json!({
        "message": format!("Applied {applied} recommendation(s)"),
        "applied": applied,
    })))
}

/// Apply one recommendation: set a new memory limit, or enable Ghost Mode.
async fn apply_one(
    state: &AppState,
    app: &App,
    kind: RecommendationKind,
    memory_bytes: Option<i64>,
) -> Result<(), ApiError> {
    let update = match kind {
        RecommendationKind::OverProvisioned | RecommendationKind::UnderProvisioned => {
            let mem =
                memory_bytes.ok_or_else(|| ApiError::BadRequest("memory_bytes required".into()))?;
            // Preserve any existing cpu_shares; only change memory_bytes.
            let mut limits: serde_json::Value = app
                .resource_limits
                .as_deref()
                .and_then(|j| serde_json::from_str(j).ok())
                .unwrap_or_else(|| serde_json::json!({}));
            limits["memory_bytes"] = serde_json::json!(mem);
            UpdateApp {
                resource_limits: Some(limits.to_string()),
                ..Default::default()
            }
        }
        RecommendationKind::Idle => UpdateApp {
            ghost_mode_enabled: Some(true),
            ..Default::default()
        },
        // Co-location is advisory — there's nothing safe to auto-apply.
        RecommendationKind::Colocation => {
            return Err(ApiError::BadRequest(
                "Co-location recommendations must be applied manually".into(),
            ))
        }
    };

    state.db.update_app(&app.id, &update).await?;
    Ok(())
}
