use axum::body::Body;
use axum::extract::{FromRequest, FromRequestParts, Query as AxumQuery};
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use validator::Validate;

#[derive(Debug, thiserror::Error)]
pub enum ValidationRejection {
    #[error("Invalid JSON: {0}")]
    JsonError(String),
    #[error("Validation failed: {0}")]
    Validation(String),
    #[error("Invalid query: {0}")]
    QueryError(String),
}

impl IntoResponse for ValidationRejection {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            Self::JsonError(msg) => (
                axum::http::StatusCode::BAD_REQUEST,
                serde_json::json!({ "error": msg }),
            ),
            Self::Validation(msg) => (
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                serde_json::json!({ "error": "Validation failed", "details": msg }),
            ),
            Self::QueryError(msg) => (
                axum::http::StatusCode::BAD_REQUEST,
                serde_json::json!({ "error": msg }),
            ),
        };
        (status, axum::Json(body)).into_response()
    }
}

pub struct ValidatedJson<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S, Body> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ValidationRejection;

    async fn from_request(req: axum::http::Request<Body>, state: &S) -> Result<Self, Self::Rejection> {
        let axum::Json(value) = axum::Json::<T>::from_request(req, state)
            .await
            .map_err(|e| ValidationRejection::JsonError(e.body_text()))?;
        value.validate().map_err(|e| ValidationRejection::Validation(e.to_string()))?;
        Ok(ValidatedJson(value))
    }
}

pub struct ValidatedQuery<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ValidationRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let AxumQuery(value) = AxumQuery::<T>::from_request_parts(parts, _state)
            .await
            .map_err(|e| ValidationRejection::QueryError(e.to_string()))?;
        value.validate().map_err(|e| ValidationRejection::Validation(e.to_string()))?;
        Ok(ValidatedQuery(value))
    }
}