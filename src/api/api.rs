use std::env;

use axum::{http::StatusCode, response::{IntoResponse, Response}};
use axum_extra::headers::{authorization::Bearer, Authorization};
use dotenvy::dotenv;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub enum ApiError {
    DatabaseOperationFailed,
    InvalidToken,
    UnauthorizedAccess
}

#[derive(Serialize, Deserialize)]
pub struct TokenData {
    pub user_id: i32,
    pub exp: usize
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::DatabaseOperationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseOperationFailed").into_response()
            },
            ApiError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, "InvalidToken").into_response()
            },
            ApiError::UnauthorizedAccess => {
                (StatusCode::UNAUTHORIZED, "UnauthorizedAccess").into_response()
            }
        }
    }
}

pub fn extract_token_data(auth: Authorization<Bearer>) -> Result<TokenData, ApiError> {
    dotenv().expect("Failed to load environment variables!");
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be defined in .env!");
    let token_data = decode::<Value>(
        auth.token(),
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default()
    )
    .map_err(|_| ApiError::InvalidToken)?;
    Ok(TokenData {
        user_id: token_data.claims
            .get("sub")
            .and_then(|v| v.as_str())
            .ok_or(ApiError::InvalidToken)?
            .parse::<i32>()
            .map_err(|_| ApiError::InvalidToken)?,
        exp: token_data.claims
            .get("exp")
            .and_then(|v| v.as_str())
            .ok_or(ApiError::InvalidToken)?
            .parse::<usize>()
            .map_err(|_| ApiError::InvalidToken)?
    })
}