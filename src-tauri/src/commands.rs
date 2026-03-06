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
        }
    }
}

fn settings_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
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
pub fn create_meeting(title: String, db: State<'_, DbState>) -> Result<Meeting, String> {
    let conn = (*db).0.lock().unwrap();
    models::create_meeting(&conn, &title).map_err(|e| e.to_string())
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
    let conn = (*db).0.lock().unwrap();
    models::update_meeting_audio_path(&conn, meeting_id, &audio_path_str)
        .map_err(|e| e.to_string())?;
    models::update_meeting_status(&conn, meeting_id, "idle").map_err(|e| e.to_string())?;

    Ok(audio_path_str)
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
    let model_path = format!("models/ggml-{}.bin", cfg.whisper_model);
    let asr = WhisperAsr::new(&model_path, &cfg.language);

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

    let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);
    let output = pipeline.run(&transcript_text).map_err(|e| e.to_string())?;

    // Save action items
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
    }

    Ok(PipelineResult {
        clean_transcript: output.clean_transcript,
        summary: output.summary,
        report: output.report,
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
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    *(*config).0.lock().unwrap() = settings;
    Ok(())
}
