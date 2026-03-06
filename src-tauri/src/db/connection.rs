use rusqlite::Connection;
use std::path::Path;
use crate::error::AppResult;

const SCHEMA: &str = include_str!("../../../schema/init.sql");

pub fn init_db(db_path: &Path) -> AppResult<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(db_path)?;
    // Enable WAL mode for better concurrency
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;
    // Inline migration: add auto_titled if missing (idempotent)
    let has_col: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='auto_titled'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);
    if has_col == 0 {
        conn.execute_batch("ALTER TABLE meetings ADD COLUMN auto_titled INTEGER NOT NULL DEFAULT 0;")?;
        log::info!("Migration: added auto_titled column to meetings");
    }
    Ok(conn)
}
