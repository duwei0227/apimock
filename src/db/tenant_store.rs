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
        .get::<_, String>(5)
        .ok()
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .unwrap_or_default();
    Ok(Tenant {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        mock_count: row.get(4)?,
        created_at,
    })
}

#[async_trait]
impl TenantStore for SqliteTenantStore {
    async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.conn
            .call(|conn| {
                let mut stmt = conn.prepare(
                    "SELECT t.id, t.name, t.slug, t.enabled,
                            (SELECT COUNT(*) FROM mock_apis WHERE tenant_id = t.id) AS mock_count,
                            t.created_at
                     FROM tenants t ORDER BY t.id",
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
                    "SELECT t.id, t.name, t.slug, t.enabled,
                            (SELECT COUNT(*) FROM mock_apis WHERE tenant_id = t.id) AS mock_count,
                            t.created_at
                     FROM tenants t WHERE t.id = ?1",
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
                    "SELECT t.id, t.name, t.slug, t.enabled,
                            (SELECT COUNT(*) FROM mock_apis WHERE tenant_id = t.id) AS mock_count,
                            t.created_at
                     FROM tenants t WHERE t.slug = ?1",
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
                    "SELECT t.id, t.name, t.slug, t.enabled,
                            (SELECT COUNT(*) FROM mock_apis WHERE tenant_id = t.id) AS mock_count,
                            t.created_at
                     FROM tenants t WHERE t.id = ?1",
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
                    "SELECT t.id, t.name, t.slug, t.enabled,
                            (SELECT COUNT(*) FROM mock_apis WHERE tenant_id = t.id) AS mock_count,
                            t.created_at
                     FROM tenants t WHERE t.id = ?1",
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
                // request_logs.tenant_id has no ON DELETE action, so NULL it out first.
                conn.execute(
                    "UPDATE request_logs SET tenant_id = NULL WHERE tenant_id = ?1",
                    rusqlite::params![id],
                )?;
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

    async fn has_mocks_for_port(&self, port_id: i64) -> Result<bool> {
        self.conn
            .call(move |conn| {
                let count: i64 = conn.query_row(
                    "SELECT COUNT(*) FROM mock_apis WHERE port_id = ?1",
                    rusqlite::params![port_id],
                    |row| row.get(0),
                )?;
                Ok(count > 0)
            })
            .await
            .map_err(AppError::from)
    }
}
