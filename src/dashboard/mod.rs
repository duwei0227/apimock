pub mod routes;
pub mod static_files;
pub mod ws;

use std::net::UdpSocket;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;
use tower_http::cors::CorsLayer;

use crate::error::Result;
use crate::AppState;

use routes::{admin_tenants, admin_users, auth as auth_routes, info, logs, mocks, ports};

pub async fn run(state: AppState, mgmt_port: u16) -> Result<()> {
    let app = build_router(state);
    let addr = format!("0.0.0.0:{}", mgmt_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    let display_ip = local_ip().unwrap_or_else(|| "127.0.0.1".to_owned());
    tracing::info!("Dashboard listening on http://{}:{}", display_ip, mgmt_port);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
        .await?;

    Ok(())
}

fn local_ip() -> Option<String> {
    // Connect a UDP socket to a public address without sending data — the OS
    // picks the outbound interface and we read back the local address.
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

fn build_router(state: AppState) -> Router {
    let auth_api = Router::new()
        .route("/login", post(auth_routes::login))
        .route("/logout", post(auth_routes::logout))
        .route("/switch-tenant", post(auth_routes::switch_tenant))
        .route("/me", get(auth_routes::me))
        .route("/me/default-tenant", put(auth_routes::set_default_tenant));

    let admin_api = Router::new()
        .route("/tenants", get(admin_tenants::list_tenants).post(admin_tenants::create_tenant))
        .route(
            "/tenants/:id",
            get(admin_tenants::get_tenant)
                .put(admin_tenants::update_tenant)
                .delete(admin_tenants::delete_tenant),
        )
        .route("/users", get(admin_users::list_users).post(admin_users::create_user))
        .route(
            "/users/:id",
            get(admin_users::get_user)
                .put(admin_users::update_user)
                .delete(admin_users::delete_user),
        )
        .route("/users/:id/disable", post(admin_users::disable_user))
        .route("/users/:id/enable", post(admin_users::enable_user))
        .route("/users/:id/reset-password", post(admin_users::reset_password))
        .route(
            "/users/:id/tenants",
            get(admin_users::list_user_tenants).post(admin_users::assign_tenant),
        )
        .route("/users/:id/tenants/:tenant_id", delete(admin_users::remove_tenant));

    let api = Router::new()
        .route("/info", get(info::get_info))
        .route("/ports", get(ports::list_ports).post(ports::create_port))
        .route(
            "/ports/:id",
            get(ports::get_port)
                .put(ports::update_port)
                .delete(ports::delete_port),
        )
        .route("/ports/:id/start", post(ports::start_port))
        .route("/ports/:id/stop", post(ports::stop_port))
        .route("/ports/:id/restart", post(ports::restart_port))
        .route("/ports/:id/status", get(ports::port_status))
        .route("/mocks", get(mocks::list_mocks).post(mocks::create_mock))
        .route(
            "/mocks/:id",
            get(mocks::get_mock)
                .put(mocks::update_mock)
                .delete(mocks::delete_mock),
        )
        .route("/mocks/:id/enabled", patch(mocks::set_mock_enabled))
        .route("/mocks/:id/test", post(mocks::test_mock))
        .route(
            "/logs/requests",
            get(logs::list_request_logs).delete(logs::clear_request_logs),
        )
        .route("/logs/requests/:id", get(logs::get_request_log))
        .route(
            "/logs/system",
            get(logs::list_system_logs).delete(logs::clear_system_logs),
        )
        .nest("/auth", auth_api)
        .nest("/admin", admin_api);

    Router::new()
        .nest("/api/v1", api)
        .route("/ws/logs", get(ws::ws_logs))
        .fallback(static_files::static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
