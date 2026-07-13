use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::auth::AdminUser;
use crate::traits::{CreateTenantRequest, UpdateTenantRequest};
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateTenantBody {
    pub name: String,
    pub slug: String,
}

#[derive(Deserialize)]
pub struct UpdateTenantBody {
    pub name: String,
    pub slug: String,
    pub enabled: bool,
}

pub async fn list_tenants(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
) -> impl IntoResponse {
    match state.tenant_store.list_tenants().await {
        Ok(t) => Json(t).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_tenant(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Json(body): Json<CreateTenantBody>,
) -> impl IntoResponse {
    let req = CreateTenantRequest {
        name: body.name,
        slug: body.slug,
    };
    match state.tenant_store.create_tenant(req).await {
        Ok(t) => (StatusCode::CREATED, Json(t)).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn get_tenant(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.tenant_store.get_tenant(id).await {
        Ok(Some(t)) => Json(t).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_tenant(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateTenantBody>,
) -> impl IntoResponse {
    let req = UpdateTenantRequest {
        name: body.name,
        slug: body.slug,
        enabled: body.enabled,
    };
    match state.tenant_store.update_tenant(id, req).await {
        Ok(t) => Json(t).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn delete_tenant(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.tenant_store.has_mocks(id).await {
        Ok(true) => {
            return (
                StatusCode::CONFLICT,
                "tenant has mock APIs; remove them before deleting the tenant",
            )
                .into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Ok(false) => {}
    }
    match state.tenant_store.delete_tenant(id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
