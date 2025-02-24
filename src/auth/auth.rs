use argon2::password_hash::rand_core::OsRng;
use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordVerifier, PasswordHasher};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{encode, errors::Error as JWTError, EncodingKey, Header};
use serde::Serialize;
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
    DbOperationFailed
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid username or password").into_response()
            }
            AuthError::TokenCreationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Token creation failed").into_response()
            }
            AuthError::PasswordHashFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Password hash failed").into_response()
            },
            AuthError::UserAlreadyExists => {
                (StatusCode::CONFLICT, "Username already exists").into_response()
            },
            AuthError::EmailAlreadyUsed => {
                (StatusCode::CONFLICT, "Email is already in use").into_response()
            },
            AuthError::DbOperationFailed => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database operation failed").into_response()
            }
        }
    }
}

const SECRET_KEY: &[u8] = b"amparoluvsboys";

pub fn create_jwt(user: &str) -> Result<String, JWTError> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 3600;
    let claims = Claims {
        sub: user.into(),
        exp: expiration as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(SECRET_KEY),
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