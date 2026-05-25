# Multi-Tenant Design — apimock Dashboard

**Date:** 2026-05-21  
**Scope:** Dashboard mode only (`--dashboard` flag). TUI mode unchanged.

---

## Context

apimock currently has no authentication or data isolation. This design adds a multi-tenant layer:
- A single admin account manages the system (tenants, users, ports)
- Regular users log in and operate within one or more tenants
- Each tenant's mock data is fully isolated; the running mock server routes by tenant slug in the URL

---

## 1. Data Model

### New Tables

```sql
-- tenants: global tenant registry
CREATE TABLE tenants (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL UNIQUE,
    slug       TEXT    NOT NULL UNIQUE,   -- URL-safe identifier used in mock routing
    enabled    INTEGER NOT NULL DEFAULT 1,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- users: global user registry
CREATE TABLE users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT    NOT NULL UNIQUE,
    display_name  TEXT    NOT NULL DEFAULT '',  -- 显示名称，用于 UI 展示
    password_hash TEXT    NOT NULL,             -- bcrypt
    is_admin      INTEGER NOT NULL DEFAULT 0,
    enabled       INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

-- user_tenants: N:M — a user can belong to multiple tenants
CREATE TABLE user_tenants (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tenant_id  INTEGER NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    is_default INTEGER NOT NULL DEFAULT 0,
    created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    UNIQUE(user_id, tenant_id)
);
```

### Modified Tables

**`mock_apis`** — add tenant scope:
```sql
ALTER TABLE mock_apis ADD COLUMN tenant_id INTEGER NOT NULL REFERENCES tenants(id);
-- Drop old unique constraint (port_id, method, path)
-- Add new unique constraint: UNIQUE(port_id, tenant_id, method, path)
```

**`request_logs`** — add tenant scope:
```sql
ALTER TABLE request_logs ADD COLUMN tenant_id INTEGER REFERENCES tenants(id);
```

**`port_configs`** — **no change**. Ports are global shared resources.

### Seed Data

On first startup, if `users` table is empty, insert:
```
username: admin, password: Admin@123 (bcrypt hashed), is_admin: 1
```

---

## 2. Authentication

**Mechanism:** JWT (stateless, no server-side session storage).

**JWT Payload:**
```json
{ "user_id": 1, "username": "admin", "display_name": "Administrator", "is_admin": true, "current_tenant_id": 2, "exp": 1234567890 }
```

**Token lifetime:** 8 hours (configurable via env var or CLI flag).

**Tenant context:** Embedded in the token. Switching tenants issues a new token. The old token remains valid until expiry — acceptable for this use case.

**Key endpoints:**
```
POST /api/v1/auth/login              → { token, user, tenants[] }
POST /api/v1/auth/logout             → 204 (client discards token)
POST /api/v1/auth/switch-tenant      → { token } (new token with updated tenant_id)
GET  /api/v1/auth/me                 → { user, current_tenant, tenants[] }
PUT  /api/v1/auth/me/default-tenant  → set default tenant
```

**Middleware:** All `/api/v1/*` routes (except `/auth/login`) require `Authorization: Bearer <token>`. Admin-only routes additionally check `is_admin: true`.

---

## 3. Mock Request Routing

**URL format:**
```
http://<ip>:<port>/<tenant_slug>/<mock_path>
```

Example: `http://192.168.1.10:3000/acme/api/users`

**Handler changes (`src/server/handler.rs`):**

1. Port starts → loads **all tenants' mocks** for this port (from `mock_apis` joined with `tenants`)
2. `MockHandlerState.mocks` snapshot includes `tenant_id` and `tenant_slug` per mock
3. On request: extract first path segment as `tenant_slug`
4. Match against `(port_id, tenant_id, method, remaining_path)`
5. Log records include `tenant_id`

**Mock address displayed in UI:**
```
http://<host>:<port>/<tenant_slug>/<user_defined_path>
```

**Unique constraint change:**  
`(port_id, method, path)` → `(port_id, tenant_id, method, path)`  
Different tenants may define the same method+path on the same port; the slug prefix disambiguates at runtime.

---

## 4. Permission Matrix

| Resource | Admin | Regular User |
|---|---|---|
| **Ports** — list | ✓ | ✓ (read-only) |
| **Ports** — create/edit/delete/start/stop | ✓ | ✗ (UI shows "contact admin") |
| **Ports** — delete usage check | checks all tenants for mocks | N/A |
| **Mocks** | all tenants | own tenant only (CRUD) |
| **Logs** | all tenants | own tenant only |
| **Tenants** — CRUD | ✓ | ✗ (not visible) |
| **Users** — CRUD, disable, reset password | ✓ | ✗ |
| **Password change** | admin resets others' passwords | ✗ (no self-service; accounts may be shared) |
| **Tenant assignment** | admin assigns users ↔ tenants | ✗ |
| **Switch tenant** | ✓ | ✓ (among own tenants) |
| **Set default tenant** | ✓ | ✓ |

---

## 5. New API Routes

