use rusqlite::Connection;

use crate::error::DbError;

pub const SCHEMA_VERSION: u32 = 3;

pub fn initialize(connection: &Connection) -> Result<(), DbError> {
    let current_version = stored_version(connection)?;

    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS schema_meta (
          singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
          schema_version INTEGER NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );

        INSERT INTO schema_meta (singleton, schema_version, created_at, updated_at)
        VALUES (1, 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(singleton) DO UPDATE
          SET schema_version = excluded.schema_version,
              updated_at = CURRENT_TIMESTAMP;

        CREATE TABLE IF NOT EXISTS installed_packages (
          pkgname TEXT NOT NULL,
          epoch INTEGER NOT NULL DEFAULT 0,
          pkgver TEXT NOT NULL,
          pkgrel INTEGER NOT NULL,
          arch TEXT,
          package_kind TEXT NOT NULL DEFAULT 'normal',
          variant_id TEXT,
          install_reason TEXT NOT NULL DEFAULT 'explicit',
          source_kind TEXT NOT NULL DEFAULT 'local_recipe',
          source_ref TEXT,
          remote_name TEXT,
          channel TEXT,
          state_id TEXT,
          activation_backend TEXT,
          repo_commit TEXT,
          payload_sha256 TEXT,
          installed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          manifest_hash TEXT,
          pinned_version TEXT,
          held INTEGER NOT NULL DEFAULT 0,
          hold_source TEXT
        );

        CREATE INDEX IF NOT EXISTS installed_packages_name_idx
          ON installed_packages(pkgname, arch);

        CREATE TABLE IF NOT EXISTS package_files (
          pkgname TEXT NOT NULL,
          arch TEXT,
          path TEXT NOT NULL,
          path_kind TEXT NOT NULL,
          sha256 TEXT,
          size INTEGER NOT NULL DEFAULT 0,
          mode INTEGER NOT NULL DEFAULT 0,
          link_target TEXT,
          is_conffile INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (pkgname, path)
        );

        CREATE INDEX IF NOT EXISTS package_files_path_idx
          ON package_files(path);

        CREATE TABLE IF NOT EXISTS package_dependencies (
          pkgname TEXT NOT NULL,
          dependency_name TEXT NOT NULL,
          dependency_kind TEXT NOT NULL,
          raw_expr TEXT NOT NULL,
          is_weak INTEGER NOT NULL DEFAULT 0,
          provider_group TEXT,
          PRIMARY KEY (pkgname, dependency_name, dependency_kind, raw_expr)
        );

        CREATE INDEX IF NOT EXISTS package_dependencies_name_idx
          ON package_dependencies(dependency_name);
        ",
    )?;
    if current_version < 2 {
        ensure_installed_packages_column(
            connection,
            "pinned_version",
            "ALTER TABLE installed_packages ADD COLUMN pinned_version TEXT",
        )?;
        ensure_installed_packages_column(
            connection,
            "held",
            "ALTER TABLE installed_packages ADD COLUMN held INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_installed_packages_column(
            connection,
            "hold_source",
            "ALTER TABLE installed_packages ADD COLUMN hold_source TEXT",
        )?;
    }
    if current_version < 3 {
        ensure_package_files_column(
            connection,
            "is_conffile",
            "ALTER TABLE package_files ADD COLUMN is_conffile INTEGER NOT NULL DEFAULT 0",
        )?;
    }
    connection.pragma_update(None, "user_version", SCHEMA_VERSION)?;

    Ok(())
}

pub fn current_version(connection: &Connection) -> Result<u32, DbError> {
    stored_version(connection)
}

fn stored_version(connection: &Connection) -> Result<u32, DbError> {
    Ok(connection.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn ensure_installed_packages_column(
    connection: &Connection,
    column_name: &str,
    alter_sql: &str,
) -> Result<(), DbError> {
    ensure_table_column(connection, "installed_packages", column_name, alter_sql)
}

fn ensure_package_files_column(
    connection: &Connection,
    column_name: &str,
    alter_sql: &str,
) -> Result<(), DbError> {
    ensure_table_column(connection, "package_files", column_name, alter_sql)
}

fn ensure_table_column(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    alter_sql: &str,
) -> Result<(), DbError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    let has_column = columns
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .any(|name| name == column_name);

    if !has_column {
        connection.execute(alter_sql, [])?;
    }

    Ok(())
}
