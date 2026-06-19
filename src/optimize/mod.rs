//! Smart Resource Packer (IF-191): a pure recommendations engine producing
//! right-sizing and placement suggestions from limits, usage, and servers.

use serde::{Deserialize, Serialize};

/// How a recommendation should be applied. The API uses this to decide what to
/// mutate, and the UI to label the action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationKind {
    /// Memory limit is far above peak usage — lower it to free RAM.
    OverProvisioned,
    /// Container is pressing against its memory limit — raise it (or alert).
    UnderProvisioned,
    /// Consistently near-idle CPU — enable Ghost Mode to reclaim it.
    Idle,
    /// Low-resource app that could move to a more utilized server.
    Colocation,
}

/// A single actionable suggestion for one app.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub app_id: String,
    pub app_name: String,
    pub kind: RecommendationKind,
    /// Human-readable explanation (current → suggested + why).
    pub message: String,
    /// Current memory limit in bytes (0 = unlimited / unset).
    pub current_memory_bytes: i64,
    /// Suggested memory limit in bytes, when the recommendation changes it.
    pub suggested_memory_bytes: Option<i64>,
    /// RAM freed if applied, in bytes (0 for non-memory recs).
    pub ram_saved_bytes: i64,
    /// Rough monthly cost saving in USD, derived from RAM freed.
    pub estimated_monthly_savings_usd: f64,
    /// Whether "Apply all" may apply this automatically. Under-provisioned and
    /// co-location recs are advisory and excluded.
    pub auto_applicable: bool,
}

/// The current limits + usage for one app, the per-app input to the engine.
#[derive(Debug, Clone)]
pub struct AppUsage {
    pub app_id: String,
    pub app_name: String,
    /// Current memory limit in bytes; 0 means unlimited/unset.
    pub current_memory_bytes: i64,
    pub ghost_mode_enabled: bool,
    pub avg_cpu_percent: f64,
    pub peak_memory_bytes: i64,
    pub sample_count: i64,
}

/// Tuning constants for the policy. Centralized so tests and callers share them.
pub mod policy {
    /// Memory limit considered over-provisioned at this multiple of peak usage.
    pub const OVERPROVISION_RATIO: f64 = 2.0;
    /// Headroom added above peak when right-sizing memory.
    pub const HEADROOM: f64 = 1.20;
    /// Usage this close to the limit counts as under-provisioned.
    pub const PRESSURE_RATIO: f64 = 0.90;
    /// Average CPU below this (percent) is "idle".
    pub const IDLE_CPU_PERCENT: f64 = 5.0;
    /// Ignore apps with fewer samples than this — too little data to trust.
    pub const MIN_SAMPLES: i64 = 12;
    /// Approximate USD/month per GB of RAM on a typical small VPS, for savings
    /// estimates. Deliberately conservative; the UI labels it "approximate".
    pub const USD_PER_GB_MONTH: f64 = 1.50;
}

fn monthly_savings_usd(ram_bytes: i64) -> f64 {
    let gb = ram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    (gb * policy::USD_PER_GB_MONTH).max(0.0)
}

