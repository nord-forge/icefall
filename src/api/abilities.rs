//! API token ability scoping (IF-168).
//!
//! Tokens may carry a JSON array of granted ability scopes. A `None`/null
//! abilities value means full access (the token inherits the user's
//! permissions). When a token IS scoped, every request it makes is checked
//! against the ability required by the route it targets.

use axum::http::Method;

/// All recognised ability scopes. Exposed so the API can advertise the valid
/// set to the token-creation UI.
pub const ALL_ABILITIES: &[&str] = &[
    "apps:read",
    "apps:write",
    "apps:deploy",
    "databases:read",
    "databases:write",
    "domains:read",
    "domains:write",
    "env:read",
    "env:write",
    "servers:read",
    "servers:write",
    "users:read",
    "users:write",
    "settings:read",
    "settings:write",
];

fn is_write(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

/// Determine the ability required to access `path` with `method`. Returns
/// `None` for routes that are not ability-gated (e.g. the caller's own profile,
/// auth, health) — those remain accessible to any authenticated token.
///
/// `path` is the full request path, e.g. `/api/v1/apps/abc/deploy`.
pub fn required_ability(method: &Method, path: &str) -> Option<String> {
    let rest = path.strip_prefix("/api/v1/").or_else(|| path.strip_prefix("/api/"))?;
    let segments: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    let resource = *segments.first()?;
    let write = is_write(method);

    // Deploy/rollback actions on apps need the dedicated deploy ability. These
    // are always writes (POST), so a read-only token can still GET deploy
    // history under apps:read.
    let is_deploy_action = segments
        .iter()
        .any(|s| matches!(*s, "deploy" | "deploys" | "redeploy" | "rollback" | "cancel"));
    if write && is_deploy_action && matches!(resource, "apps" | "projects" | "services" | "deploys")
    {
        return Some("apps:deploy".to_string());
    }

    // Environment variables are nested under apps but scoped separately.
    let touches_env = segments
        .iter()
        .any(|s| matches!(*s, "env" | "env-vars" | "environment-variables" | "variables"));
    if touches_env {
        return Some(if write { "env:write" } else { "env:read" }.to_string());
    }

    let prefix = match resource {
        "apps" | "projects" | "services" => "apps",
        "databases" => "databases",
        "domains" => "domains",
        "servers" => "servers",
        "users" | "invitations" | "teams" => "users",
        "settings" => "settings",
        // Unmapped resource: gate writes behind a broad scope so a read-only
        // token can never mutate, but leave reads open.
        _ => return write.then(|| "settings:write".to_string()),
    };
    Some(format!("{prefix}:{}", if write { "write" } else { "read" }))
}

/// Validate a requested ability list, returning only the recognised scopes.
/// Unknown scopes are dropped.
pub fn sanitize_abilities(requested: &[String]) -> Vec<String> {
    requested
        .iter()
        .filter(|a| ALL_ABILITIES.contains(&a.as_str()))
        .cloned()
        .collect()
}

/// Check whether a token's granted abilities satisfy the required ability.
pub fn granted(token_abilities: &[String], required: &str) -> bool {
    token_abilities.iter().any(|a| a == required)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_vs_write() {
        assert_eq!(
            required_ability(&Method::GET, "/api/v1/apps").as_deref(),
            Some("apps:read")
        );
        assert_eq!(
            required_ability(&Method::POST, "/api/v1/apps").as_deref(),
            Some("apps:write")
        );
    }

    #[test]
    fn deploy_action_maps_to_deploy() {
        assert_eq!(
            required_ability(&Method::POST, "/api/v1/apps/abc/deploy").as_deref(),
            Some("apps:deploy")
        );
        assert_eq!(
            required_ability(&Method::POST, "/api/v1/apps/abc/rollback").as_deref(),
            Some("apps:deploy")
        );
        // Reading deploy history is a read, not a deploy action.
        assert_eq!(
            required_ability(&Method::GET, "/api/v1/apps/abc/deploys").as_deref(),
            Some("apps:read")
        );
    }

    #[test]
    fn env_is_scoped_separately() {
        assert_eq!(
            required_ability(&Method::GET, "/api/v1/apps/abc/env-vars").as_deref(),
            Some("env:read")
        );
        assert_eq!(
            required_ability(&Method::PUT, "/api/v1/apps/abc/env-vars").as_deref(),
            Some("env:write")
        );
    }

    #[test]
    fn unmapped_read_is_open_write_is_gated() {
        assert_eq!(required_ability(&Method::GET, "/api/v1/health"), None);
        assert_eq!(
            required_ability(&Method::POST, "/api/v1/something-new").as_deref(),
            Some("settings:write")
        );
    }

    #[test]
    fn sanitize_drops_unknown() {
        let got = sanitize_abilities(&["apps:read".into(), "bogus:scope".into()]);
        assert_eq!(got, vec!["apps:read".to_string()]);
    }
}
