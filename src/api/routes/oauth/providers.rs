use crate::api::error::ApiError;
use crate::db::models::OAuthSettings;

pub(super) struct ProviderConfig {
    pub authorize_url: &'static str,
    pub token_url: &'static str,
    pub _user_info_url: &'static str,
    pub scopes: Vec<&'static str>,
}

pub(super) fn provider_config(provider: &str) -> Option<ProviderConfig> {
    match provider {
        "github" => Some(ProviderConfig {
            authorize_url: "https://github.com/login/oauth/authorize",
            token_url: "https://github.com/login/oauth/access_token",
            _user_info_url: "https://api.github.com/user",
            scopes: vec!["read:user", "user:email"],
        }),
        "google" => Some(ProviderConfig {
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
            token_url: "https://oauth2.googleapis.com/token",
            _user_info_url: "https://www.googleapis.com/oauth2/v2/userinfo",
            scopes: vec!["openid", "email", "profile"],
        }),
        _ => None,
    }
}

pub(super) fn get_client_credentials(
    settings: &OAuthSettings,
    provider: &str,
) -> Option<(String, String)> {
    match provider {
        "github" if settings.github_enabled => {
            match (&settings.github_client_id, &settings.github_client_secret) {
                (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
                    Some((id.clone(), secret.clone()))
                }
                _ => None,
            }
        }
        "google" if settings.google_enabled => {
            match (&settings.google_client_id, &settings.google_client_secret) {
                (Some(id), Some(secret)) if !id.is_empty() && !secret.is_empty() => {
                    Some((id.clone(), secret.clone()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Fetch user profile from the OAuth provider using the access token.
pub(super) async fn fetch_user_profile(
    provider: &str,
    access_token: &str,
    client: &reqwest::Client,
) -> Result<(String, Option<String>, Option<String>), ApiError> {
    match provider {
        "github" => fetch_github_profile(access_token, client).await,
        "google" => fetch_google_profile(access_token, client).await,
        _ => Err(ApiError::BadRequest(format!(
            "Unsupported provider: {provider}"
        ))),
    }
}

async fn fetch_github_profile(
    access_token: &str,
    client: &reqwest::Client,
) -> Result<(String, Option<String>, Option<String>), ApiError> {
    let user_info: serde_json::Value = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/json")
        .header("User-Agent", "icefall")
        .send()
        .await
        .map_err(ApiError::internal)?
        .json()
        .await
        .map_err(ApiError::internal)?;

    let provider_user_id = user_info["id"]
        .as_i64()
        .map(|id| id.to_string())
        .ok_or_else(|| ApiError::BadRequest("GitHub did not return a user ID".into()))?;

    let name = user_info["name"].as_str().map(String::from);

    // GitHub may not return email in the user endpoint — need to call /user/emails
    let mut email = user_info["email"].as_str().map(String::from);

    if email.is_none() {
        let emails: Vec<serde_json::Value> = client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("Bearer {access_token}"))
            .header("Accept", "application/json")
            .header("User-Agent", "icefall")
            .send()
            .await
            .map_err(ApiError::internal)?
            .json()
            .await
            .unwrap_or_default();

        // Prefer the primary verified email
        email = emails
            .iter()
            .filter(|e| e["verified"].as_bool() == Some(true))
            .find(|e| e["primary"].as_bool() == Some(true))
            .or_else(|| {
                emails
                    .iter()
                    .find(|e| e["verified"].as_bool() == Some(true))
            })
            .and_then(|e| e["email"].as_str().map(String::from));
    }

    Ok((provider_user_id, email, name))
}

async fn fetch_google_profile(
    access_token: &str,
    client: &reqwest::Client,
) -> Result<(String, Option<String>, Option<String>), ApiError> {
    let user_info: serde_json::Value = client
        .get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {access_token}"))
        .send()
        .await
        .map_err(ApiError::internal)?
        .json()
        .await
        .map_err(ApiError::internal)?;

    let provider_user_id = user_info["id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| ApiError::BadRequest("Google did not return a user ID".into()))?;

    let email = user_info["email"].as_str().map(String::from);
    let name = user_info["name"].as_str().map(String::from);

    Ok((provider_user_id, email, name))
}
