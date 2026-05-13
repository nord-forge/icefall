use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::NewUser;

use super::pkce::{store_pkce, take_pkce};
use super::providers::{fetch_user_profile, get_client_credentials, provider_config};

// URL encoding helper (minimal, no extra dependency)
mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 3);
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
        result
    }
}

/// GET /api/v1/auth/oauth/{provider}/authorize
/// Generates PKCE challenge, builds authorization URL, redirects to provider.
pub(super) async fn oauth_authorize(
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> Result<axum::response::Redirect, ApiError> {
    let prov_config = provider_config(&provider)
        .ok_or_else(|| ApiError::BadRequest(format!("Unsupported OAuth provider: {provider}")))?;

    let oauth_settings = state.db.get_oauth_settings().await?.ok_or_else(|| {
        ApiError::BadRequest(
            "OAuth is not configured. Ask an admin to configure OAuth settings.".into(),
        )
    })?;

    let (client_id, _client_secret) = get_client_credentials(&oauth_settings, &provider)
        .ok_or_else(|| {
            ApiError::BadRequest(format!(
                "OAuth provider '{provider}' is not enabled or configured"
            ))
        })?;

    use base64::Engine;
    use sha2::Digest;

    let (code_verifier, code_challenge, state_token) = {
        use rand::RngCore;
        let mut rng = rand::rng();
        let mut verifier_bytes = [0u8; 32];
        rng.fill_bytes(&mut verifier_bytes);
        let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);

        let mut hasher = sha2::Sha256::new();
        hasher.update(verifier.as_bytes());
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());

        let mut state_bytes = [0u8; 32];
        rng.fill_bytes(&mut state_bytes);
        let token = hex::encode(state_bytes);
        (verifier, challenge, token)
    };

    // Store PKCE verifier keyed by state token
    store_pkce(&state_token, &code_verifier).await;

    // Build the callback URL
    let base = state.config.base_domain.as_deref().map_or_else(
        || "http://localhost:3000".to_string(),
        |d| format!("https://{d}"),
    );
    let redirect_uri = format!("{base}/api/v1/auth/oauth/{provider}/callback");

    // Build authorization URL
    let scope = prov_config.scopes.join(" ");
    let mut params = vec![
        ("client_id", client_id.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("scope", scope.as_str()),
        ("state", state_token.as_str()),
        ("response_type", "code"),
        ("code_challenge", code_challenge.as_str()),
        ("code_challenge_method", "S256"),
    ];

    // Google requires access_type=offline for refresh tokens
    if provider == "google" {
        params.push(("access_type", "offline"));
        params.push(("prompt", "consent"));
    }

    let query_string: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    let authorize_url = format!("{}?{}", prov_config.authorize_url, query_string);

    Ok(Redirect::temporary(&authorize_url))
}