### Auth (public)
```
POST /api/v1/auth/login
POST /api/v1/auth/logout
POST /api/v1/auth/switch-tenant
GET  /api/v1/auth/me
PUT  /api/v1/auth/me/default-tenant
```

### Admin-only (`/api/v1/admin/*`)
```
GET    /api/v1/admin/tenants
POST   /api/v1/admin/tenants
GET    /api/v1/admin/tenants/:id
PUT    /api/v1/admin/tenants/:id
DELETE /api/v1/admin/tenants/:id      -- rejects if any mock references this tenant

GET    /api/v1/admin/users
POST   /api/v1/admin/users
GET    /api/v1/admin/users/:id
PUT    /api/v1/admin/users/:id
DELETE /api/v1/admin/users/:id
POST   /api/v1/admin/users/:id/disable
POST   /api/v1/admin/users/:id/enable
POST   /api/v1/admin/users/:id/reset-password
POST   /api/v1/admin/users/:id/tenants            -- assign tenant
DELETE /api/v1/admin/users/:id/tenants/:tenant_id -- remove assignment
```

### Modified Existing Routes

All existing routes now require Bearer auth. Behavior changes:

| Route | Change |
|---|---|
| `GET /api/v1/ports` | No filter; both admin and user see all ports |
| `POST/PUT/DELETE /api/v1/ports*` | Admin-only (401 for regular users) |
| `GET /api/v1/mocks` | Filtered by `current_tenant_id` from JWT |
| `POST/PUT/DELETE /api/v1/mocks*` | Filtered by `current_tenant_id` from JWT |
| `GET /api/v1/logs/*` | Filtered by `current_tenant_id` (admin sees all) |
| `DELETE /api/v1/logs/*` | Admin: all tenants; user: own tenant |

---

## 6. Frontend Changes

### New Views / Components
- **LoginView** — username/password form, no sidebar; redirects to dashboard after login
- **AdminView** — sidebar entry visible only to admin:
  - Tenant management tab (list, create, edit, delete)
  - User management tab (list, create, edit, disable, reset password, assign tenants)
- **TenantSwitcher** — dropdown in `AppTopBar`; shows current tenant, lists user's tenants, "switch" action

### Modified Views
- **AppTopBar** — add TenantSwitcher + current user display
- **PortsView** — read-only for regular users; show alert banner: "需要开启新端口？请联系管理员"
- **MocksView** — mock address column shows full URL `host:port/slug/path`; all API calls scoped to current tenant
- **LogsView** — scoped to current tenant

### Auth Flow (Frontend)
1. Unauthenticated → redirect to `/login`
2. Login → store JWT in `localStorage`, parse payload to get `is_admin` and `current_tenant_id`
3. All axios requests attach `Authorization: Bearer <token>` header
4. 401 response → clear token, redirect to `/login`
5. Switch tenant → call switch-tenant API → replace token in localStorage → reload stores

### Router Guards
- All routes except `/login` require a valid token
- Admin routes (`/admin/*`) additionally require `is_admin: true`

---

## 7. Backend Rust Changes

### New modules
- `src/auth/` — JWT encode/decode, middleware extractor (`AuthUser` struct), password hashing (bcrypt via `bcrypt` crate)
- `src/db/user_store.rs` — implements `UserStore` trait (users + user_tenants)
- `src/db/tenant_store.rs` — implements `TenantStore` trait
- `src/dashboard/routes/auth.rs` — login/logout/switch/me handlers
- `src/dashboard/routes/admin.rs` — tenant + user management handlers

### AppState additions
```rust
pub struct AppState {
    // existing fields ...
    pub user_store: Arc<dyn UserStore>,
    pub tenant_store: Arc<dyn TenantStore>,
    pub jwt_secret: String,   // generated or from config at startup
}
```

### MockHandlerState change
```rust
pub struct MockHandlerState {
    pub port: u16,
    pub mocks: Arc<Vec<MockWithTenant>>,  // includes tenant_slug per mock
    pub log_store: Arc<dyn LogStore>,
    pub log_tx: broadcast::Sender<LogEvent>,
}
```

### New crate dependencies
- `jsonwebtoken` — JWT encode/decode
- `bcrypt` — password hashing

---

## 8. Verification

1. **Start server**: `cargo run -- --dashboard`
2. **Login**: `POST /api/v1/auth/login` with `admin/Admin@123` → get JWT
3. **Create tenant + user**: via admin API, assign user to tenant
4. **Login as user**: verify only own tenant's mocks and logs are visible
5. **Switch tenant**: verify data changes to second tenant's context
6. **Port read-only**: regular user `GET /api/v1/ports` succeeds; `POST /api/v1/ports` returns 403
7. **Mock routing**: create mock `GET /api/users` for tenant slug `acme`; start port; `curl http://localhost:3000/acme/api/users` returns mock response
8. **Cross-tenant isolation**: create same path mock for tenant `beta`; `curl .../beta/api/users` returns beta's response independently
9. **Delete port with mocks**: admin tries to delete port with active mocks → rejected with error message
