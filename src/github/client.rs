use reqwest::Client;
use serde::Deserialize;

/// Percent-encode a single URL path segment so a repo/owner/branch containing
/// reserved characters can't break out of its segment or inject query params.
fn enc(segment: &str) -> String {
    const ENCODE: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    percent_encoding::utf8_percent_encode(segment, ENCODE).to_string()
}

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

    /// List branch names for a repo (installation-token auth, paginated).
    pub async fn list_repo_branches(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<String>, String> {
        let mut branches = Vec::new();
        let mut page = 1u32;
        loop {
            let url = format!(
                "{}/repos/{}/{}/branches",
                self.api_url,
                enc(owner),
                enc(repo)
            );
            let resp = self
                .http
                .get(&url)
                .query(&[("per_page", "100"), ("page", &page.to_string())])
                .header("Authorization", format!("Bearer {token}"))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "Icefall-PaaS")
                .send()
                .await
                .map_err(|e| e.to_string())?;
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Failed to list branches ({status}): {body}"));
            }
            let page_branches: Vec<BranchRef> = resp.json().await.map_err(|e| e.to_string())?;
            let count = page_branches.len();
            branches.extend(page_branches.into_iter().map(|b| b.name));
            if count < 100 || page > 50 {
                break;
            }
            page += 1;
        }
        Ok(branches)
    }

    /// Create a repository webhook delivering `push`, `pull_request`, and `create`
    /// events to `url`, secured with `secret`. Returns the new webhook's id.
    pub async fn create_webhook(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        url: &str,
        secret: &str,
    ) -> Result<i64, String> {
        let api = format!("{}/repos/{}/{}/hooks", self.api_url, enc(owner), enc(repo));
        let payload = serde_json::json!({
            "name": "web",
            "active": true,
            "events": ["push", "pull_request", "create"],
            "config": {
                "url": url,
                "content_type": "json",
                "secret": secret,
                "insecure_ssl": "0",
            },
        });
        let resp = self
            .http
            .post(&api)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Icefall-PaaS")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Failed to create webhook ({status}): {body}"));
        }
        let created: WebhookCreated = resp.json().await.map_err(|e| e.to_string())?;
        Ok(created.id)
    }

    /// Create or update a commit status. `state` is one of pending/success/
    /// failure/error. `context` groups statuses (e.g. "icefall/deploy").
    #[allow(clippy::too_many_arguments)]
    pub async fn create_commit_status(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        sha: &str,
        state: &str,
        description: &str,
        context: &str,
        target_url: Option<&str>,
    ) -> Result<(), String> {
        let api = format!(
            "{}/repos/{}/{}/statuses/{}",
            self.api_url,
            enc(owner),
            enc(repo),
            enc(sha)
        );
        let mut payload = serde_json::json!({
            "state": state,
            // GitHub truncates descriptions at 140 chars.
            "description": description.chars().take(140).collect::<String>(),
            "context": context,
        });
        if let Some(url) = target_url {
            payload["target_url"] = serde_json::Value::String(url.to_string());
        }
        let resp = self
            .http
            .post(&api)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Icefall-PaaS")
            .json(&payload)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Failed to set commit status ({status}): {body}"));
        }
        Ok(())
    }

    /// Post a comment on an issue/PR (PRs are issues in GitHub's API). Returns
    /// the new comment's id so it can be edited later.
    pub async fn create_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        issue_number: i64,
        body: &str,
    ) -> Result<i64, String> {
        let api = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.api_url,
            enc(owner),
            enc(repo),
            issue_number
        );
        let resp = self
            .http
            .post(&api)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Icefall-PaaS")
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Failed to create comment ({status}): {text}"));
        }
        let created: CommentCreated = resp.json().await.map_err(|e| e.to_string())?;
        Ok(created.id)
    }

    /// Edit an existing issue/PR comment by id.
    pub async fn update_comment(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        comment_id: i64,
        body: &str,
    ) -> Result<(), String> {
        let api = format!(
            "{}/repos/{}/{}/issues/comments/{}",
            self.api_url,
            enc(owner),
            enc(repo),
            comment_id
        );
        let resp = self
            .http
            .patch(&api)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Icefall-PaaS")
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Failed to update comment ({status}): {text}"));
        }
        Ok(())
    }

    /// Find the open PR number for a branch, if any. Filters the pulls list by
    /// `head` (`owner:branch`). Note: only matches PRs from the same repo, not
    /// forks (a fork's head owner differs).
    pub async fn find_pr_for_branch(
        &self,
        token: &str,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<Option<i64>, String> {
        let api = format!("{}/repos/{}/{}/pulls", self.api_url, enc(owner), enc(repo));
        // Pass query params through reqwest so `head`/`branch` are encoded and
        // can't inject extra parameters.
        let resp = self
            .http
            .get(&api)
            .query(&[
                ("state", "open"),
                ("head", &format!("{owner}:{branch}")),
                ("per_page", "1"),
            ])
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "Icefall-PaaS")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Failed to find PR ({status}): {body}"));
        }
        let pulls: Vec<PullRef> = resp.json().await.map_err(|e| e.to_string())?;
        Ok(pulls.first().map(|p| p.number))
    }
}

#[derive(Deserialize)]
struct BranchRef {
    name: String,
}

#[derive(Deserialize)]
struct WebhookCreated {
    id: i64,
}

#[derive(Deserialize)]
struct CommentCreated {
    id: i64,
}

#[derive(Deserialize)]
struct PullRef {
    number: i64,
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
    fn enc_escapes_path_breaking_chars() {
        // Reserved/query chars must be percent-encoded so they can't break the
        // URL path or inject query parameters.
        assert_eq!(enc("a/b"), "a%2Fb");
        assert_eq!(enc("x?y=1&z=2"), "x%3Fy%3D1%26z%3D2");
        assert_eq!(enc("with space"), "with%20space");
        // Unreserved chars pass through unchanged.
        assert_eq!(enc("Owner-Repo_1.0~"), "Owner-Repo_1.0~");
    }

    #[test]
    fn new_preserves_custom_api_url() {
        let client = GitHubClient::new("https://github.example.com/api/v3");
        assert_eq!(client.api_url, "https://github.example.com/api/v3");
    }
}
