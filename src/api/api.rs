use axum::{http::StatusCode, response::{IntoResponse, Response}};

pub enum ApiError {
    DatabaseOperationFailed,
    InvalidToken
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::DatabaseOperationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseOperationFailed").into_response()
            },
            ApiError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, "InvalidToken").into_response()
            }
        }
    }
}