use axum::{http::StatusCode, response::IntoResponse, response::Response, Json};
use serde_json::json;

use crate::storage::StorageError;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("resource not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl From<StorageError> for AppError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound => Self::NotFound,
            StorageError::Conflict(s) => Self::Conflict(s),
            StorageError::Other(e) => Self::Internal(e),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        if let Self::Internal(ref err) = self {
            tracing::error!(error = %err, "internal error");
        }

        let body = Json(json!({
            "error": self.to_string(),
        }));
        (status, body).into_response()
    }
}
