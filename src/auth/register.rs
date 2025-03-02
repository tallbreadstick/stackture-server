use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use super::auth::hash_password;
use super::auth::AuthError;
use super::auth::create_jwt;
use crate::debug::{log, LogType::HTTP};

#[derive(Serialize, Deserialize)]
pub struct RegisterRequest {
    username: String,
    email: String,
    password: String
}

#[derive(Serialize)]
pub struct RegisterResponse {
    token: String
}

#[axum::debug_handler]
pub async fn register(
    State(db): State<Pool<Postgres>>,
    Json(payload): Json<RegisterRequest>
) -> Result<Json<RegisterResponse>, AuthError> {
    log(HTTP, &format!("Anonymous user requested REGISTER as <{}> with <{}>", payload.username, payload.email));
    let user = sqlx::query!(
        "SELECT id, username, email FROM users WHERE username = $1 OR email = $2",
        payload.username,
        payload.email
    )
    .fetch_optional(&db)
    .await
    .map_err(|_| AuthError::DatabaseOperationFailed)?;
    if let Some(user) = user {
        if user.username == payload.username {
            return Err(AuthError::UserAlreadyExists);
        }
        if user.email == payload.email {
            return Err(AuthError::EmailAlreadyUsed);
        }
    }
    let hash = hash_password(&payload.password)?;
    let user_id = sqlx::query!(
        "INSERT INTO users (username, email, password) VALUES ($1, $2, $3) RETURNING id",
        payload.username,
        payload.email,
        hash
    )
    .fetch_one(&db)
    .await
    .map_err(|_| AuthError::TokenCreationFailed)?
    .id;
    let token = create_jwt(user_id)
        .map_err(|_| AuthError::TokenCreationFailed)?;

    Ok(Json(RegisterResponse { token }))
}