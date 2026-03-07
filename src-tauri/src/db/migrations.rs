use rusqlite::Connection;
use crate::error::AppResult;

/// Current schema version. Increment when adding migrations.
const CURRENT_VERSION: u32 = 3;

pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    log::info!("DB schema version: {} -> target: {}", version, CURRENT_VERSION);

    if version < 1 {
        migrate_v1(conn)?;
    }
    if version < 2 {
        migrate_v2(conn)?;
    }
    if version < 3 {
        migrate_v3(conn)?;
    }

    conn.execute_batch(&format!("PRAGMA user_version = {}", CURRENT_VERSION))?;
    Ok(())
}

/// v1: initial schema (already created via init.sql, mark as migrated)
fn migrate_v1(_conn: &Connection) -> AppResult<()> {
    log::info!("DB migration: v1 (baseline, no-op)");
    Ok(())
}

/// v2: add auto_titled column to meetings
fn migrate_v2(conn: &Connection) -> AppResult<()> {
    log::info!("DB migration: v2 - add auto_titled to meetings");
    let has_col: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='auto_titled'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);
    if has_col == 0 {
        conn.execute_batch(
            "ALTER TABLE meetings ADD COLUMN auto_titled INTEGER NOT NULL DEFAULT 0;"
        )?;
    }
    Ok(())
}

/// v3: add pipeline intermediate columns to meetings
fn migrate_v3(conn: &Connection) -> AppResult<()> {
    log::info!("DB migration: v3 - add clean_transcript / organized_transcript to meetings");
    for col in &["clean_transcript", "organized_transcript"] {
        let has_col: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='{}'", col),
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        if has_col == 0 {
            conn.execute_batch(
                &format!("ALTER TABLE meetings ADD COLUMN {} TEXT;", col)
            )?;
        }
    }
    Ok(())
}
