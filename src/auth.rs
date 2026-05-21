use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    RequestPartsExt,
};
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::db::Database;
use crate::api::ApiState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,  // username
    pub user_id: i64,
    pub is_admin: bool,
    pub exp: usize,   // expiration time
}

#[derive(Debug, Clone)]
pub struct AuthState {
    pub jwt_secret: String,
    pub db: Arc<Database>,
}

#[async_trait]
impl FromRequestParts<ApiState> for AuthState {
    type Rejection = Response;

    async fn from_request_parts(_parts: &mut Parts, state: &ApiState) -> Result<Self, Self::Rejection> {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change-this-secret-key-in-production".to_string());

        Ok(AuthState {
            jwt_secret,
            db: Arc::clone(&state.db),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: i64,
    pub username: String,
    pub is_admin: bool,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AuthState: FromRequestParts<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_state = AuthState::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response())?;

        let jar = parts
            .extract::<CookieJar>()
            .await
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response())?;

        let token = jar
            .get("token")
            .map(|c| c.value())
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Unauthorized").into_response())?;

        let claims = decode::<Claims>(
            token,
            &DecodingKey::from_secret(auth_state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token").into_response())?
        .claims;

        Ok(AuthUser {
            user_id: claims.user_id,
            username: claims.sub,
            is_admin: claims.is_admin,
        })
    }
}

// 可选的认证用户（不强制要求登录）
#[derive(Debug, Clone)]
pub struct OptionalAuthUser(pub Option<AuthUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let jar = parts
            .extract::<CookieJar>()
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Internal error").into_response())?;

        let token = jar.get("token").map(|c| c.value());

        if token.is_none() {
            return Ok(OptionalAuthUser(None));
        }

        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change-this-secret-key-in-production".to_string());

        let claims = match decode::<Claims>(
            token.unwrap(),
            &DecodingKey::from_secret(jwt_secret.as_bytes()),
            &Validation::default(),
        ) {
            Ok(token_data) => token_data.claims,
            Err(_) => return Ok(OptionalAuthUser(None)),
        };

        Ok(OptionalAuthUser(Some(AuthUser {
            user_id: claims.user_id,
            username: claims.sub,
            is_admin: claims.is_admin,
        })))
    }
}

pub fn create_jwt(username: &str, user_id: i64, is_admin: bool, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: username.to_string(),
        user_id,
        is_admin,
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub success: bool,
    pub message: String,
    pub user: Option<UserInfo>,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
}
