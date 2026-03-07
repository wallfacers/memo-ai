use rusqlite::{Connection, params, Row};
use serde::{Deserialize, Serialize};
use crate::error::AppResult;

// ─── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Meeting {
    pub id: i64,
    pub title: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub status: String,
    pub summary: Option<String>,
    pub report: Option<String>,
    pub audio_path: Option<String>,
    pub auto_titled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transcript {
    pub id: i64,
    pub meeting_id: i64,
    pub speaker: Option<String>,
    pub text: String,
    pub timestamp: f64,
    pub confidence: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionItem {
    pub id: i64,
    pub meeting_id: i64,
    pub task: String,
    pub owner: Option<String>,
    pub deadline: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(dead_code)]
pub struct MeetingStructure {
    pub id: i64,
    pub meeting_id: i64,
    pub topic: Option<String>,
    pub participants: Vec<String>,
    pub key_points: Vec<String>,
    pub decisions: Vec<String>,
    pub risks: Vec<String>,
    pub created_at: String,
}

// ─── Meeting CRUD ─────────────────────────────────────────────────────────────

fn row_to_meeting(row: &Row<'_>) -> rusqlite::Result<Meeting> {
    let auto_titled_int: i64 = row.get(8)?;
    Ok(Meeting {
        id: row.get(0)?,
        title: row.get(1)?,
        start_time: row.get(2)?,
        end_time: row.get(3)?,
        status: row.get(4)?,
        summary: row.get(5)?,
        report: row.get(6)?,
        audio_path: row.get(7)?,
        auto_titled: auto_titled_int != 0,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn create_meeting(conn: &Connection, title: &str, auto_titled: bool) -> AppResult<Meeting> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO meetings (title, start_time, status, auto_titled, created_at, updated_at) VALUES (?1, ?2, 'idle', ?3, ?2, ?2)",
        params![title, now, auto_titled as i64],
    )?;
    let id = conn.last_insert_rowid();
    get_meeting(conn, id)
}

pub fn update_meeting_title(conn: &Connection, id: i64, title: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET title = ?1, auto_titled = 0, updated_at = ?2 WHERE id = ?3",
        params![title, now, id],
    )?;
    Ok(())
}

pub fn get_meeting(conn: &Connection, id: i64) -> AppResult<Meeting> {
    let meeting = conn.query_row(
        "SELECT id, title, start_time, end_time, status, summary, report, audio_path, auto_titled, created_at, updated_at FROM meetings WHERE id = ?1",
        params![id],
        row_to_meeting,
    )?;
    Ok(meeting)
}

pub fn list_meetings(conn: &Connection) -> AppResult<Vec<Meeting>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, start_time, end_time, status, summary, report, audio_path, auto_titled, created_at, updated_at FROM meetings ORDER BY created_at DESC"
    )?;
    let rows = stmt.query_map([], row_to_meeting)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn update_meeting_status(conn: &Connection, id: i64, status: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status, now, id],
    )?;
    Ok(())
}

pub fn update_meeting_end_time(conn: &Connection, id: i64, end_time: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET end_time = ?1, updated_at = ?2 WHERE id = ?3",
        params![end_time, now, id],
    )?;
    Ok(())
}

pub fn update_meeting_audio_path(conn: &Connection, id: i64, audio_path: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET audio_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![audio_path, now, id],
    )?;
    Ok(())
}

pub fn update_meeting_summary_report(
    conn: &Connection,
    id: i64,
    summary: &str,
    report: &str,
) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET summary = ?1, report = ?2, status = 'completed', updated_at = ?3 WHERE id = ?4",
        params![summary, report, now, id],
    )?;
    Ok(())
}

pub fn update_meeting_summary(conn: &Connection, id: i64, summary: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET summary = ?1, updated_at = ?2 WHERE id = ?3",
        params![summary, now, id],
    )?;
    Ok(())
}

pub fn delete_meeting(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM meetings WHERE id = ?1", params![id])?;
    Ok(())
}

// ─── Transcript CRUD ──────────────────────────────────────────────────────────

