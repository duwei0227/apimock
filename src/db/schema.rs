use rusqlite::Connection;
const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_schema_migrations", "
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version    TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
    "),
    ("0002_port_configs", "
        CREATE TABLE IF NOT EXISTS port_configs (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            port       INTEGER NOT NULL UNIQUE,
            label      TEXT    NOT NULL DEFAULT '',
            enabled    INTEGER NOT NULL DEFAULT 1,
            created_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
    "),
    ("0003_mock_apis", "
        CREATE TABLE IF NOT EXISTS mock_apis (
            id                INTEGER PRIMARY KEY AUTOINCREMENT,
            port_id           INTEGER NOT NULL REFERENCES port_configs(id) ON DELETE CASCADE,
            name              TEXT    NOT NULL,
            description       TEXT    NOT NULL DEFAULT '',
            method            TEXT    NOT NULL DEFAULT 'ANY',
            path              TEXT    NOT NULL,
            request_schema    TEXT,
            response_status   INTEGER NOT NULL DEFAULT 200,
            response_headers  TEXT    NOT NULL DEFAULT '{}',
            response_body     TEXT    NOT NULL DEFAULT '',
            response_delay_ms INTEGER NOT NULL DEFAULT 0,
            enabled           INTEGER NOT NULL DEFAULT 1,
            created_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            updated_at        TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            UNIQUE(port_id, method, path)
        );
    "),
    ("0004_request_logs", "
        CREATE TABLE IF NOT EXISTS request_logs (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            mock_api_id     INTEGER REFERENCES mock_apis(id) ON DELETE SET NULL,
            port            INTEGER NOT NULL,
            method          TEXT    NOT NULL,
            path            TEXT    NOT NULL,
            query_string    TEXT,
            request_headers TEXT    NOT NULL DEFAULT '{}',
            request_body    TEXT,
            response_status INTEGER NOT NULL,
            response_body   TEXT,
            duration_ms     INTEGER NOT NULL DEFAULT 0,
            created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        CREATE INDEX IF NOT EXISTS idx_request_logs_mock_api_id ON request_logs(mock_api_id);
        CREATE INDEX IF NOT EXISTS idx_request_logs_created_at  ON request_logs(created_at DESC);
    "),
    ("0005_system_logs", "
        CREATE TABLE IF NOT EXISTS system_logs (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            level      TEXT NOT NULL,
            target     TEXT NOT NULL DEFAULT '',
            message    TEXT NOT NULL,
            fields     TEXT,
            created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );
        CREATE INDEX IF NOT EXISTS idx_system_logs_level      ON system_logs(level);
        CREATE INDEX IF NOT EXISTS idx_system_logs_created_at ON system_logs(created_at DESC);
    "),
    ("0006_request_logs_ip_resp_headers", "
        ALTER TABLE request_logs ADD COLUMN client_ip TEXT;
        ALTER TABLE request_logs ADD COLUMN response_headers TEXT NOT NULL DEFAULT '{}';
    "),
    ("0007_port_runtime_status", "
        ALTER TABLE port_configs ADD COLUMN running   INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE port_configs ADD COLUMN owner_pid INTEGER;
    "),
    ("0008_mock_filter_pagination", "
        ALTER TABLE mock_apis ADD COLUMN response_filter_enabled INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE mock_apis ADD COLUMN pagination_enabled       INTEGER NOT NULL DEFAULT 0;
        ALTER TABLE mock_apis ADD COLUMN pagination_page_size     INTEGER NOT NULL DEFAULT 10;
    "),
    ("0009_mock_request_params", "
        ALTER TABLE mock_apis ADD COLUMN request_params TEXT NOT NULL DEFAULT '{}';
    "),
    ("0010_pagination_params", "
        ALTER TABLE mock_apis ADD COLUMN pagination_page_param TEXT NOT NULL DEFAULT 'page';
        ALTER TABLE mock_apis ADD COLUMN pagination_size_param TEXT NOT NULL DEFAULT 'page_size';
        ALTER TABLE mock_apis ADD COLUMN pagination_data_field TEXT NOT NULL DEFAULT '';
    "),
    ("0011_pagination_total_field", "
        ALTER TABLE mock_apis ADD COLUMN pagination_total_field TEXT NOT NULL DEFAULT '';
    "),
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
            id                      INTEGER PRIMARY KEY AUTOINCREMENT,
            port_id                 INTEGER NOT NULL REFERENCES port_configs(id) ON DELETE CASCADE,
            tenant_id               INTEGER NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
            name                    TEXT    NOT NULL,
            description             TEXT    NOT NULL DEFAULT '',
            method                  TEXT    NOT NULL DEFAULT 'ANY',
            path                    TEXT    NOT NULL,
            request_schema          TEXT,
            response_status         INTEGER NOT NULL DEFAULT 200,
            response_headers        TEXT    NOT NULL DEFAULT '{}',
            response_body           TEXT    NOT NULL DEFAULT '',
            response_delay_ms       INTEGER NOT NULL DEFAULT 0,
            enabled                 INTEGER NOT NULL DEFAULT 1,
            created_at              TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            updated_at              TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            response_filter_enabled INTEGER NOT NULL DEFAULT 0,
            pagination_enabled      INTEGER NOT NULL DEFAULT 0,
            pagination_page_size    INTEGER NOT NULL DEFAULT 10,
            request_params          TEXT    NOT NULL DEFAULT '{}',
            pagination_page_param   TEXT    NOT NULL DEFAULT 'page',
            pagination_size_param   TEXT    NOT NULL DEFAULT 'page_size',
            pagination_data_field   TEXT    NOT NULL DEFAULT '',
            pagination_total_field  TEXT    NOT NULL DEFAULT '',
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
];

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    // Ensure the migrations table itself exists first.
    conn.execute_batch(MIGRATIONS[0].1)?;

    for (version, sql) in &MIGRATIONS[1..] {
        let already_applied: bool = conn.query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
            rusqlite::params![version],
            |row| row.get::<_, i64>(0),
        )? > 0;

        if !already_applied {
            conn.execute_batch(sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                rusqlite::params![version],
            )?;
        }
    }
    Ok(())
}
