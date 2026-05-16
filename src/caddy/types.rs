use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyRoute {
    #[serde(rename = "match")]
    pub matchers: Vec<CaddyMatch>,
    pub handle: Vec<CaddyHandler>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyMatch {
    pub host: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyHandler {
    pub handler: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstreams: Option<Vec<CaddyUpstream>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub try_files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_balancing: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_checks: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyUpstream {
    pub dial: String,
}

/// Map an Icefall lb_policy value to a Caddy selection policy name.
pub fn caddy_lb_policy(policy: &str) -> &'static str {
    match policy {
        "least_conn" => "least_conn",
        "ip_hash" => "ip_hash",
        "random" => "random",
        _ => "round_robin",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteInfo {
    pub domain: String,
    pub upstream: String,
}

impl CaddyRoute {
    pub fn reverse_proxy(domain: &str, upstream: &str) -> Self {
        Self::reverse_proxy_with_path(domain, None, upstream)
    }

    pub fn reverse_proxy_with_path(domain: &str, path: Option<&str>, upstream: &str) -> Self {
        let path_matcher = path.map(|p| {
            let pattern = if p.ends_with('*') {
                p.to_string()
            } else if p.ends_with('/') {
                format!("{p}*")
            } else {
                format!("{p}/*")
            };
            vec![pattern]
        });

        Self {
            matchers: vec![CaddyMatch {
                host: vec![domain.to_string()],
                path: path_matcher,
            }],
            handle: vec![CaddyHandler {
                handler: "reverse_proxy".to_string(),
                upstreams: Some(vec![CaddyUpstream {
                    dial: upstream.to_string(),
                }]),
                root: None,
                try_files: None,
                providers: None,
                load_balancing: None,
                health_checks: None,
            }],
            terminal: Some(true),
        }
    }

    /// Create a reverse_proxy route with multiple upstreams, a load balancing
    /// policy, and passive + active health checks.
    pub fn reverse_proxy_balanced(
        domain: &str,
        path: Option<&str>,
        upstreams: &[String],
        policy: &str,
        health_check_path: &str,
    ) -> Self {
        let path_matcher = path.map(|p| {
            let pattern = if p.ends_with('*') {
                p.to_string()
            } else if p.ends_with('/') {
                format!("{p}*")
            } else {
                format!("{p}/*")
            };
            vec![pattern]
        });

        let upstreams: Vec<CaddyUpstream> = upstreams
            .iter()
            .map(|u| CaddyUpstream { dial: u.clone() })
            .collect();

        Self {
            matchers: vec![CaddyMatch {
                host: vec![domain.to_string()],
                path: path_matcher,
            }],
            handle: vec![CaddyHandler {
                handler: "reverse_proxy".to_string(),
                upstreams: Some(upstreams),
                root: None,
                try_files: None,
                providers: None,
                load_balancing: Some(serde_json::json!({
                    "selection_policy": { "policy": caddy_lb_policy(policy) }
                })),
                health_checks: Some(serde_json::json!({
                    "passive": {
                        "fail_duration": "30s",
                        "max_fails": 3,
                        "unhealthy_latency": "5s"
                    },
                    "active": {
                        "interval": "10s",
                        "path": health_check_path
                    }
                })),
            }],
            terminal: Some(true),
        }
    }

    pub fn with_basic_auth(mut self, username: &str, password_hash: &str) -> Self {
        let auth_handler = CaddyHandler {
            handler: "authentication".to_string(),
            upstreams: None,
            root: None,
            try_files: None,
            load_balancing: None,
            health_checks: None,
            providers: Some(serde_json::json!({
                "http_basic": {
                    "accounts": [{
                        "username": username,
                        "password": password_hash,
                    }],
                    "hash": {
                        "algorithm": "bcrypt"
                    }
                }
            })),
        };
        self.handle.insert(0, auth_handler);
        self
    }

    /// Create a file_server route for serving static files with SPA fallback.
    pub fn file_server(domain: &str, root_path: &str) -> Self {
        Self {
            matchers: vec![CaddyMatch {
                host: vec![domain.to_string()],
                path: None,
            }],
            handle: vec![
                // rewrite handler for SPA try_files
                CaddyHandler {
                    handler: "rewrite".to_string(),
                    upstreams: None,
                    root: None,
                    try_files: Some(vec![
                        "{http.request.uri.path}".to_string(),
                        "{http.request.uri.path}/index.html".to_string(),
                        "/index.html".to_string(),
                    ]),
                    providers: None,
                    load_balancing: None,
                    health_checks: None,
                },
                CaddyHandler {
                    handler: "vars".to_string(),
                    upstreams: None,
                    root: Some(root_path.to_string()),
                    try_files: None,
                    providers: None,
                    load_balancing: None,
                    health_checks: None,
                },
                CaddyHandler {
                    handler: "file_server".to_string(),
                    upstreams: None,
                    root: Some(root_path.to_string()),
                    try_files: None,
                    providers: None,
                    load_balancing: None,
                    health_checks: None,
                },
            ],
            terminal: Some(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lb_policy_maps_known_values() {
        assert_eq!(caddy_lb_policy("round_robin"), "round_robin");
        assert_eq!(caddy_lb_policy("least_conn"), "least_conn");
        assert_eq!(caddy_lb_policy("ip_hash"), "ip_hash");
        assert_eq!(caddy_lb_policy("random"), "random");
    }

    #[test]
    fn lb_policy_falls_back_to_round_robin() {
        assert_eq!(caddy_lb_policy("nonsense"), "round_robin");
        assert_eq!(caddy_lb_policy(""), "round_robin");
    }

    #[test]
    fn balanced_route_lists_all_upstreams() {
        let upstreams = vec!["server1:8001".to_string(), "server2:8001".to_string()];
        let route = CaddyRoute::reverse_proxy_balanced(
            "app.example.com",
            None,
            &upstreams,
            "least_conn",
            "/health",
        );
        let handler = &route.handle[0];
        let ups = handler.upstreams.as_ref().unwrap();
        assert_eq!(ups.len(), 2);
        assert_eq!(ups[0].dial, "server1:8001");
        assert_eq!(ups[1].dial, "server2:8001");
    }

    #[test]
    fn balanced_route_sets_policy_and_health_checks() {
        let upstreams = vec!["a:80".to_string()];
        let route = CaddyRoute::reverse_proxy_balanced(
            "app.example.com",
            None,
            &upstreams,
            "ip_hash",
            "/healthz",
        );
        let handler = &route.handle[0];

        let lb = handler.load_balancing.as_ref().unwrap();
        assert_eq!(lb["selection_policy"]["policy"], "ip_hash");

        let hc = handler.health_checks.as_ref().unwrap();
        assert_eq!(hc["passive"]["max_fails"], 3);
        assert_eq!(hc["active"]["path"], "/healthz");
        assert_eq!(hc["active"]["interval"], "10s");
    }

    #[test]
    fn single_upstream_route_omits_lb_fields_in_json() {
        let route = CaddyRoute::reverse_proxy("app.example.com", "localhost:8000");
        let json = serde_json::to_string(&route).unwrap();
        assert!(!json.contains("load_balancing"));
        assert!(!json.contains("health_checks"));
    }
}
