# Multi-Tenant Dashboard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add JWT-based auth, tenant isolation, user management, and per-tenant mock routing to the apimock dashboard.

**Architecture:** JWT tokens carry `current_tenant_id`; all existing routes gain auth middleware; mock server routes requests via `/{tenant_slug}/path` URL prefix; single SQLite DB gains `tenants`, `users`, `user_tenants` tables plus `tenant_id` columns on `mock_apis` and `request_logs`.

**Tech Stack:** Rust/Axum 0.7, `jsonwebtoken 9`, `bcrypt 0.15`, Vue 3 + Pinia + PrimeVue, existing `tokio-rusqlite` + `rand 0.8`.

**Spec:** `docs/superpowers/specs/2026-05-21-multi-tenant-design.md`

---

## Task 1: Add Cargo dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Add jsonwebtoken and bcrypt**

```toml
# in [dependencies]
jsonwebtoken = "9"
bcrypt       = "0.15"
```

- [ ] **Verify compilation**

```bash
cargo check
```

Expected: compiles with no errors (no code uses the new crates yet).

- [ ] **Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add jsonwebtoken and bcrypt dependencies"
```

---

## Task 2: DB schema migrations 0012–0016

**Files:**
- Modify: `src/db/schema.rs`

- [ ] **Add five new migrations** — append to the `MIGRATIONS` slice before the closing `];`:

```rust
    ("0012_tenants", "
        CREATE TABLE IF NOT EXISTS tenants (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            name       TEXT    NOT NULL UNIQUE,
            slug       TEXT    NOT NULL UNIQUE,
            enabled    INTEGER NOT NULL DEFAULT 1,
            created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        INSERT OR IGNORE INTO tenants (name, slug) VALUES ('Default', 'default');
    "),
    ("0013_users", "
        CREATE TABLE IF NOT EXISTS users (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            username      TEXT    NOT NULL UNIQUE,
            display_name  TEXT    NOT NULL DEFAULT '',
            password_hash TEXT    NOT NULL,
            is_admin      INTEGER NOT NULL DEFAULT 0,
            enabled       INTEGER NOT NULL DEFAULT 1,
            created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
    "),
    ("0014_user_tenants", "
        CREATE TABLE IF NOT EXISTS user_tenants (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            tenant_id  INTEGER NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            is_default INTEGER NOT NULL DEFAULT 0,
            created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            UNIQUE(user_id, tenant_id)
        );
    "),
    ("0015_mock_apis_tenant", "
        CREATE TABLE mock_apis_new (
            id                     INTEGER PRIMARY KEY AUTOINCREMENT,
            port_id                INTEGER NOT NULL REFERENCES port_configs(id) ON DELETE CASCADE,
            tenant_id              INTEGER NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            name                   TEXT    NOT NULL,
            description            TEXT    NOT NULL DEFAULT '',
            method                 TEXT    NOT NULL DEFAULT 'ANY',
            path                   TEXT    NOT NULL,
            request_schema         TEXT,
            response_status        INTEGER NOT NULL DEFAULT 200,
            response_headers       TEXT    NOT NULL DEFAULT '{}',
            response_body          TEXT    NOT NULL DEFAULT '',
            response_delay_ms      INTEGER NOT NULL DEFAULT 0,
            enabled                INTEGER NOT NULL DEFAULT 1,
            created_at             TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            updated_at             TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            response_filter_enabled INTEGER NOT NULL DEFAULT 0,
            pagination_enabled     INTEGER NOT NULL DEFAULT 0,
            pagination_page_size   INTEGER NOT NULL DEFAULT 10,
            request_params         TEXT    NOT NULL DEFAULT '{}',
            pagination_page_param  TEXT    NOT NULL DEFAULT 'page',
            pagination_size_param  TEXT    NOT NULL DEFAULT 'page_size',
            pagination_data_field  TEXT    NOT NULL DEFAULT '',
            pagination_total_field TEXT    NOT NULL DEFAULT '',
            UNIQUE(port_id, tenant_id, method, path)
        );
        INSERT INTO mock_apis_new
            SELECT m.id, m.port_id,
                   (SELECT id FROM tenants WHERE slug='default' LIMIT 1),
                   m.name, m.description, m.method, m.path, m.request_schema,
                   m.response_status, m.response_headers, m.response_body,
                   m.response_delay_ms, m.enabled, m.created_at, m.updated_at,
                   m.response_filter_enabled, m.pagination_enabled,
                   m.pagination_page_size, m.request_params, m.pagination_page_param,
                   m.pagination_size_param, m.pagination_data_field, m.pagination_total_field
            FROM mock_apis m;
        DROP TABLE mock_apis;
        ALTER TABLE mock_apis_new RENAME TO mock_apis;
        CREATE INDEX IF NOT EXISTS idx_mock_apis_port_id   ON mock_apis(port_id);
        CREATE INDEX IF NOT EXISTS idx_mock_apis_tenant_id ON mock_apis(tenant_id);
    "),
    ("0016_request_logs_tenant", "
        ALTER TABLE request_logs ADD COLUMN tenant_id INTEGER REFERENCES tenants(id);
    "),
```

- [ ] **Verify migrations run**

```bash
cargo run -- --help   # triggers compile; no DB yet
rm -f /tmp/test_migrate.db
cargo run -- --db /tmp/test_migrate.db serve &
sleep 2 && kill %1
sqlite3 /tmp/test_migrate.db ".tables"
```

Expected output includes: `mock_apis  port_configs  request_logs  system_logs  tenants  user_tenants  users`

- [ ] **Commit**

```bash
git add src/db/schema.rs
git commit -m "feat: add tenants/users/user_tenants migrations, add tenant_id to mock_apis and request_logs"
```

---

## Task 3: Update models

**Files:**
- Modify: `src/models/mod.rs`

- [ ] **Add Tenant, User, UserTenant structs and tenant_id fields**

Add after the `HttpMethod` block and update `MockApi` and `RequestLog`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: i64,
    pub name: String,
    pub slug: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub is_admin: bool,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTenant {
    pub id: i64,
    pub user_id: i64,
    pub tenant_id: i64,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockWithTenant {
    #[serde(flatten)]
    pub mock: MockApi,
    pub tenant_slug: String,
}
```

In `MockApi`, add `tenant_id` field after `port_id`:

```rust
pub struct MockApi {
    pub id: i64,
    pub port_id: i64,
    pub tenant_id: i64,   // ← add this line
    pub name: String,
    // ... rest unchanged
```

In `RequestLog`, add `tenant_id` field after `client_ip`:

```rust
pub struct RequestLog {
    // ... existing fields ...
    pub client_ip: Option<String>,
    pub tenant_id: Option<i64>,   // ← add this line
    pub created_at: DateTime<Utc>,
}
```

- [ ] **Verify compilation**

```bash
cargo check 2>&1 | head -40
```

Expected: errors only about missing `tenant_id` in `row_to_mock` / `row_to_request_log` (fixed in Tasks 7 and 8).

- [ ] **Commit** (after Tasks 7 and 8 fix compile errors)

---

## Task 4: Auth module

**Files:**
- Create: `src/auth/mod.rs`

- [ ] **Create `src/auth/mod.rs`**

```rust
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};
use axum::RequestPartsExt;
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
    axum::extract::FromRef<S, AppState>: Sized,
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
    axum::extract::FromRef<S, AppState>: Sized,
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
```

- [ ] **Register auth module in `src/main.rs`**

Add `mod auth;` to the module declarations at the top of `src/main.rs`.

- [ ] **Verify compilation**

```bash
cargo check 2>&1 | grep "^error" | head -20
```

Expected: `FromRef` trait bound may need adjustment — see Task 9 for the `AppState: FromRef` impl.

---

## Task 5: TenantStore trait and implementation

**Files:**
- Modify: `src/traits/mod.rs`
- Create: `src/db/tenant_store.rs`

- [ ] **Add TenantStore to `src/traits/mod.rs`**

Add after the `PortManager` trait:

```rust
// ---------------------------------------------------------------------------
// TenantStore
// ---------------------------------------------------------------------------

use crate::models::{Tenant, User, UserTenant};

#[derive(Debug, Clone)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct UpdateTenantRequest {
    pub name: String,
    pub slug: String,
    pub enabled: bool,
}

#[async_trait]
pub trait TenantStore: Send + Sync {
    async fn list_tenants(&self) -> Result<Vec<Tenant>>;
    async fn get_tenant(&self, id: i64) -> Result<Option<Tenant>>;
    async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;
    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant>;
    async fn update_tenant(&self, id: i64, req: UpdateTenantRequest) -> Result<Tenant>;
    async fn delete_tenant(&self, id: i64) -> Result<()>;
    /// Returns true if any mock_api row references this tenant_id.
    async fn has_mocks(&self, tenant_id: i64) -> Result<bool>;
}
```

- [ ] **Create `src/db/tenant_store.rs`**

```rust
use async_trait::async_trait;
use chrono::DateTime;
use tokio_rusqlite::Connection;

use crate::error::{AppError, Result};
use crate::models::Tenant;
use crate::traits::{CreateTenantRequest, TenantStore, UpdateTenantRequest};

pub struct SqliteTenantStore {
    conn: Connection,
}

impl SqliteTenantStore {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}

fn row_to_tenant(row: &rusqlite::Row<'_>) -> rusqlite::Result<Tenant> {
    let created_at = row
        .get::<_, String>(4)
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_default();
    Ok(Tenant {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        created_at,
    })
}

#[async_trait]
impl TenantStore for SqliteTenantStore {
    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.conn
            .call(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, slug, enabled, created_at FROM tenants ORDER BY id",
                )?;
                let items = stmt
                    .query_map([], row_to_tenant)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(items)
            })
            .await
            .map_err(AppError::from)
    }

    async fn get_tenant(&self, id: i64) -> Result<Option<Tenant>> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, slug, enabled, created_at FROM tenants WHERE id = ?1",
                )?;
                let item = stmt
                    .query_map(rusqlite::params![id], row_to_tenant)?
                    .next()
                    .transpose()?;
                Ok(item)
            })
            .await
            .map_err(AppError::from)
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let slug = slug.to_owned();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, name, slug, enabled, created_at FROM tenants WHERE slug = ?1",
                )?;
                let item = stmt
                    .query_map(rusqlite::params![slug], row_to_tenant)?
                    .next()
                    .transpose()?;
                Ok(item)
            })
            .await
            .map_err(AppError::from)
    }

    async fn create_tenant(&self, req: CreateTenantRequest) -> Result<Tenant> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO tenants (name, slug) VALUES (?1, ?2)",
                    rusqlite::params![req.name, req.slug],
                )?;
                let id = conn.last_insert_rowid();
                let mut stmt = conn.prepare(
                    "SELECT id, name, slug, enabled, created_at FROM tenants WHERE id = ?1",
                )?;
                let item = stmt
                    .query_map(rusqlite::params![id], row_to_tenant)?
                    .next()
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)??;
                Ok(item)
            })
            .await
            .map_err(AppError::from)
    }

    async fn update_tenant(&self, id: i64, req: UpdateTenantRequest) -> Result<Tenant> {
        let enabled_i64 = req.enabled as i64;
        self.conn
            .call(move |conn| {
                conn.execute(
                    "UPDATE tenants SET name = ?1, slug = ?2, enabled = ?3 WHERE id = ?4",
                    rusqlite::params![req.name, req.slug, enabled_i64, id],
                )?;
                let mut stmt = conn.prepare(
                    "SELECT id, name, slug, enabled, created_at FROM tenants WHERE id = ?1",
                )?;
                let item = stmt
                    .query_map(rusqlite::params![id], row_to_tenant)?
                    .next()
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)??;
                Ok(item)
            })
            .await
            .map_err(AppError::from)
    }

    async fn delete_tenant(&self, id: i64) -> Result<()> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM tenants WHERE id = ?1",
                    rusqlite::params![id],
                )?;
                Ok(())
            })
            .await
            .map_err(AppError::from)
    }

    async fn has_mocks(&self, tenant_id: i64) -> Result<bool> {
        self.conn
            .call(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM mock_apis WHERE tenant_id = ?1",
                    rusqlite::params![tenant_id],
                    |row| row.get(0),
                )?;
                Ok(count > 0)
            })
            .await
            .map_err(AppError::from)
    }
}
```

---

## Task 6: UserStore trait and implementation

**Files:**
- Modify: `src/traits/mod.rs`
- Create: `src/db/user_store.rs`

- [ ] **Add UserStore to `src/traits/mod.rs`** (append after TenantStore):

```rust
// ---------------------------------------------------------------------------
// UserStore
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub is_admin: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateUserRequest {
    pub display_name: String,
    pub is_admin: bool,
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn list_users(&self) -> Result<Vec<User>>;
    async fn get_user(&self, id: i64) -> Result<Option<User>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>>;
    async fn create_user(&self, req: CreateUserRequest) -> Result<User>;
    async fn update_user(&self, id: i64, req: UpdateUserRequest) -> Result<User>;
    async fn delete_user(&self, id: i64) -> Result<()>;
    async fn set_user_enabled(&self, id: i64, enabled: bool) -> Result<()>;
    async fn set_password_hash(&self, id: i64, hash: &str) -> Result<()>;

    async fn list_user_tenants(&self, user_id: i64) -> Result<Vec<UserTenant>>;
    async fn assign_tenant(&self, user_id: i64, tenant_id: i64) -> Result<UserTenant>;
    async fn remove_tenant(&self, user_id: i64, tenant_id: i64) -> Result<()>;
    async fn set_default_tenant(&self, user_id: i64, tenant_id: i64) -> Result<()>;
    /// Returns true if the user belongs to the given tenant.
    async fn user_has_tenant(&self, user_id: i64, tenant_id: i64) -> Result<bool>;
    /// Check if DB has any users (for admin seed check).
    async fn is_empty(&self) -> Result<bool>;
}
```

- [ ] **Create `src/db/user_store.rs`**

```rust
use async_trait::async_trait;
use chrono::DateTime;
use tokio_rusqlite::Connection;

use crate::error::{AppError, Result};
use crate::models::{User, UserTenant};
use crate::traits::{CreateUserRequest, UpdateUserRequest, UserStore};

pub struct SqliteUserStore {
    conn: Connection,
}

impl SqliteUserStore {
    pub fn new(conn: Connection) -> Self {
        Self { conn }
    }
}

fn row_to_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    let created_at = row
        .get::<_, String>(5)
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_default();
    Ok(User {
        id: row.get(0)?,
        username: row.get(1)?,
        display_name: row.get(2)?,
        is_admin: row.get::<_, i64>(3)? != 0,
        enabled: row.get::<_, i64>(4)? != 0,
        created_at,
    })
}

fn row_to_user_tenant(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserTenant> {
    let created_at = row
        .get::<_, String>(4)
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_default();
    Ok(UserTenant {
        id: row.get(0)?,
        user_id: row.get(1)?,
        tenant_id: row.get(2)?,
        is_default: row.get::<_, i64>(3)? != 0,
        created_at,
    })
}

#[async_trait]
impl UserStore for SqliteUserStore {
    async fn list_users(&self) -> Result<Vec<User>> {
        self.conn
            .call(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, username, display_name, is_admin, enabled, created_at FROM users ORDER BY id",
                )?;
                let items = stmt
                    .query_map([], row_to_user)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(items)
            })
            .await
            .map_err(AppError::from)
    }

    async fn get_user(&self, id: i64) -> Result<Option<User>> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, username, display_name, is_admin, enabled, created_at FROM users WHERE id = ?1",
                )?;
                Ok(stmt
                    .query_map(rusqlite::params![id], row_to_user)?
                    .next()
                    .transpose()?)
            })
            .await
            .map_err(AppError::from)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
        let username = username.to_owned();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, username, display_name, is_admin, enabled, created_at, password_hash FROM users WHERE username = ?1",
                )?;
                let result = stmt
                    .query_map(rusqlite::params![username], |row| {
                        let user = row_to_user(row)?;
                        let hash: String = row.get(6)?;
                        Ok((user, hash))
                    })?
                    .next()
                    .transpose()?;
                Ok(result)
            })
            .await
            .map_err(AppError::from)
    }

    async fn create_user(&self, req: CreateUserRequest) -> Result<User> {
        let is_admin_i64 = req.is_admin as i64;
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT INTO users (username, display_name, password_hash, is_admin) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![req.username, req.display_name, req.password_hash, is_admin_i64],
                )?;
                let id = conn.last_insert_rowid();
                let mut stmt = conn.prepare(
                    "SELECT id, username, display_name, is_admin, enabled, created_at FROM users WHERE id = ?1",
                )?;
                let item = stmt
                    .query_map(rusqlite::params![id], row_to_user)?
                    .next()
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)??;
                Ok(item)
            })
            .await
            .map_err(AppError::from)
    }

    async fn update_user(&self, id: i64, req: UpdateUserRequest) -> Result<User> {
        let is_admin_i64 = req.is_admin as i64;
        self.conn
            .call(move |conn| {
                conn.execute(
                    "UPDATE users SET display_name = ?1, is_admin = ?2 WHERE id = ?3",
                    rusqlite::params![req.display_name, is_admin_i64, id],
                )?;
                let mut stmt = conn.prepare(
                    "SELECT id, username, display_name, is_admin, enabled, created_at FROM users WHERE id = ?1",
                )?;
                let item = stmt
                    .query_map(rusqlite::params![id], row_to_user)?
                    .next()
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)??;
                Ok(item)
            })
            .await
            .map_err(AppError::from)
    }

    async fn delete_user(&self, id: i64) -> Result<()> {
        self.conn
            .call(move |conn| {
                conn.execute("DELETE FROM users WHERE id = ?1", rusqlite::params![id])?;
                Ok(())
            })
            .await
            .map_err(AppError::from)
    }

    async fn set_user_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let v = enabled as i64;
        self.conn
            .call(move |conn| {
                conn.execute(
                    "UPDATE users SET enabled = ?1 WHERE id = ?2",
                    rusqlite::params![v, id],
                )?;
                Ok(())
            })
            .await
            .map_err(AppError::from)
    }

    async fn set_password_hash(&self, id: i64, hash: &str) -> Result<()> {
        let hash = hash.to_owned();
        self.conn
            .call(move |conn| {
                conn.execute(
                    "UPDATE users SET password_hash = ?1 WHERE id = ?2",
                    rusqlite::params![hash, id],
                )?;
                Ok(())
            })
            .await
            .map_err(AppError::from)
    }

    async fn list_user_tenants(&self, user_id: i64) -> Result<Vec<UserTenant>> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, user_id, tenant_id, is_default, created_at FROM user_tenants WHERE user_id = ?1",
                )?;
                let items = stmt
                    .query_map(rusqlite::params![user_id], row_to_user_tenant)?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(items)
            })
            .await
            .map_err(AppError::from)
    }

    async fn assign_tenant(&self, user_id: i64, tenant_id: i64) -> Result<UserTenant> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "INSERT OR IGNORE INTO user_tenants (user_id, tenant_id) VALUES (?1, ?2)",
                    rusqlite::params![user_id, tenant_id],
                )?;
                let mut stmt = conn.prepare(
                    "SELECT id, user_id, tenant_id, is_default, created_at FROM user_tenants WHERE user_id = ?1 AND tenant_id = ?2",
                )?;
                let item = stmt
                    .query_map(rusqlite::params![user_id, tenant_id], row_to_user_tenant)?
                    .next()
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)??;
                Ok(item)
            })
            .await
            .map_err(AppError::from)
    }

    async fn remove_tenant(&self, user_id: i64, tenant_id: i64) -> Result<()> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "DELETE FROM user_tenants WHERE user_id = ?1 AND tenant_id = ?2",
                    rusqlite::params![user_id, tenant_id],
                )?;
                Ok(())
            })
            .await
            .map_err(AppError::from)
    }

    async fn set_default_tenant(&self, user_id: i64, tenant_id: i64) -> Result<()> {
        self.conn
            .call(move |conn| {
                conn.execute(
                    "UPDATE user_tenants SET is_default = 0 WHERE user_id = ?1",
                    rusqlite::params![user_id],
                )?;
                conn.execute(
                    "UPDATE user_tenants SET is_default = 1 WHERE user_id = ?1 AND tenant_id = ?2",
                    rusqlite::params![user_id, tenant_id],
                )?;
                Ok(())
            })
            .await
            .map_err(AppError::from)
    }

    async fn user_has_tenant(&self, user_id: i64, tenant_id: i64) -> Result<bool> {
        self.conn
            .call(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM user_tenants WHERE user_id = ?1 AND tenant_id = ?2",
                    rusqlite::params![user_id, tenant_id],
                    |row| row.get(0),
                )?;
                Ok(count > 0)
            })
            .await
            .map_err(AppError::from)
    }

    async fn is_empty(&self) -> Result<bool> {
        self.conn
            .call(|conn| {
                let count: i64 =
                    conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
                Ok(count == 0)
            })
            .await
            .map_err(AppError::from)
    }
}
```

---

## Task 7: Update MockStore — tenant_id filter + MockWithTenant query

**Files:**
- Modify: `src/traits/mod.rs`
- Modify: `src/db/mock_store.rs`

- [ ] **Update `MockStore` trait in `src/traits/mod.rs`**

Change `list_mocks` signature and add `list_mocks_for_port_all_tenants`:

```rust
// Change from:
async fn list_mocks(&self, port_id: Option<i64>) -> Result<Vec<MockApi>>;
// To:
async fn list_mocks(&self, port_id: Option<i64>, tenant_id: Option<i64>) -> Result<Vec<MockApi>>;

// Add new method:
async fn list_mocks_for_port_all_tenants(&self, port_id: i64) -> Result<Vec<MockWithTenant>>;
```

Also update `CreateMockRequest` — add `tenant_id: i64` field:

```rust
pub struct CreateMockRequest {
    pub port_id: i64,
    pub tenant_id: i64,   // ← add
    pub name: String,
    // ... rest unchanged
```

- [ ] **Update `src/db/mock_store.rs`**

Update `SELECT_COLS` to include `tenant_id`:

```rust
const SELECT_COLS: &str =
    "id, port_id, name, description, method, path, request_schema, \
     response_status, response_headers, response_body, response_delay_ms, \
     created_at, updated_at, enabled, \
     pagination_enabled, pagination_page_size, request_params, \
     pagination_page_param, pagination_size_param, pagination_data_field, \
     pagination_total_field, tenant_id";
```

Update `row_to_mock` to read `tenant_id` at column index 21:

```rust
Ok(MockApi {
    id: row.get(0)?,
    port_id: row.get(1)?,
    name: row.get(2)?,
    // ... unchanged cols 2-20 ...
    pagination_total_field: row.get::<_, String>(20).unwrap_or_default(),
    tenant_id: row.get::<_, i64>(21).unwrap_or(0),   // ← add
    created_at,
    updated_at,
})
```

Update `list_mocks` implementation to accept optional `tenant_id` filter:

```rust
async fn list_mocks(&self, port_id: Option<i64>, tenant_id: Option<i64>) -> Result<Vec<MockApi>> {
    self.conn
        .call(move |conn| {
            let mut sql = format!("SELECT {} FROM mock_apis", SELECT_COLS);
            let mut wheres = Vec::<String>::new();
            if port_id.is_some() { wheres.push("port_id = ?1".into()); }
            if tenant_id.is_some() { wheres.push(format!("tenant_id = ?{}", if port_id.is_some() { 2 } else { 1 })); }
            if !wheres.is_empty() { sql.push_str(&format!(" WHERE {}", wheres.join(" AND "))); }
            sql.push_str(" ORDER BY id");

            let mut stmt = conn.prepare(&sql)?;
            let items = match (port_id, tenant_id) {
                (Some(p), Some(t)) => stmt.query_map(rusqlite::params![p, t], row_to_mock)?,
                (Some(p), None)    => stmt.query_map(rusqlite::params![p], row_to_mock)?,
                (None, Some(t))    => stmt.query_map(rusqlite::params![t], row_to_mock)?,
                (None, None)       => stmt.query_map([], row_to_mock)?,
            }
            .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(items)
        })
        .await
        .map_err(AppError::from)
}
```

Add `list_mocks_for_port_all_tenants` implementation:

```rust
async fn list_mocks_for_port_all_tenants(&self, port_id: i64) -> Result<Vec<MockWithTenant>> {
    self.conn
        .call(move |conn| {
            let sql = format!(
                "SELECT {}, t.slug FROM mock_apis m JOIN tenants t ON m.tenant_id = t.id \
                 WHERE m.port_id = ?1 AND m.enabled = 1 ORDER BY m.id",
                SELECT_COLS.replace("id,", "m.id,")
                    .replace("port_id,", "m.port_id,")
                    // simpler: use explicit table-qualified cols
            );
            // Use explicit column list to avoid ambiguity:
            let explicit = "m.id, m.port_id, m.name, m.description, m.method, m.path, \
                m.request_schema, m.response_status, m.response_headers, m.response_body, \
                m.response_delay_ms, m.created_at, m.updated_at, m.enabled, \
                m.pagination_enabled, m.pagination_page_size, m.request_params, \
                m.pagination_page_param, m.pagination_size_param, m.pagination_data_field, \
                m.pagination_total_field, m.tenant_id, t.slug";
            let query = format!(
                "SELECT {} FROM mock_apis m JOIN tenants t ON m.tenant_id = t.id \
                 WHERE m.port_id = ?1 AND m.enabled = 1 ORDER BY m.id",
                explicit
            );
            let mut stmt = conn.prepare(&query)?;
            let items = stmt
                .query_map(rusqlite::params![port_id], |row| {
                    let mock = row_to_mock(row)?;
                    let tenant_slug: String = row.get(22)?;
                    Ok(crate::models::MockWithTenant { mock, tenant_slug })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(items)
        })
        .await
        .map_err(AppError::from)
}
```

Update `create_mock` to use `tenant_id` from request:

```rust
async fn create_mock(&self, req: CreateMockRequest) -> Result<MockApi> {
    // ... existing code, add req.tenant_id to INSERT ...
    conn.execute(
        "INSERT INTO mock_apis (port_id, tenant_id, name, description, method, path, \
         request_schema, response_status, response_headers, response_body, response_delay_ms, \
         request_params, pagination_enabled, pagination_page_size, pagination_page_param, \
         pagination_size_param, pagination_data_field, pagination_total_field) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        rusqlite::params![
            req.port_id, req.tenant_id, req.name, req.description,
            req.method.to_string(), req.path, schema_json,
            req.response_status as i64, headers_json, req.response_body,
            req.response_delay_ms as i64, params_json,
            req.pagination_enabled as i64, req.pagination_page_size as i64,
            req.pagination_page_param, req.pagination_size_param,
            req.pagination_data_field, req.pagination_total_field,
        ],
    )?;
```

- [ ] **Fix all call sites of `list_mocks`** — the TUI and any other caller passing one argument:

Search and update every `list_mocks(Some(...))` / `list_mocks(None)` call to pass a second `None` argument:

```bash
grep -rn "list_mocks(" src/ --include="*.rs"
```

For each call site found, add `, None` as the second argument (maintains existing behavior — no tenant filter).

- [ ] **Compile check**

```bash
cargo check 2>&1 | grep "^error" | head -30
```

---

## Task 8: Update LogStore — tenant_id in RequestLog

**Files:**
- Modify: `src/traits/mod.rs`
- Modify: `src/db/log_store.rs`

- [ ] **Add `tenant_id` filter to `LogQuery` in `src/traits/mod.rs`**

```rust
pub struct LogQuery {
    pub port: Option<u16>,
    pub mock_api_id: Option<i64>,
    pub path: Option<String>,
    pub level: Option<String>,
    pub tenant_id: Option<i64>,   // ← add
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    pub page: u32,
    pub page_size: u32,
}
```

- [ ] **Update `src/db/log_store.rs`**

Update `row_to_request_log` to read `tenant_id` at column 14:

```rust
Ok(RequestLog {
    // ... existing cols 0-13 ...
    client_ip: row.get(11)?,
    tenant_id: row.get::<_, Option<i64>>(14).ok().flatten(),   // ← add
    created_at,
})
```

Update `append_request_log` INSERT to include `tenant_id`:

```rust
conn.execute(
    "INSERT INTO request_logs (mock_api_id, port, method, path, query_string, \
     request_headers, request_body, response_status, response_body, duration_ms, \
     client_ip, response_headers, tenant_id) \
     VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
    rusqlite::params![
        log.mock_api_id, log.port as i64, log.method, log.path,
        log.query_string, headers_json, log.request_body,
        log.response_status as i64, log.response_body,
        log.duration_ms as i64, log.client_ip, resp_headers_json,
        log.tenant_id,
    ],
)?;
```

Update `list_request_logs` WHERE-clause builder to filter by `tenant_id` when set:

```rust
if let Some(tid) = query.tenant_id {
    conditions.push(format!("tenant_id = ?{}", params.len() + 1));
    params.push(Box::new(tid));
}
```

- [ ] **Compile check**

```bash
cargo check 2>&1 | grep "^error" | head -30
```

---

## Task 9: Update AppState, DB module, and admin seed

**Files:**
- Modify: `src/db/mod.rs`
- Modify: `src/main.rs`

- [ ] **Update `src/db/mod.rs`** — export new stores and add seed function:

```rust
mod log_store;
mod mock_store;
mod port_store;
mod schema;
mod tenant_store;
mod user_store;

pub use log_store::SqliteLogStore;
pub use mock_store::SqliteMockStore;
pub use port_store::SqlitePortStore;
pub use tenant_store::SqliteTenantStore;
pub use user_store::SqliteUserStore;

use tokio_rusqlite::Connection;
use crate::error::Result;

pub async fn open(path: &str) -> Result<Connection> {
    let conn = Connection::open(path).await?;
    conn.call(|c| {
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        schema::run_migrations(c)?;
        Ok(())
    }).await?;
    Ok(conn)
}

/// Seed the admin account on first startup if the users table is empty.
pub async fn seed_admin(conn: &Connection) -> Result<()> {
    use crate::traits::UserStore;
    let store = SqliteUserStore::new(conn.clone());
    if store.is_empty().await? {
        let hash = crate::auth::hash_password("Admin@123")
            .map_err(|e| crate::error::AppError::Other(e.to_string()))?;
        store.create_user(crate::traits::CreateUserRequest {
            username: "admin".into(),
            display_name: "Administrator".into(),
            password_hash: hash,
            is_admin: true,
        }).await?;
        tracing::info!("Seeded default admin account (admin/Admin@123)");
    }
    Ok(())
}
```

- [ ] **Update `AppState` in `src/main.rs`**

```rust
#[derive(Clone)]
pub struct AppState {
    pub mock_store: Arc<dyn traits::MockStore>,
    pub port_store: Arc<dyn traits::PortStore>,
    pub log_store: Arc<dyn traits::LogStore>,
    pub user_store: Arc<dyn traits::UserStore>,
    pub tenant_store: Arc<dyn traits::TenantStore>,
    pub port_manager: Arc<dyn PortManager>,
    pub log_tx: broadcast::Sender<LogEvent>,
    pub management_port: u16,
    pub jwt_secret: String,
}
```

Implement `FromRef<AppState>` for `AppState` (needed by auth extractor):

```rust
impl axum::extract::FromRef<AppState> for AppState {
    fn from_ref(state: &AppState) -> Self {
        state.clone()
    }
}
```

- [ ] **Initialize new stores and jwt_secret in `main()`**

```rust
use crate::db::{SqliteLogStore, SqliteMockStore, SqlitePortStore, SqliteTenantStore, SqliteUserStore};

// after `let conn = db::open(&cli.db).await?;`
db::seed_admin(&conn).await?;

let user_store: Arc<dyn traits::UserStore> = Arc::new(SqliteUserStore::new(conn.clone()));
let tenant_store: Arc<dyn traits::TenantStore> = Arc::new(SqliteTenantStore::new(conn.clone()));

// generate jwt_secret once per process
let jwt_secret: String = {
    use rand::Rng;
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
};

// update AppState construction:
let state = AppState {
    mock_store,
    port_store,
    log_store,
    user_store,
    tenant_store,
    port_manager: port_manager.clone(),
    log_tx,
    management_port: cli.port,
    jwt_secret,
};
```

- [ ] **Full compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: 0 errors.

- [ ] **Commit all backend foundation work** (Tasks 3–9):

```bash
git add src/
git commit -m "feat: add auth module, tenant/user stores, update AppState with jwt_secret"
```

---

## Task 10: Auth routes

**Files:**
- Create: `src/dashboard/routes/auth.rs`

- [ ] **Create `src/dashboard/routes/auth.rs`**

```rust
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::{encode_token, make_claims, verify_password, AuthUser};
use crate::models::{Tenant, User};
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

    let user_tenants = state.user_store.list_user_tenants(user.id).await.unwrap_or_default();
    let tenant_ids: Vec<i64> = user_tenants.iter().map(|ut| ut.tenant_id).collect();
    let all_tenants = state.tenant_store.list_tenants().await.unwrap_or_default();

    let tenants: Vec<Tenant> = if user.is_admin {
        all_tenants.clone()
    } else {
        all_tenants.into_iter().filter(|t| tenant_ids.contains(&t.id)).collect()
    };

    let default_tenant_id = user_tenants.iter().find(|ut| ut.is_default).map(|ut| ut.tenant_id)
        .or_else(|| tenant_ids.first().copied());
    let current_tenant = tenants.iter().find(|t| Some(t.id) == default_tenant_id).cloned();

    let claims = make_claims(
        user.id, &user.username, &user.display_name,
        user.is_admin, current_tenant.as_ref().map(|t| t.id),
    );
    let token = match encode_token(&claims, &state.jwt_secret) {
        Ok(t) => t,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    Json(LoginResponse { token, user, tenants, current_tenant }).into_response()
}

pub async fn logout() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub async fn switch_tenant(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<SwitchTenantBody>,
) -> impl IntoResponse {
    // admin can switch to any tenant; regular user must belong to it
    if !claims.is_admin {
        match state.user_store.user_has_tenant(claims.user_id, body.tenant_id).await {
            Ok(true) => {}
            _ => return (StatusCode::FORBIDDEN, "not a member of this tenant").into_response(),
        }
    }
    let new_claims = make_claims(
        claims.user_id, &claims.username, &claims.display_name,
        claims.is_admin, Some(body.tenant_id),
    );
    match encode_token(&new_claims, &state.jwt_secret) {
        Ok(token) => Json(serde_json::json!({ "token": token })).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn me(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> impl IntoResponse {
    let user = match state.user_store.get_user(claims.user_id).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let user_tenants = state.user_store.list_user_tenants(user.id).await.unwrap_or_default();
    let tenant_ids: Vec<i64> = user_tenants.iter().map(|ut| ut.tenant_id).collect();
    let all_tenants = state.tenant_store.list_tenants().await.unwrap_or_default();
    let tenants: Vec<Tenant> = if user.is_admin {
        all_tenants.clone()
    } else {
        all_tenants.into_iter().filter(|t| tenant_ids.contains(&t.id)).collect()
    };
    let current_tenant = claims.current_tenant_id
        .and_then(|id| tenants.iter().find(|t| t.id == id).cloned());
    Json(MeResponse { user, tenants, current_tenant }).into_response()
}

pub async fn set_default_tenant(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<SetDefaultTenantBody>,
) -> impl IntoResponse {
    match state.user_store.set_default_tenant(claims.user_id, body.tenant_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
```

---

## Task 11: Admin — tenant routes

**Files:**
- Create: `src/dashboard/routes/admin_tenants.rs`

- [ ] **Create `src/dashboard/routes/admin_tenants.rs`**

```rust
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
    let req = CreateTenantRequest { name: body.name, slug: body.slug };
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
    let req = UpdateTenantRequest { name: body.name, slug: body.slug, enabled: body.enabled };
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
        Ok(true) => return (StatusCode::CONFLICT,
            "tenant has mock APIs; remove them before deleting the tenant").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Ok(false) => {}
    }
    match state.tenant_store.delete_tenant(id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
```

---

## Task 12: Admin — user routes

**Files:**
- Create: `src/dashboard/routes/admin_users.rs`

- [ ] **Create `src/dashboard/routes/admin_users.rs`**

```rust
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

pub async fn list_users(State(state): State<AppState>, AdminUser(_): AdminUser) -> impl IntoResponse {
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
```

---

## Task 13: Update existing route handlers for auth + tenant scope

**Files:**
- Modify: `src/dashboard/routes/ports.rs`
- Modify: `src/dashboard/routes/mocks.rs`
- Modify: `src/dashboard/routes/logs.rs`

- [ ] **`src/dashboard/routes/ports.rs`** — add `AdminUser` extractor to mutating handlers, `AuthUser` to read handlers:

```rust
// add to imports
use crate::auth::{AdminUser, AuthUser};

// list_ports: add AuthUser (any authenticated user can list)
pub async fn list_ports(
    State(state): State<AppState>,
    AuthUser(_): AuthUser,
) -> impl IntoResponse { /* unchanged body */ }

// create_port, update_port, delete_port, start_port, stop_port, restart_port:
// replace State(...) signature to add AdminUser
pub async fn create_port(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Json(body): Json<CreatePortBody>,
) -> impl IntoResponse { /* unchanged body */ }
// (repeat for update_port, delete_port, start_port, stop_port, restart_port, port_status)

// delete_port: add usage check before deleting
pub async fn delete_port(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    // Check across all tenants
    match state.tenant_store.has_mocks_for_port(id).await { /* see note below */ }
    // ...
}
```

Note: add `has_mocks_for_port(port_id: i64) -> Result<bool>` to `TenantStore` trait and `SqliteTenantStore`:

```rust
// In trait:
async fn has_mocks_for_port(&self, port_id: i64) -> Result<bool>;

// In SqliteTenantStore impl:
async fn has_mocks_for_port(&self, port_id: i64) -> Result<bool> {
    self.conn.call(move |conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM mock_apis WHERE port_id = ?1",
            rusqlite::params![port_id], |row| row.get(0))?;
        Ok(count > 0)
    }).await.map_err(AppError::from)
}
```

Then in `delete_port`:

```rust
pub async fn delete_port(
    State(state): State<AppState>,
    AdminUser(_): AdminUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.tenant_store.has_mocks_for_port(id).await {
        Ok(true) => return (StatusCode::CONFLICT,
            "port has mock APIs across tenants; remove them first").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        Ok(false) => {}
    }
    match state.port_store.delete_port(id).await {
        Ok(_) => {
            let _ = state.log_tx.send(LogEvent::StateChanged { resource: StateResource::Ports });
            StatusCode::NO_CONTENT.into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}
```

- [ ] **`src/dashboard/routes/mocks.rs`** — scope to current tenant:

```rust
use crate::auth::AuthUser;

pub async fn list_mocks(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(params): Query<ListMocksParams>,
) -> impl IntoResponse {
    let tenant_id = if claims.is_admin { None } else { claims.current_tenant_id };
    match state.mock_store.list_mocks(params.port_id, tenant_id).await {
        Ok(mocks) => Json(mocks).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn create_mock(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<CreateMockBody>,
) -> impl IntoResponse {
    let tenant_id = match claims.current_tenant_id {
        Some(id) => id,
        None => return (StatusCode::BAD_REQUEST, "no active tenant").into_response(),
    };
    let req = CreateMockRequest {
        port_id: body.port_id,
        tenant_id,
        // ... rest of fields from body
    };
    // ... rest unchanged
}
```

- [ ] **`src/dashboard/routes/logs.rs`** — scope to current tenant:

```rust
use crate::auth::AuthUser;

pub async fn list_request_logs(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Query(params): Query<RequestLogQuery>,
) -> impl IntoResponse {
    let tenant_id = if claims.is_admin { None } else { claims.current_tenant_id };
    let query = LogQuery {
        tenant_id,
        // ... rest from params
    };
    // ... rest unchanged
}
// same pattern for list_system_logs, clear_request_logs, clear_system_logs
```

---

## Task 14: Update dashboard router

**Files:**
- Modify: `src/dashboard/mod.rs`
- Modify: `src/dashboard/routes/mod.rs` (add new modules)

- [ ] **Update `src/dashboard/routes/mod.rs`** to expose new modules:

```rust
pub mod admin_tenants;
pub mod admin_users;
pub mod auth;
pub mod info;
pub mod logs;
pub mod mocks;
pub mod ports;
```

- [ ] **Update `src/dashboard/mod.rs`** — add auth and admin routes:

```rust
use routes::{admin_tenants, admin_users, auth as auth_routes, info, logs, mocks, ports};
use axum::routing::{delete, get, patch, post, put};

fn build_router(state: AppState) -> Router {
    let auth_api = Router::new()
        .route("/login", post(auth_routes::login))
        .route("/logout", post(auth_routes::logout))
        .route("/switch-tenant", post(auth_routes::switch_tenant))
        .route("/me", get(auth_routes::me))
        .route("/me/default-tenant", put(auth_routes::set_default_tenant));

    let admin_api = Router::new()
        // Tenants
        .route("/tenants", get(admin_tenants::list_tenants).post(admin_tenants::create_tenant))
        .route("/tenants/:id", get(admin_tenants::get_tenant)
            .put(admin_tenants::update_tenant)
            .delete(admin_tenants::delete_tenant))
        // Users
        .route("/users", get(admin_users::list_users).post(admin_users::create_user))
        .route("/users/:id", get(admin_users::get_user)
            .put(admin_users::update_user)
            .delete(admin_users::delete_user))
        .route("/users/:id/disable", post(admin_users::disable_user))
        .route("/users/:id/enable", post(admin_users::enable_user))
        .route("/users/:id/reset-password", post(admin_users::reset_password))
        .route("/users/:id/tenants", get(admin_users::list_user_tenants)
            .post(admin_users::assign_tenant))
        .route("/users/:id/tenants/:tenant_id", delete(admin_users::remove_tenant));

    let api = Router::new()
        .route("/info", get(info::get_info))
        .route("/ports", get(ports::list_ports).post(ports::create_port))
        .route("/ports/:id", get(ports::get_port).put(ports::update_port).delete(ports::delete_port))
        .route("/ports/:id/start", post(ports::start_port))
        .route("/ports/:id/stop", post(ports::stop_port))
        .route("/ports/:id/restart", post(ports::restart_port))
        .route("/ports/:id/status", get(ports::port_status))
        .route("/mocks", get(mocks::list_mocks).post(mocks::create_mock))
        .route("/mocks/:id", get(mocks::get_mock).put(mocks::update_mock).delete(mocks::delete_mock))
        .route("/mocks/:id/enabled", patch(mocks::set_mock_enabled))
        .route("/logs/requests", get(logs::list_request_logs).delete(logs::clear_request_logs))
        .route("/logs/requests/:id", get(logs::get_request_log))
        .route("/logs/system", get(logs::list_system_logs).delete(logs::clear_system_logs))
        .nest("/auth", auth_api)
        .nest("/admin", admin_api);

    Router::new()
        .nest("/api/v1", api)
        .route("/ws/logs", get(ws::ws_logs))
        .fallback(static_files::static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state)
}
```

- [ ] **Full cargo build**

```bash
cargo build 2>&1 | grep "^error" | head -40
```

Fix any remaining compile errors. Common ones:
- Missing `use` imports in route files
- `list_mocks` call sites in TUI needing second `None` argument

- [ ] **Smoke test auth endpoints**

```bash
cargo run -- serve &
sleep 2

# Login
curl -s -X POST http://localhost:9999/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"Admin@123"}' | jq .

# Should return {"token":"...","user":{...},"tenants":[...],"current_tenant":...}
kill %1
```

- [ ] **Commit**

```bash
git add src/
git commit -m "feat: add auth/admin routes, scope mocks+logs to tenant, admin-only port mutations"
```

---

## Task 15: Update mock server — tenant routing

**Files:**
- Modify: `src/server/manager.rs`
- Modify: `src/server/handler.rs`

- [ ] **Update `src/server/manager.rs`** — load `MockWithTenant` instead of `MockApi`:

In `spawn_server`, replace:

```rust
let mocks = self.mock_store.list_mocks(Some(port_id)).await?;
let mocks: Vec<_> = mocks.into_iter().filter(|m| m.enabled).collect();

let state = MockHandlerState {
    port: config.port,
    mocks: Arc::new(mocks),
```

With:

```rust
let mocks = self.mock_store.list_mocks_for_port_all_tenants(port_id).await?;

let state = MockHandlerState {
    port: config.port,
    mocks: Arc::new(mocks),
```

- [ ] **Update `src/server/handler.rs`** — route by tenant slug:

Change `MockHandlerState`:

```rust
use crate::models::MockWithTenant;

#[derive(Clone)]
pub struct MockHandlerState {
    pub port: u16,
    pub mocks: Arc<Vec<MockWithTenant>>,
    pub log_store: Arc<dyn LogStore>,
    pub log_tx: broadcast::Sender<LogEvent>,
}
```

Add `split_tenant_path` helper:

```rust
/// Splits "/acme/api/users" into ("acme", "/api/users").
/// Splits "/acme" into ("acme", "/").
fn split_tenant_path(path: &str) -> (&str, &str) {
    let trimmed = path.trim_start_matches('/');
    match trimmed.find('/') {
        Some(idx) => (&trimmed[..idx], &trimmed[idx..]),
        None => (trimmed, "/"),
    }
}
```

Update `find_mock` to work with `MockWithTenant`:

```rust
fn find_mock<'a>(
    mocks: &'a [MockWithTenant],
    method: &HttpMethod,
    path: &str,
) -> Option<&'a MockWithTenant> {
    let (tenant_slug, remaining) = split_tenant_path(path);
    let exact = mocks.iter().find(|m| {
        m.mock.enabled
            && m.tenant_slug == tenant_slug
            && &m.mock.method == method
            && path_matches(&m.mock.path, remaining)
    });
    if exact.is_some() {
        return exact;
    }
    mocks.iter().find(|m| {
        m.mock.enabled
            && m.tenant_slug == tenant_slug
            && m.mock.method == HttpMethod::ANY
            && path_matches(&m.mock.path, remaining)
    })
}
```

Update `mock_fallback` — `matched` is now `Option<&MockWithTenant>`, so access via `matched.mock.*`:

```rust
let matched = find_mock(&state.mocks, &method, &path);

let (response, mock_id, resp_body_str, resp_headers_map) = if let Some(m) = matched {
    let mock = &m.mock;
    let tenant_id = mock.tenant_id;
    // replace all `mock.xxx` references to use `mock.xxx` (already works since mock is &MockApi)
    // ...
    // When building RequestLog, add tenant_id:
    // tenant_id: Some(tenant_id),
```

In the log construction at the bottom of `mock_fallback`, set `tenant_id`:

```rust
let log = RequestLog {
    // ... existing fields ...
    tenant_id: matched.map(|m| m.mock.tenant_id),
    // ...
};
```

- [ ] **Build and test routing**

```bash
cargo build

# Start server
cargo run -- serve &
sleep 2

# Create tenant "acme" and a mock via API (use token from Task 14 smoke test)
TOKEN=$(curl -s -X POST http://localhost:9999/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"Admin@123"}' | jq -r .token)

# Create a port
curl -s -X POST http://localhost:9999/api/v1/ports \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"port":3000,"label":"test"}' | jq .

# Create tenant
curl -s -X POST http://localhost:9999/api/v1/admin/tenants \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"Acme","slug":"acme"}' | jq .

# Get tenant id from response, then create a mock
# (use tenant_id from response above, e.g. 2)
# Switch to that tenant first
TOKEN=$(curl -s -X POST http://localhost:9999/api/v1/auth/switch-tenant \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"tenant_id":2}' | jq -r .token)

curl -s -X POST http://localhost:9999/api/v1/mocks \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"port_id":1,"name":"test","description":"","method":"GET","path":"/hello","response_status":200,"response_body":"{\"ok\":true}","response_headers":{},"response_delay_ms":0,"request_params":{},"pagination_enabled":false,"pagination_page_size":10,"pagination_page_param":"page","pagination_size_param":"page_size","pagination_data_field":"","pagination_total_field":""}' | jq .

# Start port
curl -s -X POST http://localhost:9999/api/v1/ports/1/start \
  -H "Authorization: Bearer $TOKEN"

sleep 1

# Hit the mock via tenant-prefixed URL
curl -s http://localhost:3000/acme/hello
# Expected: {"ok":true}

kill %1
```

- [ ] **Commit**

```bash
git add src/server/
git commit -m "feat: mock server routes requests by tenant slug URL prefix"
```

---

## Task 16: Frontend — auth store + API client

**Files:**
- Create: `frontend/src/stores/auth.ts`
- Modify: `frontend/src/api/client.ts`

- [ ] **Create `frontend/src/stores/auth.ts`**

```typescript
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { login as apiLogin, switchTenant as apiSwitchTenant, getMe } from '../api/client'

export interface UserInfo {
  id: number
  username: string
  display_name: string
  is_admin: boolean
  enabled: boolean
  created_at: string
}

export interface TenantInfo {
  id: number
  name: string
  slug: string
  enabled: boolean
  created_at: string
}

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('token'))
  const user = ref<UserInfo | null>(null)
  const tenants = ref<TenantInfo[]>([])
  const currentTenant = ref<TenantInfo | null>(null)

  const isLoggedIn = computed(() => !!token.value)
  const isAdmin = computed(() => user.value?.is_admin ?? false)

  function parseToken(t: string) {
    try {
      const payload = JSON.parse(atob(t.split('.')[1]))
      return payload
    } catch {
      return null
    }
  }

  function setToken(t: string) {
    token.value = t
    localStorage.setItem('token', t)
  }

  function clearAuth() {
    token.value = null
    user.value = null
    tenants.value = []
    currentTenant.value = null
    localStorage.removeItem('token')
  }

  async function loginAction(username: string, password: string) {
    const res = await apiLogin(username, password)
    setToken(res.token)
    user.value = res.user
    tenants.value = res.tenants
    currentTenant.value = res.current_tenant
  }

  async function switchTenantAction(tenantId: number) {
    const res = await apiSwitchTenant(tenantId)
    setToken(res.token)
    const t = tenants.value.find(t => t.id === tenantId) ?? null
    currentTenant.value = t
  }

  async function fetchMe() {
    if (!token.value) return
    try {
      const res = await getMe()
      user.value = res.user
      tenants.value = res.tenants
      currentTenant.value = res.current_tenant
    } catch {
      clearAuth()
    }
  }

  function logout() {
    clearAuth()
  }

  return {
    token, user, tenants, currentTenant,
    isLoggedIn, isAdmin,
    loginAction, switchTenantAction, fetchMe, logout, setToken, clearAuth,
  }
})
```

- [ ] **Update `frontend/src/api/client.ts`** — add auth types + interceptors + new endpoints:

Add new types at the top of client.ts:

```typescript
export interface UserInfo {
  id: number; username: string; display_name: string
  is_admin: boolean; enabled: boolean; created_at: string
}
export interface TenantInfo {
  id: number; name: string; slug: string; enabled: boolean; created_at: string
}
export interface UserTenant {
  id: number; user_id: number; tenant_id: number; is_default: boolean; created_at: string
}
export interface LoginResponse {
  token: string; user: UserInfo; tenants: TenantInfo[]; current_tenant: TenantInfo | null
}
export interface MeResponse {
  user: UserInfo; tenants: TenantInfo[]; current_tenant: TenantInfo | null
}
```

Add auth interceptors after `const http = axios.create(...)`:

```typescript
import router from '../router'

http.interceptors.request.use((config) => {
  const token = localStorage.getItem('token')
  if (token) config.headers.Authorization = `Bearer ${token}`
  return config
})

http.interceptors.response.use(
  (res) => res,
  (err) => {
    if (err.response?.status === 401) {
      localStorage.removeItem('token')
      router.push('/login')
    }
    return Promise.reject(err)
  }
)
```

Add new API functions:

```typescript
// Auth
export const login = (username: string, password: string): Promise<LoginResponse> =>
  http.post('/auth/login', { username, password }).then(r => r.data)
export const logout = (): Promise<void> =>
  http.post('/auth/logout').then(() => undefined)
export const switchTenant = (tenant_id: number): Promise<{ token: string }> =>
  http.post('/auth/switch-tenant', { tenant_id }).then(r => r.data)
export const getMe = (): Promise<MeResponse> =>
  http.get('/auth/me').then(r => r.data)
export const setDefaultTenant = (tenant_id: number): Promise<void> =>
  http.put('/auth/me/default-tenant', { tenant_id }).then(() => undefined)

// Admin — tenants
export const adminListTenants = (): Promise<TenantInfo[]> =>
  http.get('/admin/tenants').then(r => r.data)
export const adminCreateTenant = (data: { name: string; slug: string }): Promise<TenantInfo> =>
  http.post('/admin/tenants', data).then(r => r.data)
export const adminUpdateTenant = (id: number, data: { name: string; slug: string; enabled: boolean }): Promise<TenantInfo> =>
  http.put(`/admin/tenants/${id}`, data).then(r => r.data)
export const adminDeleteTenant = (id: number): Promise<void> =>
  http.delete(`/admin/tenants/${id}`).then(() => undefined)

// Admin — users
export const adminListUsers = (): Promise<UserInfo[]> =>
  http.get('/admin/users').then(r => r.data)
export const adminCreateUser = (data: { username: string; display_name: string; password: string; is_admin: boolean }): Promise<UserInfo> =>
  http.post('/admin/users', data).then(r => r.data)
export const adminUpdateUser = (id: number, data: { display_name: string; is_admin: boolean }): Promise<UserInfo> =>
  http.put(`/admin/users/${id}`, data).then(r => r.data)
export const adminDeleteUser = (id: number): Promise<void> =>
  http.delete(`/admin/users/${id}`).then(() => undefined)
export const adminDisableUser = (id: number): Promise<void> =>
  http.post(`/admin/users/${id}/disable`).then(() => undefined)
export const adminEnableUser = (id: number): Promise<void> =>
  http.post(`/admin/users/${id}/enable`).then(() => undefined)
export const adminResetPassword = (id: number, password: string): Promise<void> =>
  http.post(`/admin/users/${id}/reset-password`, { password }).then(() => undefined)
export const adminListUserTenants = (userId: number): Promise<UserTenant[]> =>
  http.get(`/admin/users/${userId}/tenants`).then(r => r.data)
export const adminAssignTenant = (userId: number, tenant_id: number): Promise<UserTenant> =>
  http.post(`/admin/users/${userId}/tenants`, { tenant_id }).then(r => r.data)
export const adminRemoveTenant = (userId: number, tenantId: number): Promise<void> =>
  http.delete(`/admin/users/${userId}/tenants/${tenantId}`).then(() => undefined)
```

---

## Task 17: Frontend — Login view + router guards

**Files:**
- Create: `frontend/src/views/LoginView.vue`
- Modify: `frontend/src/router/index.ts`

- [ ] **Create `frontend/src/views/LoginView.vue`**

```vue
<template>
  <div class="flex items-center justify-center min-h-screen bg-surface-ground">
    <div class="w-full max-w-sm p-8 rounded-xl shadow-lg bg-surface-0 border border-surface-200">
      <h2 class="text-2xl font-bold mb-6 text-center">Mock APIs Dashboard</h2>
      <div class="flex flex-col gap-4">
        <div class="flex flex-col gap-1">
          <label class="text-sm font-medium">Username</label>
          <InputText v-model="username" placeholder="Username" @keyup.enter="handleLogin" />
        </div>
        <div class="flex flex-col gap-1">
          <label class="text-sm font-medium">Password</label>
          <Password v-model="password" :feedback="false" toggleMask @keyup.enter="handleLogin" />
        </div>
        <Message v-if="error" severity="error" :closable="false">{{ error }}</Message>
        <Button label="Login" :loading="loading" class="w-full" @click="handleLogin" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = useRouter()
const auth = useAuthStore()
const username = ref('')
const password = ref('')
const error = ref('')
const loading = ref(false)

async function handleLogin() {
  if (!username.value || !password.value) return
  loading.value = true
  error.value = ''
  try {
    await auth.loginAction(username.value, password.value)
    router.push('/')
  } catch (e: any) {
    error.value = e.response?.data || 'Login failed'
  } finally {
    loading.value = false
  }
}
</script>
```

- [ ] **Update `frontend/src/router/index.ts`** — add `/login` route and navigation guards:

```typescript
import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const routes = [
  { path: '/login', component: () => import('../views/LoginView.vue'), meta: { public: true } },
  { path: '/', component: () => import('../views/DashboardView.vue'), meta: { title: 'Dashboard' } },
  { path: '/ports', component: () => import('../views/PortsView.vue'), meta: { title: 'Ports' } },
  { path: '/mocks', component: () => import('../views/MocksView.vue'), meta: { title: 'Mocks' } },
  { path: '/logs', component: () => import('../views/LogsView.vue'), meta: { title: 'Logs' } },
  { path: '/functions', component: () => import('../views/FunctionsView.vue'), meta: { title: 'Functions' } },
  { path: '/admin', component: () => import('../views/AdminView.vue'), meta: { title: 'Admin', adminOnly: true } },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach((to) => {
  const auth = useAuthStore()
  if (to.meta.public) return true
  if (!auth.isLoggedIn) return '/login'
  if (to.meta.adminOnly && !auth.isAdmin) return '/'
  return true
})

export default router
```

---

## Task 18: Frontend — Tenant switcher in TopBar + user display

**Files:**
- Modify: `frontend/src/components/layout/AppTopBar.vue`

- [ ] **Update `AppTopBar.vue`** — add tenant dropdown and user chip:

```vue
<template>
  <div class="flex items-center justify-between px-4 py-2 border-b border-surface-200 bg-surface-0">
    <span class="font-semibold text-lg">{{ title }}</span>
    <div class="flex items-center gap-3">
      <!-- Tenant switcher (hidden when only 1 tenant or no tenant) -->
      <Select
        v-if="auth.tenants.length > 1"
        :modelValue="auth.currentTenant?.id"
        :options="auth.tenants"
        optionLabel="name"
        optionValue="id"
        placeholder="Select tenant"
        class="w-48"
        @change="onTenantChange"
      />
      <Chip v-else-if="auth.currentTenant" :label="auth.currentTenant.name" />

      <!-- User display -->
      <span class="text-sm text-surface-500">{{ auth.user?.display_name || auth.user?.username }}</span>

      <!-- Dark mode toggle -->
      <Button
        :icon="isDark ? 'pi pi-sun' : 'pi pi-moon'"
        text rounded
        @click="toggleDark"
      />

      <!-- Logout -->
      <Button icon="pi pi-sign-out" text rounded severity="secondary" @click="handleLogout" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAuthStore } from '../../stores/auth'
import { useToast } from 'primevue/usetoast'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const toast = useToast()

const title = computed(() => route.meta?.title as string ?? '')

// dark mode (existing logic — preserve as-is)
const isDark = /* existing ref */ false
function toggleDark() { /* existing logic */ }

async function onTenantChange(e: { value: number }) {
  try {
    await auth.switchTenantAction(e.value)
    // reload stores after tenant switch
    window.location.reload()
  } catch {
    toast.add({ severity: 'error', summary: 'Failed to switch tenant', life: 3000 })
  }
}

function handleLogout() {
  auth.logout()
  router.push('/login')
}
</script>
```

---

## Task 19: Frontend — Admin view (tenants + users)

**Files:**
- Create: `frontend/src/views/AdminView.vue`
- Modify: `frontend/src/components/layout/AppSidebar.vue`

- [ ] **Create `frontend/src/views/AdminView.vue`** with two tabs:

```vue
<template>
  <div class="p-4">
    <TabView>
      <TabPanel header="Tenants">
        <!-- Tenant list table -->
        <div class="flex justify-end mb-3">
          <Button label="New Tenant" icon="pi pi-plus" @click="openTenantDialog()" />
        </div>
        <DataTable :value="tenants" :loading="tenantsLoading">
          <Column field="name" header="Name" />
          <Column field="slug" header="Slug" />
          <Column field="enabled" header="Enabled">
            <template #body="{ data }">
              <Tag :severity="data.enabled ? 'success' : 'secondary'" :value="data.enabled ? 'Yes' : 'No'" />
            </template>
          </Column>
          <Column header="Actions">
            <template #body="{ data }">
              <Button icon="pi pi-pencil" text @click="openTenantDialog(data)" />
              <Button icon="pi pi-trash" text severity="danger" @click="confirmDeleteTenant(data)" />
            </template>
          </Column>
        </DataTable>
      </TabPanel>

      <TabPanel header="Users">
        <!-- User list table -->
        <div class="flex justify-end mb-3">
          <Button label="New User" icon="pi pi-plus" @click="openUserDialog()" />
        </div>
        <DataTable :value="users" :loading="usersLoading">
          <Column field="username" header="Username" />
          <Column field="display_name" header="Display Name" />
          <Column field="is_admin" header="Admin">
            <template #body="{ data }">
              <Tag v-if="data.is_admin" severity="warn" value="Admin" />
            </template>
          </Column>
          <Column field="enabled" header="Status">
            <template #body="{ data }">
              <Tag :severity="data.enabled ? 'success' : 'secondary'" :value="data.enabled ? 'Active' : 'Disabled'" />
            </template>
          </Column>
          <Column header="Actions">
            <template #body="{ data }">
              <Button icon="pi pi-pencil" text @click="openUserDialog(data)" />
              <Button icon="pi pi-key" text severity="warn" @click="openResetPasswordDialog(data)" />
              <Button :icon="data.enabled ? 'pi pi-ban' : 'pi pi-check'" text
                :severity="data.enabled ? 'danger' : 'success'"
                @click="toggleUserEnabled(data)" />
              <Button icon="pi pi-trash" text severity="danger" @click="confirmDeleteUser(data)" />
            </template>
          </Column>
        </DataTable>
      </TabPanel>
    </TabView>

    <!-- Tenant Dialog -->
    <Dialog v-model:visible="tenantDialog" :header="editingTenant ? 'Edit Tenant' : 'New Tenant'" modal>
      <div class="flex flex-col gap-3 w-80">
        <div><label>Name</label><InputText v-model="tenantForm.name" class="w-full mt-1" /></div>
        <div><label>Slug</label><InputText v-model="tenantForm.slug" class="w-full mt-1" placeholder="url-safe-slug" /></div>
        <ToggleButton v-if="editingTenant" v-model="tenantForm.enabled" onLabel="Enabled" offLabel="Disabled" />
      </div>
      <template #footer>
        <Button label="Cancel" text @click="tenantDialog = false" />
        <Button label="Save" @click="saveTenant" />
      </template>
    </Dialog>

    <!-- User Dialog -->
    <Dialog v-model:visible="userDialog" :header="editingUser ? 'Edit User' : 'New User'" modal>
      <div class="flex flex-col gap-3 w-80">
        <div v-if="!editingUser"><label>Username</label><InputText v-model="userForm.username" class="w-full mt-1" /></div>
        <div><label>Display Name</label><InputText v-model="userForm.display_name" class="w-full mt-1" /></div>
        <div v-if="!editingUser"><label>Password</label><Password v-model="userForm.password" :feedback="false" toggleMask class="w-full mt-1" /></div>
        <div class="flex items-center gap-2"><Checkbox v-model="userForm.is_admin" binary /><label>Administrator</label></div>
      </div>
      <template #footer>
        <Button label="Cancel" text @click="userDialog = false" />
        <Button label="Save" @click="saveUser" />
      </template>
    </Dialog>

    <!-- Reset Password Dialog -->
    <Dialog v-model:visible="resetPasswordDialog" header="Reset Password" modal>
      <div class="flex flex-col gap-3 w-72">
        <Password v-model="newPassword" :feedback="false" toggleMask placeholder="New password" />
      </div>
      <template #footer>
        <Button label="Cancel" text @click="resetPasswordDialog = false" />
        <Button label="Reset" severity="warn" @click="doResetPassword" />
      </template>
    </Dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useToast } from 'primevue/usetoast'
import { useConfirm } from 'primevue/useconfirm'
import {
  adminListTenants, adminCreateTenant, adminUpdateTenant, adminDeleteTenant,
  adminListUsers, adminCreateUser, adminUpdateUser, adminDeleteUser,
  adminDisableUser, adminEnableUser, adminResetPassword,
  type UserInfo, type TenantInfo,
} from '../api/client'

const toast = useToast()
const confirm = useConfirm()

const tenants = ref<TenantInfo[]>([])
const users = ref<UserInfo[]>([])
const tenantsLoading = ref(false)
const usersLoading = ref(false)

const tenantDialog = ref(false)
const editingTenant = ref<TenantInfo | null>(null)
const tenantForm = ref({ name: '', slug: '', enabled: true })

const userDialog = ref(false)
const editingUser = ref<UserInfo | null>(null)
const userForm = ref({ username: '', display_name: '', password: '', is_admin: false })

const resetPasswordDialog = ref(false)
const resetPasswordTarget = ref<UserInfo | null>(null)
const newPassword = ref('')

async function loadTenants() {
  tenantsLoading.value = true
  tenants.value = await adminListTenants().finally(() => { tenantsLoading.value = false })
}
async function loadUsers() {
  usersLoading.value = true
  users.value = await adminListUsers().finally(() => { usersLoading.value = false })
}

onMounted(() => { loadTenants(); loadUsers() })

function openTenantDialog(t?: TenantInfo) {
  editingTenant.value = t ?? null
  tenantForm.value = t ? { name: t.name, slug: t.slug, enabled: t.enabled } : { name: '', slug: '', enabled: true }
  tenantDialog.value = true
}

async function saveTenant() {
  try {
    if (editingTenant.value) {
      await adminUpdateTenant(editingTenant.value.id, tenantForm.value)
    } else {
      await adminCreateTenant({ name: tenantForm.value.name, slug: tenantForm.value.slug })
    }
    tenantDialog.value = false
    await loadTenants()
    toast.add({ severity: 'success', summary: 'Saved', life: 2000 })
  } catch (e: any) {
    toast.add({ severity: 'error', summary: e.response?.data || 'Error', life: 3000 })
  }
}

function confirmDeleteTenant(t: TenantInfo) {
  confirm.require({
    message: `Delete tenant "${t.name}"?`,
    accept: async () => {
      try {
        await adminDeleteTenant(t.id)
        await loadTenants()
      } catch (e: any) {
        toast.add({ severity: 'error', summary: e.response?.data || 'Error', life: 4000 })
      }
    },
  })
}

function openUserDialog(u?: UserInfo) {
  editingUser.value = u ?? null
  userForm.value = u ? { username: u.username, display_name: u.display_name, password: '', is_admin: u.is_admin }
    : { username: '', display_name: '', password: '', is_admin: false }
  userDialog.value = true
}

async function saveUser() {
  try {
    if (editingUser.value) {
      await adminUpdateUser(editingUser.value.id, { display_name: userForm.value.display_name, is_admin: userForm.value.is_admin })
    } else {
      await adminCreateUser(userForm.value)
    }
    userDialog.value = false
    await loadUsers()
    toast.add({ severity: 'success', summary: 'Saved', life: 2000 })
  } catch (e: any) {
    toast.add({ severity: 'error', summary: e.response?.data || 'Error', life: 3000 })
  }
}

async function toggleUserEnabled(u: UserInfo) {
  await (u.enabled ? adminDisableUser(u.id) : adminEnableUser(u.id))
  await loadUsers()
}

function confirmDeleteUser(u: UserInfo) {
  confirm.require({
    message: `Delete user "${u.username}"?`,
    accept: async () => { await adminDeleteUser(u.id); await loadUsers() },
  })
}

function openResetPasswordDialog(u: UserInfo) {
  resetPasswordTarget.value = u
  newPassword.value = ''
  resetPasswordDialog.value = true
}

async function doResetPassword() {
  if (!resetPasswordTarget.value || !newPassword.value) return
  await adminResetPassword(resetPasswordTarget.value.id, newPassword.value)
  resetPasswordDialog.value = false
  toast.add({ severity: 'success', summary: 'Password reset', life: 2000 })
}
</script>
```

- [ ] **Update `AppSidebar.vue`** — add Admin menu item visible only to admin:

```vue
<!-- Add to navItems or menu list, after Functions: -->
<li v-if="auth.isAdmin">
  <router-link to="/admin" class="...">
    <i class="pi pi-shield" />
    <span>Admin</span>
  </router-link>
</li>
```

Import auth store in AppSidebar:

```typescript
import { useAuthStore } from '../../stores/auth'
const auth = useAuthStore()
```

---

## Task 20: Frontend — Ports view read-only banner

**Files:**
- Modify: `frontend/src/views/PortsView.vue`

- [ ] **Add read-only banner and hide mutating controls for non-admin**

At the top of the template, add:

```vue
<Message v-if="!auth.isAdmin" severity="info" :closable="false" class="mb-4">
  需要开启新端口？请联系管理员。
</Message>
```

Hide action buttons and "create port" button for non-admin users:

```vue
<!-- Wrap create button: -->
<Button v-if="auth.isAdmin" label="New Port" icon="pi pi-plus" @click="openDialog()" />

<!-- Wrap start/stop/delete buttons in DataTable column: -->
<template #body="{ data }">
  <Button ... />  <!-- view/info always visible -->
  <template v-if="auth.isAdmin">
    <Button icon="pi pi-play" ... @click="startPort(data)" />
    <Button icon="pi pi-stop" ... @click="stopPort(data)" />
    <Button icon="pi pi-pencil" ... @click="openDialog(data)" />
    <Button icon="pi pi-trash" ... @click="confirmDelete(data)" />
  </template>
</template>
```

Import auth store:

```typescript
import { useAuthStore } from '../stores/auth'
const auth = useAuthStore()
```

---

## Task 21: Frontend — Mocks view tenant scope + full URL display

**Files:**
- Modify: `frontend/src/views/MocksView.vue`
- Modify: `frontend/src/stores/mocks.ts`

- [ ] **Update `frontend/src/stores/mocks.ts`** — `fetchMocks` now auto-filters by tenant via backend (JWT handles it); just remove any client-side tenant filtering. Also add helper for full mock URL:

No change needed to store actions — the backend now handles tenant filtering automatically. The mocks returned are already scoped.

- [ ] **Update `MocksView.vue`** — add mock address column showing full URL:

Add a computed helper for full URL:

```vue
<script setup lang="ts">
import { useAuthStore } from '../stores/auth'
import { useInfoStore } from '../stores/info'  // or fetch info inline

const auth = useAuthStore()

function mockFullUrl(mock: MockApi, portNumber: number): string {
  const slug = auth.currentTenant?.slug ?? 'unknown'
  const host = serverIp.value  // from existing GET /info logic
  return `http://${host}:${portNumber}/${slug}${mock.path}`
}
</script>
```

In the DataTable, add a "URL" column:

```vue
<Column header="Request URL">
  <template #body="{ data }">
    <code class="text-xs break-all">{{ mockFullUrl(data, getPortNumber(data.port_id)) }}</code>
  </template>
</Column>
```

Where `getPortNumber` looks up the port number from the ports store by `port_id`.

---

## Task 22: Frontend — Logs view tenant scope + App init

**Files:**
- Modify: `frontend/src/views/LogsView.vue`
- Modify: `frontend/src/App.vue`

- [ ] **LogsView** — logs are already scoped by the backend. No store changes needed. Add tenant label to display:

In LogsView, add informational text showing current tenant context:

```vue
<p class="text-sm text-surface-400 mb-3" v-if="auth.currentTenant">
  Showing logs for tenant: <strong>{{ auth.currentTenant.name }}</strong>
</p>
```

- [ ] **App.vue** — initialize auth on mount, redirect if not logged in:

```vue
<script setup lang="ts">
import { onMounted } from 'vue'
import { useAuthStore } from './stores/auth'

const auth = useAuthStore()

onMounted(async () => {
  if (auth.isLoggedIn) {
    await auth.fetchMe()
  }
  // router guards handle redirect to /login if not logged in
})
</script>
```

- [ ] **Build frontend**

```bash
cd frontend && npm run build
```

Expected: no TypeScript errors, dist/ rebuilt.

- [ ] **Commit frontend work** (Tasks 16–22):

```bash
git add frontend/src/
git commit -m "feat: add login page, tenant switcher, admin panel, update views for tenant scope"
```

---

## Task 23: End-to-end verification

- [ ] **Build everything**

```bash
cargo build --release
```

- [ ] **Run full smoke test**

```bash
./target/release/mock serve &
sleep 2
TOKEN=$(curl -s -X POST http://localhost:9999/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"Admin@123"}' | jq -r .token)

echo "=== Create tenant acme ==="
TENANT=$(curl -s -X POST http://localhost:9999/api/v1/admin/tenants \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"name":"Acme Corp","slug":"acme"}')
echo $TENANT | jq .
TENANT_ID=$(echo $TENANT | jq -r .id)

echo "=== Create user alice ==="
curl -s -X POST http://localhost:9999/api/v1/admin/users \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"username\":\"alice\",\"display_name\":\"Alice\",\"password\":\"pass123\",\"is_admin\":false}" | jq .

ALICE_ID=$(curl -s http://localhost:9999/api/v1/admin/users \
  -H "Authorization: Bearer $TOKEN" | jq -r '.[] | select(.username=="alice") | .id')

echo "=== Assign alice to acme tenant ==="
curl -s -X POST http://localhost:9999/api/v1/admin/users/$ALICE_ID/tenants \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"tenant_id\":$TENANT_ID}" | jq .

echo "=== Create port 3001 ==="
PORT=$(curl -s -X POST http://localhost:9999/api/v1/ports \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"port":3001,"label":"test"}')
PORT_ID=$(echo $PORT | jq -r .id)

echo "=== Switch admin to acme tenant ==="
TOKEN=$(curl -s -X POST http://localhost:9999/api/v1/auth/switch-tenant \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"tenant_id\":$TENANT_ID}" | jq -r .token)

echo "=== Create mock on port 3001 ==="
curl -s -X POST http://localhost:9999/api/v1/mocks \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d "{\"port_id\":$PORT_ID,\"name\":\"hello\",\"description\":\"\",\"method\":\"GET\",\"path\":\"/hello\",\"response_status\":200,\"response_body\":\"{\\\"tenant\\\":\\\"acme\\\"}\",\"response_headers\":{},\"response_delay_ms\":0,\"request_params\":{},\"pagination_enabled\":false,\"pagination_page_size\":10,\"pagination_page_param\":\"page\",\"pagination_size_param\":\"page_size\",\"pagination_data_field\":\"\",\"pagination_total_field\":\"\"}" | jq .

echo "=== Start port 3001 ==="
curl -s -X POST http://localhost:9999/api/v1/ports/$PORT_ID/start \
  -H "Authorization: Bearer $TOKEN"
sleep 1

echo "=== Hit mock via tenant URL ==="
curl -s http://localhost:3001/acme/hello
# Expected: {"tenant":"acme"}

echo "=== Port read-only for alice ==="
ALICE_TOKEN=$(curl -s -X POST http://localhost:9999/api/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","password":"pass123"}' | jq -r .token)
curl -s -o /dev/null -w "%{http_code}" -X POST http://localhost:9999/api/v1/ports \
  -H "Authorization: Bearer $ALICE_TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"port":4000}'
# Expected: 403

echo "=== Delete port with mocks is rejected ==="
curl -s -X DELETE http://localhost:9999/api/v1/ports/$PORT_ID \
  -H "Authorization: Bearer $TOKEN" -w "\nHTTP %{http_code}\n"
# Expected: HTTP 409

kill %1
```

- [ ] **Open dashboard in browser**

```bash
cargo run -- serve
# Open http://localhost:9999
# Verify: login page appears, login with admin/Admin@123 works,
# tenant switcher visible in top bar, Admin menu item in sidebar,
# Ports view shows read-only banner for non-admin users
```

- [ ] **Final commit**

```bash
git add -A
git commit -m "feat: multi-tenant dashboard — auth, tenant isolation, mock URL routing, admin panel"
```
