# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build (also triggers frontend build via build.rs if frontend/dist/ is missing)
cargo build

# Run (TUI mode, default)
cargo run

# Run (foreground server: ports + web dashboard at http://localhost:9999, no TUI)
cargo run -- serve

# Check without producing a binary
cargo check

# Lint
cargo clippy

# Frontend dev server (proxy to backend at localhost:9999)
cd frontend && npm run dev

# Frontend production build (also triggered automatically by cargo build)
cd frontend && npm run build
```

There are no automated tests currently in this project.

## Architecture

This is a **mock API server** — a developer tool for defining HTTP endpoints that return canned responses. It has a terminal UI (TUI) and a web dashboard that share all backend infrastructure. The web dashboard is always served at `http://localhost:9999` when the server is running; there is no separate flag to enable it.

### Data flow

1. **SQLite** (`tokio-rusqlite` with bundled SQLite) is the single source of truth. Schema migrations live in `src/db/schema.rs` as versioned `(name, SQL)` pairs run at startup.
2. **Three store traits** (`MockStore`, `PortStore`, `LogStore` in `src/traits/mod.rs`) abstract all DB access. Concrete implementations live in `src/db/`.
3. **`LivePortManager`** (`src/server/manager.rs`) manages a set of tokio tasks — one per enabled port. Each task runs an independent Axum server. When a port is started it snapshots the enabled mocks into `MockHandlerState`; a restart is required for config changes to take effect.
4. **Mock handler** (`src/server/handler.rs`) matches incoming requests against the snapshot using simple glob matching (`*` per segment). Exact method match beats `ANY`. Logging is fire-and-forget via `tokio::spawn`.
5. **Real-time events** flow through a `broadcast::Sender<LogEvent>` that both the tracing subscriber (system logs) and the request handler (request logs) write to. The dashboard WebSocket endpoint (`src/dashboard/ws.rs`) broadcasts these to connected browser clients.

### Two UIs, one `AppState`

`AppState` (defined in `src/main.rs`) carries all stores, the port manager, and the broadcast sender. It is `Clone` and threaded into both subsystems:

- **TUI** (`src/tui/`) — `ratatui` + `crossterm` terminal UI with tabs for Ports, Mocks, and Logs. The `App` struct in `src/tui/app.rs` holds all UI state. Modals handle creation/editing.
- **Web dashboard** (`src/dashboard/`) — Axum router under `/api/v1` serving REST endpoints for ports, mocks, logs, auth, and admin. A WebSocket at `/ws/logs` streams real-time events. Static assets (the compiled Vue app) are embedded into the binary via `rust-embed`.

### Auth & multi-tenancy

- **JWT auth** (`src/auth/`) — login issues a short-lived JWT. All dashboard API routes are protected by the `AuthUser` extractor. `AuthUser` carries the user's claims including `is_admin` and `current_tenant_id`.
- **Tenants** — each mock is scoped to a tenant. The mock server routes by reading the tenant slug from the first URL segment (`/{tenant}/{path}`). The `default` tenant is served without a prefix (`/{path}`). Non-admin users only see mocks for their assigned tenants.

### Frontend

Vue 3 + Vite + PrimeVue + Pinia + Tailwind CSS. Stores in `frontend/src/stores/` mirror the three backend resource types. The `build.rs` script runs `npm install && npm run build` automatically when `frontend/dist/` does not exist, embedding the output into the binary.

### Key design constraints

- Each mock server port holds a **snapshot** of mocks at start time — changes to mocks require `restart_port` to take effect on running servers.
- Path matching is glob-style (`*` per segment, trailing `*` matches remaining segments), not regex.
- The `(port_id, method, path)` combination is UNIQUE in the DB — duplicate mock definitions are rejected at the database level.
- Logs are stored in SQLite but also broadcast in-memory; WebSocket clients only see events that occur after they connect.
