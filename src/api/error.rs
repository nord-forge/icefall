use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use thiserror::Error;

use crate::build::BuildError;
use crate::caddy::CaddyError;
use crate::db::DbError;
use crate::deploy::DeployError;
use crate::docker::DockerError;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Conflict(String),
    #[error("internal error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg.clone()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            ApiError::Internal(err) => {
                tracing::error!(error = %err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error".to_string(),
                )
            }
            ApiError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                msg.clone(),
            ),
        };

        let body = serde_json::json!({ "error": code, "message": message });
        (status, Json(body)).into_response()
    }
}

impl ApiError {
    pub fn internal(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal(Box::new(err))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::internal(err)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::internal(err)
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(msg) => ApiError::NotFound(msg),
            DbError::Duplicate(msg) => ApiError::Conflict(msg),
            other => ApiError::internal(other),
        }
    }
}

impl From<DockerError> for ApiError {
    fn from(err: DockerError) -> Self {
        match err {
            DockerError::ContainerNotFound(msg) => ApiError::NotFound(msg),
            DockerError::ImageNotFound(msg) => ApiError::NotFound(msg),
            DockerError::Unavailable(msg) => ApiError::ServiceUnavailable(msg),
            other => ApiError::internal(other),
        }
    }
}

impl From<CaddyError> for ApiError {
    fn from(err: CaddyError) -> Self {
        match err {
            CaddyError::RouteNotFound(msg) => ApiError::NotFound(msg),
            CaddyError::Unreachable(msg) => ApiError::ServiceUnavailable(msg),
            other => ApiError::internal(other),
        }
    }
}

impl From<BuildError> for ApiError {
    fn from(err: BuildError) -> Self {
        ApiError::internal(err)
    }
}

impl From<DeployError> for ApiError {
    fn from(err: DeployError) -> Self {
        ApiError::internal(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn response_json(resp: Response) -> serde_json::Value {
        let (parts, body) = resp.into_parts();
        let bytes = to_bytes(body, 1024 * 1024).await.unwrap();
        let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        serde_json::json!({ "status": parts.status.as_u16(), "body": val })
    }

    #[tokio::test]
    async fn not_found_returns_404_with_code() {
        let err = ApiError::NotFound("app xyz".into());
        let resp = err.into_response();
        let json = response_json(resp).await;
        assert_eq!(json["status"], 404);
        assert_eq!(json["body"]["error"], "not_found");
        assert_eq!(json["body"]["message"], "app xyz");
    }

    #[tokio::test]
    async fn bad_request_returns_400_with_code() {
        let err = ApiError::BadRequest("missing field".into());
        let resp = err.into_response();
        let json = response_json(resp).await;
        assert_eq!(json["status"], 400);
        assert_eq!(json["body"]["error"], "bad_request");
        assert_eq!(json["body"]["message"], "missing field");
    }

    #[tokio::test]
    async fn forbidden_returns_403_with_code() {
        let err = ApiError::Forbidden("admin only".into());
        let resp = err.into_response();
        let json = response_json(resp).await;
        assert_eq!(json["status"], 403);
        assert_eq!(json["body"]["error"], "forbidden");
    }

    #[tokio::test]
    async fn conflict_returns_409_with_code() {
        let err = ApiError::Conflict("already exists".into());
        let resp = err.into_response();
        let json = response_json(resp).await;
        assert_eq!(json["status"], 409);
        assert_eq!(json["body"]["error"], "conflict");
    }

    #[tokio::test]
    async fn internal_returns_500_and_hides_details() {
        let err = ApiError::internal(std::io::Error::other("secret db password in error"));
        let resp = err.into_response();
        let json = response_json(resp).await;
        assert_eq!(json["status"], 500);
        assert_eq!(json["body"]["error"], "internal_error");
        assert_eq!(json["body"]["message"], "Internal server error");
        assert!(!json["body"]["message"].as_str().unwrap().contains("secret"));
    }

    #[tokio::test]
    async fn service_unavailable_returns_503() {
        let err = ApiError::ServiceUnavailable("Docker unreachable".into());
        let resp = err.into_response();
        let json = response_json(resp).await;
        assert_eq!(json["status"], 503);
        assert_eq!(json["body"]["error"], "service_unavailable");
    }

    #[test]
    fn from_db_not_found_maps_to_404() {
        let db_err = DbError::NotFound("user 123".into());
        let api_err: ApiError = db_err.into();
        assert!(matches!(api_err, ApiError::NotFound(msg) if msg == "user 123"));
    }

    #[test]
    fn from_db_duplicate_maps_to_conflict() {
        let db_err = DbError::Duplicate("email taken".into());
        let api_err: ApiError = db_err.into();
        assert!(matches!(api_err, ApiError::Conflict(msg) if msg == "email taken"));
    }

    #[test]
    fn from_db_sqlx_maps_to_internal() {
        let db_err = DbError::Sqlx(sqlx::Error::RowNotFound);
        let api_err: ApiError = db_err.into();
        assert!(matches!(api_err, ApiError::Internal(_)));
    }

    #[test]
    fn from_docker_container_not_found_maps_to_404() {
        let err = DockerError::ContainerNotFound("abc123".into());
        let api_err: ApiError = err.into();
        assert!(matches!(api_err, ApiError::NotFound(_)));
    }

    #[test]
    fn from_docker_unavailable_maps_to_503() {
        let err = DockerError::Unavailable("socket missing".into());
        let api_err: ApiError = err.into();
        assert!(matches!(api_err, ApiError::ServiceUnavailable(_)));
    }

    #[test]
    fn from_caddy_unreachable_maps_to_503() {
        let err = CaddyError::Unreachable("connection refused".into());
        let api_err: ApiError = err.into();
        assert!(matches!(api_err, ApiError::ServiceUnavailable(_)));
    }

    #[test]
    fn from_caddy_route_not_found_maps_to_404() {
        let err = CaddyError::RouteNotFound("example.com".into());
        let api_err: ApiError = err.into();
        assert!(matches!(api_err, ApiError::NotFound(_)));
    }

    #[test]
    fn from_io_error_maps_to_internal() {
        let err = std::io::Error::other("disk full");
        let api_err: ApiError = err.into();
        assert!(matches!(api_err, ApiError::Internal(_)));
    }

    #[test]
    fn internal_helper_wraps_error() {
        let err = ApiError::internal(std::io::Error::other("test"));
        assert!(matches!(err, ApiError::Internal(_)));
    }

    #[tokio::test]
    async fn response_body_has_error_and_message_fields() {
        let err = ApiError::BadRequest("test".into());
        let resp = err.into_response();
        let json = response_json(resp).await;
        let body = &json["body"];
        assert!(body.get("error").is_some());
        assert!(body.get("message").is_some());
        assert!(body.get("data").is_none());
    }
}
