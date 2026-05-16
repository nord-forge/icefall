use crate::caddy::types::{CaddyRoute, RouteInfo};
use crate::caddy::{CaddyClient, CaddyError};

impl CaddyClient {
    pub async fn add_route(&self, domain: &str, upstream: &str) -> Result<(), CaddyError> {
        self.add_route_with_options(domain, None, upstream, None)
            .await
    }

    pub async fn add_route_with_path(
        &self,
        domain: &str,
        path: Option<&str>,
        upstream: &str,
    ) -> Result<(), CaddyError> {
        self.add_route_with_options(domain, path, upstream, None)
            .await
    }

    pub async fn add_route_with_options(
        &self,
        domain: &str,
        path: Option<&str>,
        upstream: &str,
        basic_auth: Option<(&str, &str)>,
    ) -> Result<(), CaddyError> {
        let mut route = CaddyRoute::reverse_proxy_with_path(domain, path, upstream);
        if let Some((username, password_hash)) = basic_auth {
            route = route.with_basic_auth(username, password_hash);
        }
        let url = format!("{}/config/apps/http/servers/srv0/routes", self.base_url());

        let response = self.client().post(&url).json(&route).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        Ok(())
    }

    /// Add a reverse_proxy route balanced across multiple upstreams, with the
    /// given load balancing policy and active/passive health checks.
    pub async fn add_route_balanced(
        &self,
        domain: &str,
        upstreams: &[String],
        policy: &str,
        health_check_path: &str,
    ) -> Result<(), CaddyError> {
        let route =
            CaddyRoute::reverse_proxy_balanced(domain, None, upstreams, policy, health_check_path);
        let url = format!("{}/config/apps/http/servers/srv0/routes", self.base_url());

        let response = self.client().post(&url).json(&route).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        Ok(())
    }

    /// Update an existing route to balance across multiple upstreams.
    pub async fn update_route_balanced(
        &self,
        domain: &str,
        upstreams: &[String],
        policy: &str,
        health_check_path: &str,
    ) -> Result<(), CaddyError> {
        let routes = self.get_routes_raw().await?;

        let index = routes
            .iter()
            .position(|r| {
                r.matchers
                    .iter()
                    .any(|m| m.host.contains(&domain.to_string()))
            })
            .ok_or_else(|| CaddyError::RouteNotFound(domain.to_string()))?;

        let route =
            CaddyRoute::reverse_proxy_balanced(domain, None, upstreams, policy, health_check_path);
        let url = format!(
            "{}/config/apps/http/servers/srv0/routes/{}",
            self.base_url(),
            index
        );

        let response = self.client().put(&url).json(&route).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        Ok(())
    }

    /// Set a balanced route: update it if the domain already has a route,
    /// otherwise add a new one.
    pub async fn set_route_balanced(
        &self,
        domain: &str,
        upstreams: &[String],
        policy: &str,
        health_check_path: &str,
    ) -> Result<(), CaddyError> {
        match self
            .update_route_balanced(domain, upstreams, policy, health_check_path)
            .await
        {
            Err(CaddyError::RouteNotFound(_)) => {
                self.add_route_balanced(domain, upstreams, policy, health_check_path)
                    .await
            }
            other => other,
        }
    }

    pub async fn remove_route(&self, domain: &str) -> Result<(), CaddyError> {
        let routes = self.get_routes_raw().await?;

        let index = routes
            .iter()
            .position(|r| {
                r.matchers
                    .iter()
                    .any(|m| m.host.contains(&domain.to_string()))
            })
            .ok_or_else(|| CaddyError::RouteNotFound(domain.to_string()))?;

        let url = format!(
            "{}/config/apps/http/servers/srv0/routes/{}",
            self.base_url(),
            index
        );

        let response = self.client().delete(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        Ok(())
    }

    pub async fn update_route(&self, domain: &str, new_upstream: &str) -> Result<(), CaddyError> {
        let routes = self.get_routes_raw().await?;

        let index = routes
            .iter()
            .position(|r| {
                r.matchers
                    .iter()
                    .any(|m| m.host.contains(&domain.to_string()))
            })
            .ok_or_else(|| CaddyError::RouteNotFound(domain.to_string()))?;

        let route = CaddyRoute::reverse_proxy(domain, new_upstream);
        let url = format!(
            "{}/config/apps/http/servers/srv0/routes/{}",
            self.base_url(),
            index
        );

        let response = self.client().put(&url).json(&route).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        Ok(())
    }

    pub async fn list_routes(&self) -> Result<Vec<RouteInfo>, CaddyError> {
        let routes = self.get_routes_raw().await?;

        let infos = routes
            .into_iter()
            .filter_map(|r| {
                let domain = r.matchers.first()?.host.first()?.clone();
                let upstream = r.handle.first()?.upstreams.as_ref()?.first()?.dial.clone();
                Some(RouteInfo { domain, upstream })
            })
            .collect();

        Ok(infos)
    }

    /// Add a file_server route for serving static files directly from disk.
    /// Uses try_files for SPA fallback (serves index.html for missing paths).
    pub async fn add_file_server_route(
        &self,
        domain: &str,
        root_path: &str,
    ) -> Result<(), CaddyError> {
        let route = CaddyRoute::file_server(domain, root_path);
        let url = format!("{}/config/apps/http/servers/srv0/routes", self.base_url());

        let response = self.client().post(&url).json(&route).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        Ok(())
    }

    /// Update an existing route to a file_server route for static files.
    pub async fn update_file_server_route(
        &self,
        domain: &str,
        root_path: &str,
    ) -> Result<(), CaddyError> {
        let routes = self.get_routes_raw().await?;

        let index = routes
            .iter()
            .position(|r| {
                r.matchers
                    .iter()
                    .any(|m| m.host.contains(&domain.to_string()))
            })
            .ok_or_else(|| CaddyError::RouteNotFound(domain.to_string()))?;

        let route = CaddyRoute::file_server(domain, root_path);
        let url = format!(
            "{}/config/apps/http/servers/srv0/routes/{}",
            self.base_url(),
            index
        );

        let response = self.client().put(&url).json(&route).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        Ok(())
    }

    pub async fn add_wildcard(&self, base_domain: &str) -> Result<(), CaddyError> {
        let wildcard = format!("*.{base_domain}");
        self.add_route(&wildcard, "localhost:0").await
    }

    async fn get_routes_raw(&self) -> Result<Vec<CaddyRoute>, CaddyError> {
        let url = format!("{}/config/apps/http/servers/srv0/routes", self.base_url());

        let response = self.client().get(&url).send().await?;

        if response.status().as_u16() == 404 {
            return Ok(Vec::new());
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(CaddyError::ApiError { status, body });
        }

        let routes: Vec<CaddyRoute> = response.json().await.unwrap_or_default();
        Ok(routes)
    }
}
