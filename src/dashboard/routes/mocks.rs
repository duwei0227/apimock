use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use std::collections::HashMap;

use crate::auth::AuthUser;
use crate::models::{HttpMethod, LogEvent, StateResource, TestMockPayload, TestMockResult};
use crate::traits::{CreateMockRequest, UpdateMockRequest};
use crate::AppState;

#[derive(Deserialize)]
pub struct ListMocksQuery {
    pub port_id: Option<i64>,
    pub tenant_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct CreateMockBody {
    pub port_id: i64,
    pub tenant_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub method: HttpMethod,
    pub path: String,
    pub request_schema: Option<serde_json::Value>,
    pub response_status: Option<u16>,
    pub response_headers: Option<HashMap<String, String>>,
    pub response_body: Option<String>,
    pub request_params: Option<HashMap<String, String>>,
    pub response_delay_ms: Option<u64>,
    pub pagination_enabled: Option<bool>,
    pub pagination_page_size: Option<u32>,
    pub pagination_page_param: Option<String>,
    pub pagination_size_param: Option<String>,
    pub pagination_data_field: Option<String>,
    pub pagination_total_field: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMockBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub method: Option<HttpMethod>,
    pub path: Option<String>,
    pub request_schema: Option<Option<serde_json::Value>>,
    pub response_status: Option<u16>,
    pub response_headers: Option<HashMap<String, String>>,
    pub response_body: Option<String>,
    pub response_delay_ms: Option<u64>,
    pub request_params: Option<HashMap<String, String>>,
    pub enabled: Option<bool>,
    pub pagination_enabled: Option<bool>,
    pub pagination_page_size: Option<u32>,
    pub pagination_page_param: Option<String>,
    pub pagination_size_param: Option<String>,
    pub pagination_data_field: Option<String>,
    pub pagination_total_field: Option<String>,
}

#[derive(Deserialize)]
pub struct SetEnabledBody {
    pub enabled: bool,
}

pub async fn list_mocks(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(q): Query<ListMocksQuery>,
) -> impl IntoResponse {
    let tenant_id = if claims.is_admin { q.tenant_id } else { claims.current_tenant_id };
    match state.mock_store.list_mocks(q.port_id, tenant_id).await {
        Ok(mocks) => Json(mocks).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_mock(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateMockBody>,
) -> impl IntoResponse {
    let tenant_id = body.tenant_id
        .or(claims.current_tenant_id)
        .ok_or(())
        .unwrap_or_else(|_| 0);
    if tenant_id == 0 {
        return (StatusCode::BAD_REQUEST, "no active tenant").into_response();
    }
    let req = CreateMockRequest {
        port_id: body.port_id,
        tenant_id,
        name: body.name,
        description: body.description.unwrap_or_default(),
        method: body.method,
        path: body.path,
        request_schema: body.request_schema,
        response_status: body.response_status.unwrap_or(200),
        response_headers: body.response_headers.unwrap_or_default(),
        response_body: body.response_body.unwrap_or_default(),
        request_params: body.request_params.unwrap_or_default(),
        response_delay_ms: body.response_delay_ms.unwrap_or(0),
        pagination_enabled: body.pagination_enabled.unwrap_or(false),
        pagination_page_size: body.pagination_page_size.unwrap_or(10),
        pagination_page_param: body.pagination_page_param.unwrap_or_else(|| "page".into()),
        pagination_size_param: body.pagination_size_param.unwrap_or_else(|| "page_size".into()),
        pagination_data_field: body.pagination_data_field.unwrap_or_default(),
        pagination_total_field: body.pagination_total_field.unwrap_or_default(),
    };
    match state.mock_store.create_mock(req).await {
        Ok(m) => {
            let _ = state.port_manager.restart_port(m.port_id).await;
            let _ = state.log_tx.send(LogEvent::StateChanged { resource: StateResource::Mocks });
            (StatusCode::CREATED, Json(m)).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn get_mock(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.mock_store.get_mock(id).await {
        Ok(Some(m)) => Json(m).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn update_mock(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateMockBody>,
) -> impl IntoResponse {
    let req = UpdateMockRequest {
        name: body.name,
        description: body.description,
        method: body.method,
        path: body.path,
        request_schema: body.request_schema,
        response_status: body.response_status,
        response_headers: body.response_headers,
        response_body: body.response_body,
        response_delay_ms: body.response_delay_ms,
        request_params: body.request_params,
        enabled: body.enabled,
        pagination_enabled: body.pagination_enabled,
        pagination_page_size: body.pagination_page_size,
        pagination_page_param: body.pagination_page_param,
        pagination_size_param: body.pagination_size_param,
        pagination_data_field: body.pagination_data_field,
        pagination_total_field: body.pagination_total_field,
    };
    match state.mock_store.update_mock(id, req).await {
        Ok(m) => {
            let _ = state.port_manager.restart_port(m.port_id).await;
            let _ = state.log_tx.send(LogEvent::StateChanged { resource: StateResource::Mocks });
            Json(m).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

pub async fn delete_mock(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    // Fetch port_id before deletion so we can restart the server.
    let port_id = state
        .mock_store
        .get_mock(id)
        .await
        .ok()
        .flatten()
        .map(|m| m.port_id);

    match state.mock_store.delete_mock(id).await {
        Ok(()) => {
            if let Some(pid) = port_id {
                let _ = state.port_manager.restart_port(pid).await;
            }
            let _ = state.log_tx.send(LogEvent::StateChanged { resource: StateResource::Mocks });
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn set_mock_enabled(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<SetEnabledBody>,
) -> impl IntoResponse {
    let port_id = state
        .mock_store
        .get_mock(id)
        .await
        .ok()
        .flatten()
        .map(|m| m.port_id);

    match state.mock_store.set_mock_enabled(id, body.enabled).await {
        Ok(()) => {
            if let Some(pid) = port_id {
                let _ = state.port_manager.restart_port(pid).await;
            }
            let _ = state.log_tx.send(LogEvent::StateChanged { resource: StateResource::Mocks });
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn test_mock(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<TestMockPayload>,
) -> impl IntoResponse {
    // 1. Look up mock
    let mock = match state.mock_store.get_mock(id).await {
        Ok(Some(m)) => m,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // 2. Look up port number
    let port_cfg = match state.port_store.get_port(mock.port_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::BAD_GATEWAY, "port not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // 3. Look up tenant slug
    let slug = match state.tenant_store.get_tenant(mock.tenant_id).await {
        Ok(Some(t)) => t.slug,
        Ok(None) => return (StatusCode::BAD_GATEWAY, "tenant not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // 4. Build target URL.
    //    "default" tenant is served without a slug prefix: ip:port/path
    let path = mock.path.trim_start_matches('/');
    let url = if slug == "default" {
        format!("http://127.0.0.1:{}/{}", port_cfg.port, path)
    } else {
        format!("http://127.0.0.1:{}/{}/{}", port_cfg.port, slug, path)
    };

    // 5. Map HttpMethod → reqwest::Method (ANY defaults to GET)
    let method = match mock.method {
        HttpMethod::GET     => reqwest::Method::GET,
        HttpMethod::POST    => reqwest::Method::POST,
        HttpMethod::PUT     => reqwest::Method::PUT,
        HttpMethod::PATCH   => reqwest::Method::PATCH,
        HttpMethod::DELETE  => reqwest::Method::DELETE,
        HttpMethod::HEAD    => reqwest::Method::HEAD,
        HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
        HttpMethod::ANY     => reqwest::Method::GET,
    };

    let client = reqwest::Client::new();
    let mut req = client.request(method, &url);

    // Query params
    if !payload.query_params.is_empty() {
        req = req.query(&payload.query_params);
    }

    // Request headers
    for (k, v) in &payload.headers {
        if let (Ok(name), Ok(val)) = (
            reqwest::header::HeaderName::from_bytes(k.as_bytes()),
            reqwest::header::HeaderValue::from_str(v),
        ) {
            req = req.header(name, val);
        }
    }

    // Body
    if let Some(body) = payload.body {
        req = req.body(body);
    }

    // 6. Fire and collect response
    let start = std::time::Instant::now();
    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => return (StatusCode::BAD_GATEWAY, e.to_string()).into_response(),
    };
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let status = resp.status().as_u16();
    let headers: HashMap<String, String> = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_owned()))
        .collect();
    let body = resp.text().await.unwrap_or_default();

    Json(TestMockResult { status, headers, body, elapsed_ms }).into_response()
}
