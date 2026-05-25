use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::auth::{hash_password, AdminUser};
use crate::traits::{CreateUserRequest, UpdateUserRequest};
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateUserBody {
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub is_admin: bool,
}

#[derive(Deserialize)]
pub struct UpdateUserBody {
    pub display_name: String,
    pub is_admin: bool,
}

#[derive(Deserialize)]
pub struct ResetPasswordBody {
    pub password: String,
}

#[derive(Deserialize)]
pub struct AssignTenantBody {
    pub tenant_id: i64,
}

pub async fn list_users(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
) -> impl IntoResponse {
    match state.user_store.list_users().await {
        Ok(u) => Json(u).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_user(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Json(body): Json<CreateUserBody>,
) -> impl IntoResponse {
    let hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let req = CreateUserRequest {
        username: body.username,
        display_name: body.display_name,
        password_hash: hash,
        is_admin: body.is_admin,
    };
    match state.user_store.create_user(req).await {
        Ok(u) => (StatusCode::CREATED, Json(u)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn get_user(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.user_store.get_user(id).await {
        Ok(Some(u)) => Json(u).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_user(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUserBody>,
) -> impl IntoResponse {
    let req = UpdateUserRequest { display_name: body.display_name, is_admin: body.is_admin };
    match state.user_store.update_user(id, req).await {
        Ok(u) => Json(u).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn delete_user(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.user_store.delete_user(id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn disable_user(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.user_store.set_user_enabled(id, false).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn enable_user(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.user_store.set_user_enabled(id, true).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn reset_password(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
    Json(body): Json<ResetPasswordBody>,
) -> impl IntoResponse {
    let hash = match hash_password(&body.password) {
        Ok(h) => h,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    match state.user_store.set_password_hash(id, &hash).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn assign_tenant(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(user_id): Path<i64>,
    Json(body): Json<AssignTenantBody>,
) -> impl IntoResponse {
    match state.user_store.assign_tenant(user_id, body.tenant_id).await {
        Ok(ut) => (StatusCode::CREATED, Json(ut)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn remove_tenant(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path((user_id, tenant_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    match state.user_store.remove_tenant(user_id, tenant_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn list_user_tenants(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(user_id): Path<i64>,
) -> impl IntoResponse {
    match state.user_store.list_user_tenants(user_id).await {
        Ok(uts) => Json(uts).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