/// Produce right-sizing recommendations for the given apps. `colocation`
/// recommendations are added separately by [`colocation_recommendations`] since
/// they need cross-server context.
pub fn analyze(apps: &[AppUsage]) -> Vec<Recommendation> {
    let mut recs = Vec::new();

    for app in apps {
        // Skip apps without enough samples to make a confident call.
        if app.sample_count < policy::MIN_SAMPLES {
            continue;
        }

        let has_limit = app.current_memory_bytes > 0;

        // Over-provisioned: limit ≥ 2× peak → suggest peak + 20% headroom.
        if has_limit
            && app.peak_memory_bytes > 0
            && (app.current_memory_bytes as f64)
                >= app.peak_memory_bytes as f64 * policy::OVERPROVISION_RATIO
        {
            let suggested = ((app.peak_memory_bytes as f64) * policy::HEADROOM) as i64;
            let suggested = suggested.max(64 * 1024 * 1024); // never below 64 MB
            if suggested < app.current_memory_bytes {
                let saved = app.current_memory_bytes - suggested;
                recs.push(Recommendation {
                    app_id: app.app_id.clone(),
                    app_name: app.app_name.clone(),
                    kind: RecommendationKind::OverProvisioned,
                    message: format!(
                        "Memory limit ({}) is {:.1}× peak usage ({}). Lower it to {} (+20% headroom).",
                        fmt_bytes(app.current_memory_bytes),
                        app.current_memory_bytes as f64 / app.peak_memory_bytes.max(1) as f64,
                        fmt_bytes(app.peak_memory_bytes),
                        fmt_bytes(suggested),
                    ),
                    current_memory_bytes: app.current_memory_bytes,
                    suggested_memory_bytes: Some(suggested),
                    ram_saved_bytes: saved,
                    estimated_monthly_savings_usd: monthly_savings_usd(saved),
                    auto_applicable: true,
                });
            }
        }

        // Under-provisioned: peak usage near the limit → suggest a higher limit.
        if has_limit
            && (app.peak_memory_bytes as f64)
                >= app.current_memory_bytes as f64 * policy::PRESSURE_RATIO
        {
            let suggested = ((app.peak_memory_bytes as f64) * policy::HEADROOM) as i64;
            recs.push(Recommendation {
                app_id: app.app_id.clone(),
                app_name: app.app_name.clone(),
                kind: RecommendationKind::UnderProvisioned,
                message: format!(
                    "Peak memory ({}) is close to the limit ({}). Raise it to {} to avoid OOM kills.",
                    fmt_bytes(app.peak_memory_bytes),
                    fmt_bytes(app.current_memory_bytes),
                    fmt_bytes(suggested),
                ),
                current_memory_bytes: app.current_memory_bytes,
                suggested_memory_bytes: Some(suggested),
                ram_saved_bytes: 0,
                estimated_monthly_savings_usd: 0.0,
                auto_applicable: false,
            });
        }

        // Idle: low average CPU and not already in Ghost Mode → suggest it.
        if !app.ghost_mode_enabled && app.avg_cpu_percent < policy::IDLE_CPU_PERCENT {
            recs.push(Recommendation {
                app_id: app.app_id.clone(),
                app_name: app.app_name.clone(),
                kind: RecommendationKind::Idle,
                message: format!(
                    "Average CPU is {:.1}% — consistently idle. Enable Ghost Mode to hibernate it when unused.",
                    app.avg_cpu_percent
                ),
                current_memory_bytes: app.current_memory_bytes,
                suggested_memory_bytes: None,
                // Hibernation frees the whole footprint while asleep; credit the
                // current limit as potential savings (advisory).
                ram_saved_bytes: app.current_memory_bytes,
                estimated_monthly_savings_usd: monthly_savings_usd(app.current_memory_bytes),
                auto_applicable: true,
            });
        }
    }

    recs
}

/// A server's spare capacity, used for co-location suggestions.
#[derive(Debug, Clone)]
pub struct ServerCapacity {
    pub server_id: String,
    pub server_name: String,
    /// Fraction of RAM in use, 0.0–1.0 (from the latest server metrics).
    pub memory_used_ratio: f64,
    pub app_count: usize,
}

/// Suggest moving apps off the most-utilized server toward an underutilized one,
/// when more than one server exists. This is intentionally conservative — it
/// surfaces the opportunity; it never auto-moves.
pub fn colocation_recommendations(servers: &[ServerCapacity]) -> Vec<Recommendation> {
    if servers.len() < 2 {
        return Vec::new();
    }
    // Underutilized target: lowest memory ratio. Source: highest with apps.
    let mut by_util = servers.to_vec();
    by_util.sort_by(|a, b| a.memory_used_ratio.total_cmp(&b.memory_used_ratio));
    let target = &by_util[0];
    let source = by_util.last().unwrap();

    // Only suggest when there's a meaningful gap and the source has apps to move.
    if source.server_id == target.server_id
        || source.app_count == 0
        || source.memory_used_ratio - target.memory_used_ratio < 0.30
    {
        return Vec::new();
    }

    vec![Recommendation {
        app_id: String::new(),
        app_name: source.server_name.clone(),
        kind: RecommendationKind::Colocation,
        message: format!(
            "Server '{}' is {:.0}% full while '{}' is only {:.0}% full. Move low-resource apps to balance them.",
            source.server_name,
            source.memory_used_ratio * 100.0,
            target.server_name,
            target.memory_used_ratio * 100.0,
        ),
        current_memory_bytes: 0,
        suggested_memory_bytes: None,
        ram_saved_bytes: 0,
        estimated_monthly_savings_usd: 0.0,
        auto_applicable: false,
    }]
}

