use argon2::password_hash::rand_core::OsRng;
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordVerifier, PasswordHasher};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dotenvy::dotenv;
use jsonwebtoken::{encode, errors::Error as JWTError, EncodingKey, Header};
use serde::Serialize;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct Claims {
    sub: String,
    exp: usize,
}

pub enum AuthError {
    InvalidCredentials,
    TokenCreationFailed,
    PasswordHashFailed,
    UserAlreadyExists,
    EmailAlreadyUsed,
    DatabaseOperationFailed
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "InvalidCredentials").into_response()
            }
            AuthError::TokenCreationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "TokenCreationFailed").into_response()
            }
            AuthError::PasswordHashFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "PasswordHashFailed").into_response()
            },
            AuthError::UserAlreadyExists => {
                (StatusCode::CONFLICT, "UsernameAlreadyExists").into_response()
            },
            AuthError::EmailAlreadyUsed => {
                (StatusCode::CONFLICT, "EmailAlreadyUsed").into_response()
            },
            AuthError::DatabaseOperationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "DatabaseOperationFailed").into_response()
            }
        }
    }
}

pub fn create_jwt(user_id: u64) -> Result<String, JWTError> {
    dotenv().expect("Failed to load environment variables!");
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 7200;
    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(
            env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set in the .env!")
                .as_bytes()
            )
    )
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| AuthError::PasswordHashFailed)?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(password: &str, hashed_password: &str) -> Result<bool, AuthError> {
    let parsed_hash = PasswordHash::new(hashed_password)
        .map_err(|_| AuthError::InvalidCredentials)?;

    Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok())
}