#[derive(Deserialize)]
pub(super) struct OAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// GET /api/v1/auth/oauth/{provider}/callback
/// Exchanges authorization code for token, fetches user info, creates/links account.
pub(super) async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<axum::response::Response, ApiError> {
    // Handle provider errors
    if let Some(ref err) = query.error {
        tracing::warn!("OAuth callback error from {provider}: {err}");
        return Ok(Redirect::temporary("/login?error=oauth_denied").into_response());
    }

    let code = query
        .code
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code".into()))?;

    let state_token = query
        .state
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".into()))?;

    // Validate state and retrieve PKCE verifier
    let code_verifier = take_pkce(state_token).await.ok_or_else(|| {
        ApiError::BadRequest("Invalid or expired OAuth state. Please try again.".into())
    })?;

    let prov_config = provider_config(&provider)
        .ok_or_else(|| ApiError::BadRequest(format!("Unsupported OAuth provider: {provider}")))?;

    let oauth_settings = state
        .db
        .get_oauth_settings()
        .await?
        .ok_or_else(|| ApiError::BadRequest("OAuth is not configured".into()))?;

    let (client_id, client_secret) = get_client_credentials(&oauth_settings, &provider)
        .ok_or_else(|| {
            ApiError::BadRequest(format!("OAuth provider '{provider}' is not configured"))
        })?;

    let base = state.config.base_domain.as_deref().map_or_else(
        || "http://localhost:3000".to_string(),
        |d| format!("https://{d}"),
    );
    let redirect_uri = format!("{base}/api/v1/auth/oauth/{provider}/callback");

    // Exchange code for access token
    let http_client = reqwest::Client::new();
    let token_response = http_client
        .post(prov_config.token_url)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!("OAuth token exchange failed for {provider}: {e}");
            ApiError::internal(e)
        })?;

    if !token_response.status().is_success() {
        let body = token_response.text().await.unwrap_or_default();
        tracing::error!("OAuth token exchange returned error for {provider}: {body}");
        return Ok(Redirect::temporary("/login?error=oauth_failed").into_response());
    }

    let token_data: serde_json::Value = token_response.json().await.map_err(|e| {
        tracing::error!("Failed to parse OAuth token response from {provider}: {e}");
        ApiError::internal(e)
    })?;

    let access_token = token_data["access_token"].as_str().ok_or_else(|| {
        tracing::error!("No access_token in OAuth response from {provider}");
        ApiError::BadRequest("OAuth token exchange did not return an access token".into())
    })?;

    // Fetch user profile from provider
    let (provider_user_id, provider_email, _provider_name) =
        fetch_user_profile(&provider, access_token, &http_client).await?;

    // Account linking logic
    let user = match state
        .db
        .get_oauth_identity(&provider, &provider_user_id)
        .await?
    {
        // Case 1: OAuth identity exists — log in as the linked user
        Some(identity) => state
            .db
            .get_user_by_id(&identity.user_id)
            .await?
            .ok_or_else(|| {
                ApiError::internal(std::io::Error::other("Linked user account not found"))
            })?,
        // Case 2: No OAuth identity yet
        None => {
            // Check if there's an existing user with matching email
            let existing_user = if let Some(ref email) = provider_email {
                state.db.get_user_by_email(email).await?
            } else {
                None
            };

            match existing_user {
                // Case 2a: Email matches existing user — link identity
                Some(user) => {
                    state
                        .db
                        .create_oauth_identity(
                            &user.id,
                            &provider,
                            &provider_user_id,
                            provider_email.as_deref(),
                        )
                        .await?;
                    user
                }
                // Case 2b: New user — create account + link identity
                None => {
                    let email = provider_email.ok_or_else(|| {
                        ApiError::BadRequest(
                            "OAuth provider did not return an email address. Cannot create account."
                                .into(),
                        )
                    })?;

                    // Generate a random password hash (user can set a password later if needed)
                    let random_password = format!("oauth-{}", uuid::Uuid::now_v7());
                    let password_hash = hash_password_for_oauth(&random_password)?;

                    let new_user = state
                        .db
                        .create_user(&NewUser {
                            email: email.clone(),
                            password_hash,
                            role: "deployer".to_string(),
                        })
                        .await?;

                    state
                        .db
                        .create_oauth_identity(
                            &new_user.id,
                            &provider,
                            &provider_user_id,
                            Some(&email),
                        )
                        .await?;

                    new_user
                }
            }
        }
    };

    // Create session
    let expires_at = (chrono::Utc::now() + chrono::Duration::days(7))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let session = state
        .db
        .create_session(&user.id, &expires_at)
        .await
        .map_err(ApiError::internal)?;

    // Set cookie and redirect to dashboard
    let cookie = format!(
        "icefall_session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800",
        session.id
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&cookie).map_err(ApiError::internal)?,
    );
    headers.insert("location", HeaderValue::from_static("/"));

    Ok((axum::http::StatusCode::TEMPORARY_REDIRECT, headers).into_response())
}

pub(super) fn hash_password_for_oauth(password: &str) -> Result<String, ApiError> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::internal(std::io::Error::other(e.to_string())))?;
    Ok(hash.to_string())
}
