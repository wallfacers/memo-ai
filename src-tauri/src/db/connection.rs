use rusqlite::Connection;
use std::path::Path;
use crate::error::AppResult;
use crate::db::migrations::run_migrations;

const SCHEMA: &str = include_str!("../../../schema/init.sql");

pub fn init_db(db_path: &Path) -> AppResult<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    // Create base tables if they don't exist
    conn.execute_batch(SCHEMA)?;
    // Apply versioned migrations
    run_migrations(&conn)?;
    Ok(conn)
}
