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
    Ok(conn)
}
