use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use crate::debug::{log, LogType::HTTP};
use super::auth::{create_jwt, verify_password, AuthError};

#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String
}

pub async fn login(
    State(db): State<Pool<Postgres>>,
    Json(payload): Json<LoginRequest>
) -> Result<Json<LoginResponse>, AuthError> {
    log(HTTP, &format!("UserID <{}> requested LOGIN", payload.username));
    let user = sqlx::query!(
        "SELECT id, password FROM users WHERE username = $1",
        payload.username
    )
    .fetch_optional(&db)
    .await
    .map_err(|_| AuthError::DatabaseOperationFailed)?;
    let user = user.ok_or(AuthError::InvalidCredentials)?;
    if verify_password(&payload.password, &user.password)? {
        let token = create_jwt(user.id)
            .map_err(|_| AuthError::TokenCreationFailed)?;

        Ok(Json(LoginResponse { token }))
    } else {
        Err(AuthError::InvalidCredentials)
    }
}

