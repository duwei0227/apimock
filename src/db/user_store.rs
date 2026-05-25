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
                use rusqlite::OptionalExtension;
                conn.query_row(
                    "SELECT id, username, display_name, is_admin, enabled, created_at FROM users WHERE id = ?1",
                    rusqlite::params![id],
                    row_to_user,
                )
                .optional()
                .map_err(Into::into)
            })
            .await
            .map_err(AppError::from)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
        let username = username.to_owned();
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, username, display_name, is_admin, enabled, created_at, password_hash \
                     FROM users WHERE username = ?1",
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
                    "INSERT INTO users (username, display_name, password_hash, is_admin) \
                     VALUES (?1, ?2, ?3, ?4)",
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
                    "SELECT id, user_id, tenant_id, is_default, created_at \
                     FROM user_tenants WHERE user_id = ?1",
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
                    "SELECT id, user_id, tenant_id, is_default, created_at \
                     FROM user_tenants WHERE user_id = ?1 AND tenant_id = ?2",
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
