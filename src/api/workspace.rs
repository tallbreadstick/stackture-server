use super::api::ApiError;
use axum::{extract::State, Json};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use dotenvy::dotenv;
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::Serialize;
use serde_json::Value;
use sqlx::{Pool, Postgres};
use std::env;

// Tree Entity:
// CREATE TABLE trees (
//     id SERIAL PRIMARY KEY,
//     user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
//     title TEXT NOT NULL,
//     description TEXT,
//     tree_data JSONB NOT NULL,
//     created TIMESTAMPTZ DEFAULT now(),
//     updated TIMESTAMPTZ DEFAULT now()
// );

#[derive(Serialize)]
pub struct Workspace {
    id: i32,
    title: String,
    description: Option<String>,
}

async fn fetch_workspaces(
    State(db): State<Pool<Postgres>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<Json<Vec<Workspace>>, ApiError> {
    dotenv().expect("Failed to load environment variables!");
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env!");
    let token_data = decode::<Value>(
        auth.token(),
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| ApiError::InvalidToken)?;
    let user_id = token_data.claims
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or(ApiError::InvalidToken)?
        .parse::<i32>()
        .map_err(|_| ApiError::InvalidToken)?;
    let workspaces = sqlx::query_as!(
        Workspace,
        "SELECT id, title, description FROM trees WHERE user_id = $1",
        user_id
    )
    .fetch_all(&db)
    .await
    .map_err(|_| ApiError::DatabaseOperationFailed)?;
    Ok(Json(workspaces))
}
