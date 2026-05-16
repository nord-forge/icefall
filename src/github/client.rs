use reqwest::Client;
use serde::Deserialize;

pub struct GitHubClient {
    http: Client,
    api_url: String,
}

#[derive(Debug, Deserialize)]
pub struct InstallationToken {
    pub token: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct GitHubRepo {
    pub id: i64,
    pub full_name: String,
    pub name: String,
    pub private: bool,
    pub default_branch: String,
    pub html_url: String,
}

#[derive(Deserialize)]
struct RepoListResponse {
    pub repositories: Vec<GitHubRepo>,
}

/// Response from the GitHub App Manifest code exchange endpoint.
#[derive(Debug, Deserialize)]
pub struct AppFromManifest {
    pub id: i64,
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub pem: String,
    pub webhook_secret: String,
    pub html_url: String,
    pub external_url: String,
}

impl GitHubClient {
    pub fn new(api_url: &str) -> Self {
        Self {
            http: Client::new(),
            api_url: api_url.trim_end_matches('/').to_string(),
        }
    }

    /// Exchange a manifest code for app credentials (step after user creates app on GitHub).
    pub async fn exchange_manifest_code(&self, code: &str) -> Result<AppFromManifest, String> {
        let url = format!("{}/app-manifests/{}/conversions", self.api_url, code);
        let resp = self
            .http
            .post(&url)
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Icefall-PaaS")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("GitHub API error ({status}): {body}"));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// Generate an installation access token using a JWT.
    pub async fn get_installation_token(
        &self,
        jwt: &str,
        installation_id: i64,
    ) -> Result<InstallationToken, String> {
        let url = format!(
            "{}/app/installations/{}/access_tokens",
            self.api_url, installation_id
        );
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Icefall-PaaS")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!(
                "Failed to get installation token ({status}): {body}"
            ));
        }

        resp.json().await.map_err(|e| e.to_string())
    }

    /// List repositories accessible to an installation.
    pub async fn list_installation_repos(&self, token: &str) -> Result<Vec<GitHubRepo>, String> {
        let mut all_repos = Vec::new();
        let mut page = 1u32;

        loop {
            let url = format!(
                "{}/installation/repositories?per_page=100&page={}",
                self.api_url, page
            );
            let resp = self
                .http
                .get(&url)
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "Icefall-PaaS")
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Failed to list repos ({status}): {body}"));
            }

            let response: RepoListResponse = resp.json().await.map_err(|e| e.to_string())?;
            let count = response.repositories.len();
            all_repos.extend(response.repositories);

            if count < 100 {
                break;
            }
            page += 1;

            // Safety limit to prevent infinite pagination
            if page > 50 {
                break;
            }
        }

        Ok(all_repos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_trims_trailing_slash() {
        let client = GitHubClient::new("https://api.github.com/");
        assert_eq!(client.api_url, "https://api.github.com");
    }

    #[test]
    fn new_preserves_custom_api_url() {
        let client = GitHubClient::new("https://github.example.com/api/v3");
        assert_eq!(client.api_url, "https://github.example.com/api/v3");
    }
}
