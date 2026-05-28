use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::{encode_token, make_claims, verify_password, AuthUser};
use crate::models::{Tenant, User, UserTenant};
use crate::AppState;

#[derive(Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: User,
    pub tenants: Vec<Tenant>,
    pub user_tenants: Vec<UserTenant>,
    pub current_tenant: Option<Tenant>,
}

#[derive(Deserialize)]
pub struct SwitchTenantBody {
    pub tenant_id: i64,
}

#[derive(Serialize)]
pub struct MeResponse {
    pub user: User,
    pub tenants: Vec<Tenant>,
    pub user_tenants: Vec<UserTenant>,
    pub current_tenant: Option<Tenant>,
}

#[derive(Deserialize)]
pub struct SetDefaultTenantBody {
    pub tenant_id: i64,
}

pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginBody>,
) -> impl IntoResponse {
    let result = state.user_store.get_user_by_username(&body.username).await;
    let (user, hash) = match result {
        Ok(Some(v)) => v,
        _ => return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response(),
    };
    if !user.enabled {
        return (StatusCode::UNAUTHORIZED, "account disabled").into_response();
    }
    if !verify_password(&body.password, &hash) {
        return (StatusCode::UNAUTHORIZED, "invalid credentials").into_response();
    }

    let user_tenants = state
        .user_store
        .list_user_tenants(user.id)
        .await
        .unwrap_or_default();
    let tenant_ids: Vec<i64> = user_tenants.iter().map(|ut| ut.tenant_id).collect();
    let all_tenants = state.tenant_store.list_tenants().await.unwrap_or_default();

    let tenants: Vec<Tenant> = if user.is_admin {
        all_tenants.clone()
    } else {
        all_tenants
            .into_iter()
            .filter(|t| tenant_ids.contains(&t.id))
            .collect()
    };

    let default_tenant_id = user_tenants
        .iter()
        .find(|ut| ut.is_default)
        .map(|ut| ut.tenant_id)
        .or_else(|| tenant_ids.first().copied());
    let current_tenant = tenants
        .iter()
        .find(|t| Some(t.id) == default_tenant_id)
        .cloned();

    if !user.is_admin && tenants.is_empty() {
        return (
            StatusCode::FORBIDDEN,
            "no tenant assigned; contact your administrator",
        )
            .into_response();
    }

    let claims = make_claims(
        user.id,
        &user.username,
        &user.display_name,
        user.is_admin,
        current_tenant.as_ref().map(|t| t.id),
    );
    let token = match encode_token(&claims, &state.jwt_secret) {
        Ok(t) => t,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    Json(LoginResponse {
        token,
        user,
        tenants,
        user_tenants,
        current_tenant,
    })
    .into_response()
}

pub async fn logout() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub async fn switch_tenant(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<SwitchTenantBody>,
) -> impl IntoResponse {
    if !claims.is_admin {
        match state
            .user_store
            .user_has_tenant(claims.user_id, body.tenant_id)
            .await
        {
            Ok(true) => {}
            _ => return (StatusCode::FORBIDDEN, "not a member of this tenant").into_response(),
        }
    }
    let new_claims = make_claims(
        claims.user_id,
        &claims.username,
        &claims.display_name,
        claims.is_admin,
        Some(body.tenant_id),
    );
    match encode_token(&new_claims, &state.jwt_secret) {
        Ok(token) => Json(serde_json::json!({ "token": token })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn me(State(state): State<AppState>, AuthUser(claims): AuthUser) -> impl IntoResponse {
    let user = match state.user_store.get_user(claims.user_id).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let user_tenants = state
        .user_store
        .list_user_tenants(user.id)
        .await
        .unwrap_or_default();
    let tenant_ids: Vec<i64> = user_tenants.iter().map(|ut| ut.tenant_id).collect();
    let all_tenants = state.tenant_store.list_tenants().await.unwrap_or_default();
    let tenants: Vec<Tenant> = if user.is_admin {
        all_tenants.clone()
    } else {
        all_tenants
            .into_iter()
            .filter(|t| tenant_ids.contains(&t.id))
            .collect()
    };
    let current_tenant = claims
        .current_tenant_id
        .and_then(|id| tenants.iter().find(|t| t.id == id).cloned());
    Json(MeResponse {
        user,
        tenants,
        user_tenants,
        current_tenant,
    })
    .into_response()
}

pub async fn set_default_tenant(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<SetDefaultTenantBody>,
) -> impl IntoResponse {
    match state
        .user_store
        .set_default_tenant(claims.user_id, body.tenant_id)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
