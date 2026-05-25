pub mod log_store;
pub mod mock_store;
pub mod port_store;
pub mod schema;
pub mod tenant_store;
pub mod user_store;

use crate::error::Result;
use tokio_rusqlite::Connection;

pub use log_store::SqliteLogStore;
pub use mock_store::SqliteMockStore;
pub use port_store::SqlitePortStore;
pub use tenant_store::SqliteTenantStore;
pub use user_store::SqliteUserStore;

pub async fn open(path: &str) -> Result<Connection> {
    let conn = Connection::open(path).await?;
    conn.call(|c| {
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        schema::run_migrations(c)?;
        Ok(())
    })
    .await?;
    Ok(conn)
}

pub async fn seed_admin(conn: &Connection) -> Result<()> {
    use crate::traits::UserStore;
    let store = SqliteUserStore::new(conn.clone());
    if store.is_empty().await? {
        let hash = crate::auth::hash_password("Admin@123")
            .map_err(|e| crate::error::AppError::Other(e.to_string()))?;
        store
            .create_user(crate::traits::CreateUserRequest {
                username: "admin".into(),
                display_name: "Administrator".into(),
                password_hash: hash,
                is_admin: true,
            })
            .await?;
        tracing::info!("Seeded default admin account (admin/Admin@123)");
    }
    Ok(())
}
