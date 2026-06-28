use std::path::Path;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;

use crate::error::AppResult;

pub type DbPool = r2d2::Pool<SqliteConnectionManager>;
pub type PooledConn = r2d2::PooledConnection<SqliteConnectionManager>;

// Embed the versioned SQL migrations at compile time.
mod embedded {
    refinery::embed_migrations!("src/db/migrations");
}

/// Build a connection pool against the local SQLite file and run migrations.
///
/// WAL mode + a busy timeout keep the UI command path and the background sync
/// worker from blocking each other under concurrent access.
pub fn init_pool(db_path: &Path) -> AppResult<DbPool> {
    let manager = SqliteConnectionManager::file(db_path).with_init(|conn| {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;",
        )
    });
    let pool = r2d2::Pool::builder()
        .max_size(8)
        .build(manager)
        .map_err(crate::error::AppError::Pool)?;

    run_migrations(&pool)?;
    Ok(pool)
}

fn run_migrations(pool: &DbPool) -> AppResult<()> {
    let mut conn: PooledConn = pool.get()?;
    embedded::migrations::runner()
        .run(&mut *conn)
        .map_err(|e| crate::error::AppError::Other(format!("migration failed: {e}")))?;
    Ok(())
}

/// Convenience for an in-memory database (used by tests).
#[allow(dead_code)]
pub fn init_memory_pool() -> AppResult<DbPool> {
    let manager = SqliteConnectionManager::memory();
    let pool = r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .map_err(crate::error::AppError::Pool)?;
    run_migrations(&pool)?;
    Ok(pool)
}

#[allow(dead_code)]
pub fn open_file(path: &Path) -> AppResult<Connection> {
    Ok(Connection::open(path)?)
}
