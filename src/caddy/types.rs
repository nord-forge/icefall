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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaddyUpstream {
    pub dial: String,
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
                },
                CaddyHandler {
                    handler: "vars".to_string(),
                    upstreams: None,
                    root: Some(root_path.to_string()),
                    try_files: None,
                    providers: None,
                },
                CaddyHandler {
                    handler: "file_server".to_string(),
                    upstreams: None,
                    root: Some(root_path.to_string()),
                    try_files: None,
                    providers: None,
                },
            ],
            terminal: Some(true),
        }
    }
}