pub fn insert_transcript(
    conn: &Connection,
    meeting_id: i64,
    speaker: Option<&str>,
    text: &str,
    timestamp: f64,
    confidence: Option<f64>,
) -> AppResult<i64> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO transcripts (meeting_id, speaker, text, timestamp, confidence, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![meeting_id, speaker, text, timestamp, confidence, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_transcripts(conn: &Connection, meeting_id: i64) -> AppResult<Vec<Transcript>> {
    let mut stmt = conn.prepare(
        "SELECT id, meeting_id, speaker, text, timestamp, confidence, created_at FROM transcripts WHERE meeting_id = ?1 ORDER BY timestamp ASC"
    )?;
    let rows = stmt.query_map(params![meeting_id], |row| {
        Ok(Transcript {
            id: row.get(0)?,
            meeting_id: row.get(1)?,
            speaker: row.get(2)?,
            text: row.get(3)?,
            timestamp: row.get(4)?,
            confidence: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// ─── Action Item CRUD ─────────────────────────────────────────────────────────

pub fn insert_action_item(
    conn: &Connection,
    meeting_id: i64,
    task: &str,
    owner: Option<&str>,
    deadline: Option<&str>,
) -> AppResult<i64> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO action_items (meeting_id, task, owner, deadline, status, created_at) VALUES (?1, ?2, ?3, ?4, 'pending', ?5)",
        params![meeting_id, task, owner, deadline, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_action_items(conn: &Connection, meeting_id: i64) -> AppResult<Vec<ActionItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, meeting_id, task, owner, deadline, status, created_at FROM action_items WHERE meeting_id = ?1 ORDER BY created_at ASC"
    )?;
    let rows = stmt.query_map(params![meeting_id], |row| {
        Ok(ActionItem {
            id: row.get(0)?,
            meeting_id: row.get(1)?,
            task: row.get(2)?,
            owner: row.get(3)?,
            deadline: row.get(4)?,
            status: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn update_action_item_status(conn: &Connection, id: i64, status: &str) -> AppResult<()> {
    conn.execute(
        "UPDATE action_items SET status = ?1 WHERE id = ?2",
        params![status, id],
    )?;
    Ok(())
}

// ─── Meeting Structure CRUD ───────────────────────────────────────────────────

pub fn upsert_meeting_structure(
    conn: &Connection,
    meeting_id: i64,
    topic: Option<&str>,
    participants: &[String],
    key_points: &[String],
    decisions: &[String],
    risks: &[String],
) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO meeting_structures (meeting_id, topic, participants, key_points, decisions, risks, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            meeting_id,
            topic,
            serde_json::to_string(participants)?,
            serde_json::to_string(key_points)?,
            serde_json::to_string(decisions)?,
            serde_json::to_string(risks)?,
            now
        ],
    )?;
    Ok(())
}

// ─── Pipeline Intermediate State ──────────────────────────────────────────────

pub fn update_clean_transcript(conn: &Connection, id: i64, text: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET clean_transcript = ?1, updated_at = ?2 WHERE id = ?3",
        params![text, now, id],
    )?;
    Ok(())
}

pub fn update_organized_transcript(conn: &Connection, id: i64, text: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET organized_transcript = ?1, updated_at = ?2 WHERE id = ?3",
        params![text, now, id],
    )?;
    Ok(())
}

pub fn get_pipeline_intermediates(
    conn: &Connection,
    id: i64,
) -> AppResult<(Option<String>, Option<String>)> {
    let result = conn.query_row(
        "SELECT clean_transcript, organized_transcript FROM meetings WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(result)
}

#[allow(dead_code)]
pub fn delete_action_items_for_meeting(conn: &Connection, meeting_id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM action_items WHERE meeting_id = ?1",
        params![meeting_id],
    )?;
    Ok(())
}


/// Clear pipeline results from a given stage onward.
/// from_stage: 1=clean, 2=organized, 3=structure, 4=summary, 5=actions, 6=report
pub fn clear_pipeline_from_stage(conn: &Connection, meeting_id: i64, from_stage: u32) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    if from_stage <= 1 {
        conn.execute(
            "UPDATE meetings SET clean_transcript = NULL, organized_transcript = NULL,
             summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
        conn.execute("DELETE FROM meeting_structures WHERE meeting_id = ?1", params![meeting_id])?;
    } else if from_stage <= 2 {
        conn.execute(
            "UPDATE meetings SET organized_transcript = NULL, summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
        conn.execute("DELETE FROM meeting_structures WHERE meeting_id = ?1", params![meeting_id])?;
    } else if from_stage <= 3 {
        conn.execute(
            "UPDATE meetings SET summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
        conn.execute("DELETE FROM meeting_structures WHERE meeting_id = ?1", params![meeting_id])?;
    } else if from_stage <= 4 {
        conn.execute(
            "UPDATE meetings SET summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
    } else if from_stage <= 5 {
        conn.execute(
            "UPDATE meetings SET report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
    } else {
        // from_stage == 6: only clear report
        conn.execute(
            "UPDATE meetings SET report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
    }
    Ok(())
}
