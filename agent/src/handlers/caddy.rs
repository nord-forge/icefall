use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;

use super::HandlerError;
use crate::context::HandlerContext;

#[derive(Debug, Deserialize)]
struct AddRouteParams {
    domain: String,
    upstream: String,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateRouteParams {
    domain: String,
    upstream: String,
}

#[derive(Debug, Deserialize)]
struct RemoveRouteParams {
    domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaddyRoute {
    #[serde(rename = "match")]
    matchers: Vec<CaddyMatch>,
    handle: Vec<CaddyHandler>,
    #[serde(skip_serializing_if = "Option::is_none")]
    terminal: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaddyMatch {
    host: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaddyHandler {
    handler: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    upstreams: Option<Vec<CaddyUpstream>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    root: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    try_files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaddyUpstream {
    dial: String,
}

fn reverse_proxy_route(domain: &str, path: Option<&str>, upstream: &str) -> CaddyRoute {
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

    CaddyRoute {
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
        }],
        terminal: Some(true),
    }
}

fn routes_url(caddy_url: &str) -> String {
    format!("{caddy_url}/config/apps/http/servers/srv0/routes")
}

async fn get_routes(ctx: &HandlerContext) -> Result<Vec<CaddyRoute>, HandlerError> {
    let url = routes_url(&ctx.caddy_url);
    let response = ctx.http.get(&url).send().await?;

    if response.status().as_u16() == 404 {
        return Ok(Vec::new());
    }

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(HandlerError::Other(format!("caddy API error: {body}")));
    }

    let routes: Vec<CaddyRoute> = response.json().await.unwrap_or_default();
    Ok(routes)
}

fn find_route_index(routes: &[CaddyRoute], domain: &str) -> Option<usize> {
    routes.iter().position(|r| {
        r.matchers
            .iter()
            .any(|m| m.host.contains(&domain.to_string()))
    })
}

pub async fn add_route(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: AddRouteParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let route = reverse_proxy_route(&p.domain, p.path.as_deref(), &p.upstream);
    let url = routes_url(&ctx.caddy_url);

    let response = ctx.http.post(&url).json(&route).send().await?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(HandlerError::Other(format!(
            "caddy add_route failed: {body}"
        )));
    }

    info!(domain = %p.domain, upstream = %p.upstream, "caddy route added");
    Ok(serde_json::json!({ "ok": true, "domain": p.domain }))
}

pub async fn update_route(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: UpdateRouteParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let routes = get_routes(ctx).await?;
    let index = find_route_index(&routes, &p.domain)
        .ok_or_else(|| HandlerError::Other(format!("route not found for domain: {}", p.domain)))?;

    let route = reverse_proxy_route(&p.domain, None, &p.upstream);
    let url = format!("{}/{index}", routes_url(&ctx.caddy_url));

    let response = ctx.http.put(&url).json(&route).send().await?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(HandlerError::Other(format!(
            "caddy update_route failed: {body}"
        )));
    }

    info!(domain = %p.domain, upstream = %p.upstream, "caddy route updated");
    Ok(serde_json::json!({ "ok": true, "domain": p.domain }))
}

pub async fn remove_route(ctx: &HandlerContext, params: Value) -> Result<Value, HandlerError> {
    let p: RemoveRouteParams =
        serde_json::from_value(params).map_err(|e| HandlerError::InvalidParams(e.to_string()))?;

    let routes = get_routes(ctx).await?;
    let index = find_route_index(&routes, &p.domain)
        .ok_or_else(|| HandlerError::Other(format!("route not found for domain: {}", p.domain)))?;

    let url = format!("{}/{index}", routes_url(&ctx.caddy_url));

    let response = ctx.http.delete(&url).send().await?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(HandlerError::Other(format!(
            "caddy remove_route failed: {body}"
        )));
    }

    info!(domain = %p.domain, "caddy route removed");
    Ok(serde_json::json!({ "ok": true, "domain": p.domain }))
}
