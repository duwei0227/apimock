use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::AppState;

pub const TOKEN_TTL_SECS: u64 = 8 * 3600;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    pub display_name: String,
    pub is_admin: bool,
    pub current_tenant_id: Option<i64>,
    pub exp: u64,
}

/// Authenticated user extracted from Bearer JWT. Rejects with 401 if missing/invalid.
#[derive(Debug, Clone)]
pub struct AuthUser(pub Claims);

/// Admin-only extractor. Rejects with 403 if not admin.
#[derive(Debug, Clone)]
pub struct AdminUser(pub Claims);

pub fn encode_token(claims: &Claims, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn decode_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

pub fn make_claims(
    user_id: i64,
    username: &str,
    display_name: &str,
    is_admin: bool,
    current_tenant_id: Option<i64>,
) -> Claims {
    let exp = (Utc::now().timestamp() as u64) + TOKEN_TTL_SECS;
    Claims {
        user_id,
        username: username.to_owned(),
        display_name: display_name.to_owned(),
        is_admin,
        current_tenant_id,
        exp,
    }
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        use axum::extract::FromRef;
        let app_state = AppState::from_ref(state);
        let token_opt = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_owned());

        Box::pin(async move {
            let token = token_opt.ok_or((StatusCode::UNAUTHORIZED, "missing token"))?;
            let claims = decode_token(&token, &app_state.jwt_secret)
                .map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token"))?;
            Ok(AuthUser(claims))
        })
    }
}

impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
            if !claims.is_admin {
                return Err((StatusCode::FORBIDDEN, "admin required"));
            }
            Ok(AdminUser(claims))
        })
    }
}