/// Spawn the weekly optimization digest (IF-191): analyzes persisted usage and,
/// if there's meaningful RAM to reclaim, emits a right-sizing notification.
pub fn spawn_optimization_digest(
    db: std::sync::Arc<dyn crate::db::Database>,
    caddy_admin_url: String,
) {
    use std::time::Duration;
    const WEEK: Duration = Duration::from_secs(7 * 24 * 60 * 60);
    // Wait before the first run so a fresh install accumulates some data.
    const INITIAL_DELAY: Duration = Duration::from_secs(24 * 60 * 60);

    tokio::spawn(async move {
        tokio::time::sleep(INITIAL_DELAY).await;
        loop {
            if let Ok(usage) = db.container_usage_stats(7).await {
                let apps = db.list_apps().await.unwrap_or_default();
                let usage_by_app: std::collections::HashMap<&str, _> =
                    usage.iter().map(|u| (u.app_id.as_str(), u)).collect();
                let inputs: Vec<AppUsage> = apps
                    .iter()
                    .filter_map(|a| {
                        let u = usage_by_app.get(a.id.as_str())?;
                        let mem = a
                            .resource_limits
                            .as_deref()
                            .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
                            .and_then(|v| v.get("memory_bytes").and_then(|m| m.as_i64()))
                            .unwrap_or(0);
                        Some(AppUsage {
                            app_id: a.id.clone(),
                            app_name: a.name.clone(),
                            current_memory_bytes: mem,
                            ghost_mode_enabled: a.ghost_mode_enabled,
                            avg_cpu_percent: u.avg_cpu_percent,
                            peak_memory_bytes: u.peak_memory_bytes,
                            sample_count: u.sample_count,
                        })
                    })
                    .collect();

                let recs = analyze(&inputs);
                let ram_saved: i64 = recs.iter().map(|r| r.ram_saved_bytes).sum();
                let count = recs.iter().filter(|r| r.ram_saved_bytes > 0).count();
                // Only notify when the saving is worth surfacing (≥256 MB).
                if ram_saved >= 256 * 1024 * 1024 && count > 0 {
                    let db_arc = db.clone();
                    crate::api::routes::notifications::emit_event(
                        &db_arc,
                        &caddy_admin_url,
                        "optimization.digest",
                        None,
                        &format!(
                            "You could save ~{} RAM by right-sizing {count} container(s).",
                            fmt_bytes(ram_saved)
                        ),
                        serde_json::json!({
                            "ram_saved_bytes": ram_saved,
                            "container_count": count,
                        }),
                    )
                    .await;
                }
            }
            tokio::time::sleep(WEEK).await;
        }
    });
}

/// Human-friendly byte formatting for messages (MB/GB).
fn fmt_bytes(bytes: i64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else {
        format!("{:.0} MB", mb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: i64 = 1024 * 1024;

    fn base(name: &str) -> AppUsage {
        AppUsage {
            app_id: name.into(),
            app_name: name.into(),
            current_memory_bytes: 512 * MB,
            ghost_mode_enabled: false,
            avg_cpu_percent: 40.0,
            peak_memory_bytes: 100 * MB,
            sample_count: 100,
        }
    }

    #[test]
    fn over_provisioned_suggests_lower_limit() {
        // 512MB limit, 100MB peak ⇒ 5.1× over → suggest 120MB.
        let recs = analyze(&[base("a")]);
        let over = recs
            .iter()
            .find(|r| r.kind == RecommendationKind::OverProvisioned)
            .expect("over-provisioned");
        assert_eq!(over.suggested_memory_bytes, Some(120 * MB));
        assert_eq!(over.ram_saved_bytes, 512 * MB - 120 * MB);
        assert!(over.auto_applicable);
        assert!(over.estimated_monthly_savings_usd > 0.0);
    }

    #[test]
    fn under_provisioned_when_near_limit() {
        let mut app = base("b");
        app.peak_memory_bytes = 500 * MB; // 500/512 = 0.98 ≥ 0.90
        let recs = analyze(&[app]);
        let under = recs
            .iter()
            .find(|r| r.kind == RecommendationKind::UnderProvisioned)
            .expect("under-provisioned");
        assert!(!under.auto_applicable);
        assert_eq!(under.ram_saved_bytes, 0);
    }

    #[test]
    fn idle_suggests_ghost_mode() {
        let mut app = base("c");
        app.avg_cpu_percent = 1.5;
        let recs = analyze(&[app]);
        assert!(recs.iter().any(|r| r.kind == RecommendationKind::Idle));
    }

    #[test]
    fn idle_skipped_when_ghost_mode_already_on() {
        let mut app = base("d");
        app.avg_cpu_percent = 1.0;
        app.ghost_mode_enabled = true;
        let recs = analyze(&[app]);
        assert!(!recs.iter().any(|r| r.kind == RecommendationKind::Idle));
    }

    #[test]
    fn low_sample_count_is_ignored() {
        let mut app = base("e");
        app.sample_count = 3;
        assert!(analyze(&[app]).is_empty());
    }

    #[test]
    fn colocation_needs_a_gap_and_two_servers() {
        let balanced = vec![
            ServerCapacity {
                server_id: "1".into(),
                server_name: "a".into(),
                memory_used_ratio: 0.5,
                app_count: 3,
            },
            ServerCapacity {
                server_id: "2".into(),
                server_name: "b".into(),
                memory_used_ratio: 0.55,
                app_count: 2,
            },
        ];
        assert!(colocation_recommendations(&balanced).is_empty());

        let lopsided = vec![
            ServerCapacity {
                server_id: "1".into(),
                server_name: "busy".into(),
                memory_used_ratio: 0.85,
                app_count: 5,
            },
            ServerCapacity {
                server_id: "2".into(),
                server_name: "spare".into(),
                memory_used_ratio: 0.20,
                app_count: 1,
            },
        ];
        let recs = colocation_recommendations(&lopsided);
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].kind, RecommendationKind::Colocation);
    }
}
