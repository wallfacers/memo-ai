use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{State, Manager};
use serde::{Deserialize, Serialize};

use crate::db::models::{self, Meeting, Transcript, ActionItem};
use crate::llm::client::LlmConfig;
use crate::llm::pipeline::Pipeline;
use crate::audio::capture::AudioCapture;
use crate::asr::whisper::WhisperAsr;

pub struct DbState(pub Mutex<rusqlite::Connection>);
pub struct RecordState(pub Mutex<Option<AudioCapture>>);
pub struct ConfigState(pub Mutex<AppConfig>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm_provider: LlmProviderConfig,
    pub whisper_model: String,
    pub language: String,
    pub whisper_cli_path: String,
    pub whisper_model_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    #[serde(rename = "type")]
    pub provider_type: String,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            llm_provider: LlmProviderConfig {
                provider_type: "ollama".into(),
                base_url: "http://localhost:11434".into(),
                model: "llama3".into(),
                api_key: None,
            },
            whisper_model: "base".into(),
            language: "zh".into(),
            whisper_cli_path: "whisper-cli".into(),
            whisper_model_dir: "models".into(),
        }
    }
}

pub fn settings_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("settings.json"))
        .map_err(|e| e.to_string())
}

// ─── Meeting Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_meetings(db: State<'_, DbState>) -> Result<Vec<Meeting>, String> {
    let conn = (*db).0.lock().unwrap();
    models::list_meetings(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_meeting(id: i64, db: State<'_, DbState>) -> Result<Meeting, String> {
    let conn = (*db).0.lock().unwrap();
    models::get_meeting(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_meeting(
    title: String,
    auto_titled: bool,
    db: State<'_, DbState>,
) -> Result<Meeting, String> {
    let conn = (*db).0.lock().unwrap();
    models::create_meeting(&conn, &title, auto_titled).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_meeting(id: i64, db: State<'_, DbState>) -> Result<(), String> {
    let conn = (*db).0.lock().unwrap();
    models::delete_meeting(&conn, id).map_err(|e| e.to_string())
}

// ─── Recording Commands ───────────────────────────────────────────────────────

#[tauri::command]
pub fn start_recording(
    meeting_id: i64,
    db: State<'_, DbState>,
    recorder: State<'_, RecordState>,
) -> Result<(), String> {
    let mut rec_guard = (*recorder).0.lock().unwrap();
    if rec_guard.is_some() {
        return Err("Recording already in progress".into());
    }
    let mut capture = AudioCapture::new().map_err(|e| e.to_string())?;
    capture.start().map_err(|e| e.to_string())?;
    *rec_guard = Some(capture);

    let conn = (*db).0.lock().unwrap();
    models::update_meeting_status(&conn, meeting_id, "recording").map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn stop_recording(
    meeting_id: i64,
    app_handle: tauri::AppHandle,
    db: State<'_, DbState>,
    recorder: State<'_, RecordState>,
) -> Result<String, String> {
    let mut rec_guard = (*recorder).0.lock().unwrap();
    let capture = rec_guard.as_mut().ok_or("No recording in progress")?;

    let data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e: tauri::Error| e.to_string())?
        .join("recordings");
    std::fs::create_dir_all(&data_dir).map_err(|e: std::io::Error| e.to_string())?;

    let filename = format!("meeting_{}.wav", meeting_id);
    let audio_path = capture
        .stop_and_save(&data_dir, &filename)
        .map_err(|e| e.to_string())?;
    *rec_guard = None;

    let audio_path_str = audio_path.to_string_lossy().to_string();
    let end_time = chrono::Utc::now().to_rfc3339();
    let conn = (*db).0.lock().unwrap();
    models::update_meeting_audio_path(&conn, meeting_id, &audio_path_str)
        .map_err(|e| e.to_string())?;
    models::update_meeting_end_time(&conn, meeting_id, &end_time)
        .map_err(|e| e.to_string())?;
    models::update_meeting_status(&conn, meeting_id, "idle").map_err(|e| e.to_string())?;

    Ok(audio_path_str)
}

#[tauri::command]
pub fn rename_meeting(
    id: i64,
    title: String,
    db: State<'_, DbState>,
) -> Result<(), String> {
    let conn = (*db).0.lock().unwrap();
    models::update_meeting_title(&conn, id, &title).map_err(|e| e.to_string())
}

// ─── Transcript Commands ──────────────────────────────────────────────────────

#[tauri::command]
pub fn get_transcripts(
    meeting_id: i64,
    db: State<'_, DbState>,
) -> Result<Vec<Transcript>, String> {
    let conn = (*db).0.lock().unwrap();
    models::get_transcripts(&conn, meeting_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn transcribe_audio(
    audio_path: String,
    meeting_id: i64,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<String, String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let model_path = format!("{}/ggml-{}.bin", cfg.whisper_model_dir, cfg.whisper_model);
    let asr = WhisperAsr::new(&cfg.whisper_cli_path, &model_path, &cfg.language);

    let path = PathBuf::from(&audio_path);
    let segments = asr.transcribe(&path).map_err(|e| e.to_string())?;

    let conn = (*db).0.lock().unwrap();
    let mut full_text = String::new();
    for seg in &segments {
        models::insert_transcript(
            &conn,
            meeting_id,
            seg.speaker.as_deref(),
            &seg.text,
            seg.start,
            seg.confidence,
        )
        .map_err(|e| e.to_string())?;
        full_text.push_str(&seg.text);
        full_text.push(' ');
    }
    Ok(full_text.trim().to_string())
}

// ─── Pipeline Command ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PipelineResult {
    pub clean_transcript: String,
    pub summary: String,
    pub report: String,
    pub generated_title: Option<String>,
}

#[tauri::command]
pub fn run_pipeline(
    meeting_id: i64,
    _app_handle: tauri::AppHandle,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<PipelineResult, String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let llm_config = LlmConfig {
        provider: cfg.llm_provider.provider_type,
        base_url: cfg.llm_provider.base_url,
        model: cfg.llm_provider.model,
        api_key: cfg.llm_provider.api_key,
    };
    let client = llm_config.build_client();

    // Prompts directory — look next to the executable first, then use embedded path
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let prompts_dir = exe_dir.join("prompts");

    // Fallback to dev path
    let prompts_dir = if prompts_dir.exists() {
        prompts_dir
    } else {
        PathBuf::from("prompts")
    };

    // Collect transcript text
    let transcript_text = {
        let conn = (*db).0.lock().unwrap();
        let segments = models::get_transcripts(&conn, meeting_id).map_err(|e| e.to_string())?;
        segments
            .iter()
            .map(|s| {
                if let Some(ref speaker) = s.speaker {
                    format!("{}：{}", speaker, s.text)
                } else {
                    s.text.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    if transcript_text.is_empty() {
        return Err("No transcript available to process".into());
    }

    let auto_titled = {
        let conn = (*db).0.lock().unwrap();
        models::get_meeting(&conn, meeting_id)
            .map(|m| m.auto_titled)
            .unwrap_or(false)
    };

    let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);
    let output = pipeline.run(&transcript_text, auto_titled).map_err(|e| e.to_string())?;

    // Save action items, structure, summary/report, and title atomically in one lock
    {
        let conn = (*db).0.lock().unwrap();
        for item in &output.action_items {
            models::insert_action_item(
                &conn,
                meeting_id,
                &item.task,
                item.owner.as_deref(),
                item.deadline.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        }

        // Save structure
        models::upsert_meeting_structure(
            &conn,
            meeting_id,
            output.structure.topic.as_deref(),
            &output.structure.participants,
            &output.structure.key_points,
            &output.structure.decisions,
            &output.structure.risks,
        )
        .map_err(|e| e.to_string())?;

        // Update meeting summary + report
        models::update_meeting_summary_report(&conn, meeting_id, &output.summary, &output.report)
            .map_err(|e| e.to_string())?;

        // Update title if AI generated one (same lock — atomic with status update)
        if let Some(ref title) = output.generated_title {
            models::update_meeting_title(&conn, meeting_id, title)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(PipelineResult {
        clean_transcript: output.clean_transcript,
        summary: output.summary,
        report: output.report,
        generated_title: output.generated_title,
    })
}

// ─── Action Item Commands ─────────────────────────────────────────────────────

#[tauri::command]
pub fn get_action_items(
    meeting_id: i64,
    db: State<'_, DbState>,
) -> Result<Vec<ActionItem>, String> {
    let conn = (*db).0.lock().unwrap();
    models::get_action_items(&conn, meeting_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_action_item_status(
    id: i64,
    status: String,
    db: State<'_, DbState>,
) -> Result<(), String> {
    let conn = (*db).0.lock().unwrap();
    models::update_action_item_status(&conn, id, &status).map_err(|e| e.to_string())
}

// ─── Export / Search Commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn export_report(
    meeting_id: i64,
    path: String,
    db: State<'_, DbState>,
) -> Result<(), String> {
    let conn = (*db).0.lock().unwrap();

    let (title, start_time): (String, String) = conn.query_row(
        "SELECT title, start_time FROM meetings WHERE id = ?1",
        rusqlite::params![meeting_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| e.to_string())?;

    let (summary, report): (Option<String>, Option<String>) = conn.query_row(
        "SELECT summary, report FROM meetings WHERE id = ?1",
        rusqlite::params![meeting_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare(
        "SELECT task, owner, deadline, status FROM action_items WHERE meeting_id = ?1 ORDER BY id"
    ).map_err(|e| e.to_string())?;

    let action_lines: Vec<String> = stmt.query_map(
        rusqlite::params![meeting_id],
        |row| {
            let task: String = row.get(0)?;
            let owner: Option<String> = row.get(1)?;
            let deadline: Option<String> = row.get(2)?;
            let status: String = row.get(3)?;
            Ok((task, owner, deadline, status))
        },
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .map(|(task, owner, deadline, status)| {
        let checkbox = if status == "done" { "[x]" } else { "[ ]" };
        let meta = match (owner, deadline) {
            (Some(o), Some(d)) => format!("（{} / {}）", o, d),
            (Some(o), None) => format!("（{}）", o),
            (None, Some(d)) => format!("（{}）", d),
            (None, None) => String::new(),
        };
        format!("- {} {}{}", checkbox, task, meta)
    })
    .collect();

    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", title));
    md.push_str(&format!("**日期：** {}\n\n", start_time));

    md.push_str("## 会议总结\n\n");
    md.push_str(summary.as_deref().unwrap_or("（暂无总结）"));
    md.push_str("\n\n");

    md.push_str("## 行动项\n\n");
    if action_lines.is_empty() {
        md.push_str("（暂无行动项）\n");
    } else {
        for line in &action_lines {
            md.push_str(line);
            md.push('\n');
        }
    }
    md.push('\n');

    md.push_str("## 完整报告\n\n");
    md.push_str(report.as_deref().unwrap_or("（暂无报告）"));
    md.push('\n');

    std::fs::write(&path, md).map_err(|e| format!("Write file failed: {}", e))?;
    log::info!("Report exported to: {}", path);
    Ok(())
}

#[tauri::command]
pub fn search_meetings(
    query: String,
    db: State<'_, DbState>,
) -> Result<Vec<Meeting>, String> {
    let conn = (*db).0.lock().unwrap();
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT id, title, start_time, end_time, status, summary, report, audio_path, auto_titled, created_at, updated_at
         FROM meetings
         WHERE title LIKE ?1 OR summary LIKE ?1
         ORDER BY start_time DESC"
    ).map_err(|e| e.to_string())?;

    let meetings = stmt.query_map(
        rusqlite::params![pattern],
        |row| {
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
        },
    ).map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();

    Ok(meetings)
}

// ─── Settings Commands ────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_settings(config: State<'_, ConfigState>) -> Result<AppConfig, String> {
    Ok((*config).0.lock().unwrap().clone())
}

#[tauri::command]
pub fn save_settings(
    settings: AppConfig,
    config: State<'_, ConfigState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let path = settings_path(&app_handle)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    // Atomic write: write to a temp file first, then rename (rename is atomic on same filesystem)
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, &json).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp_path, &path).map_err(|e| e.to_string())?;
    *(*config).0.lock().unwrap() = settings;
    Ok(())
}
