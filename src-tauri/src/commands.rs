use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{State, Manager, Emitter};
use serde::{Deserialize, Serialize};

use crate::db::models::{self, Meeting, Transcript, ActionItem};
use crate::llm::client::LlmConfig;
use crate::llm::pipeline::{Pipeline, PipelineOutput};
use crate::audio::capture::AudioCapture;
use crate::asr::build_asr;
use crate::asr::{StreamingAsrSession, StreamingSegment};

pub struct DbState(pub Mutex<rusqlite::Connection>);
pub struct RecordState(pub Mutex<Option<AudioCapture>>);
pub struct ConfigState(pub Mutex<AppConfig>);

pub struct FunAsrState(pub Mutex<Option<FunAsrSessionHolder>>);

pub struct FunAsrSessionHolder {
    pub session: Box<dyn StreamingAsrSession>,
    pub collected_finals: Vec<StreamingSegment>,
    // 持有 FunAsrServer，确保进程在 session 整个生命周期内存活
    // session drop 时自动触发 FunAsrServer::drop() → 进程终止
    server: Option<crate::process::FunAsrServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm_provider: LlmProviderConfig,
    pub whisper_model: String,
    pub language: String,
    pub whisper_cli_path: String,
    pub whisper_model_dir: String,
    #[serde(default = "default_asr_provider")]
    pub asr_provider: String,
    #[serde(default)]
    pub aliyun_asr_app_key: String,
    #[serde(default)]
    pub aliyun_asr_access_key_id: String,
    #[serde(default)]
    pub aliyun_asr_access_key_secret: String,
    #[serde(default)]
    pub funasr_ws_url: String,
    #[serde(default)]
    pub funasr_server_path: String,
    #[serde(default = "default_funasr_port")]
    pub funasr_port: u16,
    #[serde(default)]
    pub funasr_enabled: bool,
}

fn default_asr_provider() -> String {
    "local_whisper".into()
}

fn default_funasr_port() -> u16 { 10095 }

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
            whisper_model_dir: String::new(),
            asr_provider: "local_whisper".into(),
            aliyun_asr_app_key: String::new(),
            aliyun_asr_access_key_id: String::new(),
            aliyun_asr_access_key_secret: String::new(),
            funasr_ws_url: String::new(),
            funasr_server_path: String::new(),
            funasr_port: 10095,
            funasr_enabled: false,
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

#[tauri::command]
pub fn update_meeting_summary(
    id: i64,
    summary: String,
    db: State<'_, DbState>,
) -> Result<(), String> {
    let conn = (*db).0.lock().unwrap();
    models::update_meeting_summary(&conn, id, &summary).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn regenerate_summary(
    meeting_id: i64,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<String, String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let llm_config = LlmConfig {
        provider: cfg.llm_provider.provider_type,
        base_url: cfg.llm_provider.base_url,
        model: cfg.llm_provider.model,
        api_key: cfg.llm_provider.api_key,
    };

    // 复用与 run_pipeline 相同的 prompts_dir 解析逻辑
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let prompts_dir = {
        let exe_adjacent = exe_dir.join("prompts");
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("prompts");
        if exe_adjacent.exists() {
            exe_adjacent
        } else if dev_path.exists() {
            dev_path
        } else {
            PathBuf::from("prompts")
        }
    };

    // 读取转写文本（锁立即释放）
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
        return Err("No transcript available to regenerate summary".into());
    }

    // 在独立 OS 线程中运行 LLM（避免 reqwest::blocking 与 Tokio 冲突）
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let client = llm_config.build_client();
        let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);
        let result = pipeline.stage1_clean(&transcript_text)
            .and_then(|clean| pipeline.stage2_speaker(&clean))
            .and_then(|organized| pipeline.stage4_summary(&organized));
        let _ = tx.send(result);
    });

    let new_summary = rx.await
        .map_err(|_| "LLM thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

    // 写库
    {
        let conn = (*db).0.lock().unwrap();
        models::update_meeting_summary(&conn, meeting_id, &new_summary)
            .map_err(|e| e.to_string())?;
    }

    Ok(new_summary)
}

#[tauri::command]
pub async fn regenerate_summary_stream(
    meeting_id: i64,
    app_handle: tauri::AppHandle,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<(), String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let llm_config = LlmConfig {
        provider: cfg.llm_provider.provider_type,
        base_url: cfg.llm_provider.base_url,
        model: cfg.llm_provider.model,
        api_key: cfg.llm_provider.api_key,
    };

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let prompts_dir = {
        let exe_adjacent = exe_dir.join("prompts");
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("prompts");
        if exe_adjacent.exists() {
            exe_adjacent
        } else if dev_path.exists() {
            dev_path
        } else {
            PathBuf::from("prompts")
        }
    };

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
        let _ = app_handle.emit("summary_error", SummaryErrorEvent {
            message: "No transcript available".into(),
        });
        return Ok(());
    }

    let app_for_cb = app_handle.clone();
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();

    std::thread::spawn(move || {
        let client = llm_config.build_client();
        let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);

        // Stage 1
        let _ = app_for_cb.emit("summary_stage", SummaryStageEvent {
            stage: 1,
            name: "正在清洗文本...".into(),
        });
        let clean = match pipeline.stage1_clean(&transcript_text) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_for_cb.emit("summary_error", SummaryErrorEvent { message: e.to_string() });
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };

        // Stage 2
        let _ = app_for_cb.emit("summary_stage", SummaryStageEvent {
            stage: 2,
            name: "正在整理说话人...".into(),
        });
        let organized = match pipeline.stage2_speaker(&clean) {
            Ok(o) => o,
            Err(e) => {
                let _ = app_for_cb.emit("summary_error", SummaryErrorEvent { message: e.to_string() });
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };

        // Stage 4 开始通知
        let _ = app_for_cb.emit("summary_stage", SummaryStageEvent {
            stage: 4,
            name: "正在生成摘要...".into(),
        });

        // Stage 4 (streaming)
        let app_for_token = app_for_cb.clone();
        let on_token: Box<dyn Fn(&str) + Send> = Box::new(move |token: &str| {
            let _ = app_for_token.emit("summary_chunk", SummaryChunkEvent {
                text: token.to_string(),
            });
        });

        let result = pipeline.stage4_summary_streaming(&organized, on_token);
        match result {
            Ok(summary) => {
                let _ = app_for_cb.emit("summary_done", SummaryDoneEvent {
                    summary: summary.clone(),
                });
                let _ = tx.send(Ok(summary));
            }
            Err(e) => {
                let _ = app_for_cb.emit("summary_error", SummaryErrorEvent { message: e.to_string() });
                let _ = tx.send(Err(e.to_string()));
            }
        }
    });

    // 等待线程完成，写库
    match rx.await {
        Ok(Ok(summary)) => {
            let conn = (*db).0.lock().unwrap();
            models::update_meeting_summary(&conn, meeting_id, &summary)
                .map_err(|e| e.to_string())?;
        }
        Ok(Err(_)) => {}   // 错误已通过 summary_error 事件发出
        Err(_) => {
            let _ = app_handle.emit("summary_error", SummaryErrorEvent {
                message: "Stream thread panicked".into(),
            });
        }
    }

    Ok(())
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
pub async fn transcribe_audio(
    audio_path: String,
    meeting_id: i64,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<String, String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let path = PathBuf::from(&audio_path);

    // 在独立 OS 线程中运行 whisper 子进程（完全脱离 Tokio 上下文，避免运行时冲突）
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let asr = build_asr(&cfg);
        let _ = tx.send(asr.transcribe(&path));
    });
    let segments = rx.await
        .map_err(|_| "ASR thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

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

#[derive(Clone, Serialize)]
pub struct PipelineStageDoneEvent {
    pub stage: u8,
    pub name: String,
    pub summary: String,
}

#[derive(Serialize, Clone)]
pub struct PipelineStageFailed {
    pub stage: u8,
    pub error: String,
}

#[derive(Clone, Serialize)]
pub struct SummaryStageEvent {
    pub stage: u8,
    pub name: String,
}

#[derive(Clone, Serialize)]
pub struct SummaryChunkEvent {
    pub text: String,
}

#[derive(Clone, Serialize)]
pub struct SummaryDoneEvent {
    pub summary: String,
}

#[derive(Clone, Serialize)]
pub struct SummaryErrorEvent {
    pub message: String,
}

#[derive(Serialize)]
pub struct PipelineResult {
    pub clean_transcript: String,
    pub summary: String,
    pub report: String,
    pub generated_title: Option<String>,
}

#[tauri::command]
pub async fn run_pipeline(
    meeting_id: i64,
    app_handle: tauri::AppHandle,
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

    // Prompts directory resolution (in priority order):
    // 1. <exe>/prompts  — production bundle
    // 2. <CARGO_MANIFEST_DIR>/../prompts — dev mode (cargo run / tauri dev)
    // 3. prompts — last-resort relative fallback
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let prompts_dir = {
        let exe_adjacent = exe_dir.join("prompts");
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("prompts");
        if exe_adjacent.exists() {
            exe_adjacent
        } else if dev_path.exists() {
            dev_path
        } else {
            PathBuf::from("prompts")
        }
    };

    // 快速 DB 读取，在 await 前释放锁
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

    // 在独立 OS 线程中运行 LLM pipeline（完全脱离 Tokio 上下文，避免 reqwest::blocking 运行时冲突）
    let (tx, rx) = tokio::sync::oneshot::channel();
    let app_for_cb = app_handle.clone();
    std::thread::spawn(move || {
        let client = llm_config.build_client();
        let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);
        let db_state = app_for_cb.state::<DbState>();

        macro_rules! run_stage {
            ($stage_num:expr, $name:expr, $result:expr, $summary_fn:expr) => {
                match $result {
                    Ok(val) => {
                        app_for_cb.emit("pipeline_stage_done", PipelineStageDoneEvent {
                            stage: $stage_num,
                            name: $name.to_string(),
                            summary: $summary_fn(&val),
                        }).ok();
                        val
                    }
                    Err(e) => {
                        app_for_cb.emit("pipeline_stage_failed", PipelineStageFailed {
                            stage: $stage_num,
                            error: e.to_string(),
                        }).ok();
                        let _ = tx.send(Err(e));
                        return;
                    }
                }
            };
        }

        // Stage 1
        let clean = run_stage!(1, "文本清洗",
            pipeline.stage1_clean(&transcript_text),
            |v: &String| format!("完成（共 {} 字）", v.len()));
        {
            let conn = db_state.0.lock().unwrap();
            if let Err(e) = models::update_clean_transcript(&conn, meeting_id, &clean) {
                log::error!("Stage 1 DB write failed (clean_transcript): {}", e);
            }
        }

        // Stage 2
        let organized = run_stage!(2, "说话人整理",
            pipeline.stage2_speaker(&clean),
            |v: &String| v.chars().take(50).collect::<String>());
        {
            let conn = db_state.0.lock().unwrap();
            if let Err(e) = models::update_organized_transcript(&conn, meeting_id, &organized) {
                log::error!("Stage 2 DB write failed (organized_transcript): {}", e);
            }
        }

        // Stage 3 (infallible)
        let structure = pipeline.stage3_structure(&organized);
        let s3_summary = format!(
            "主题：{} · 参会 {} 人 · {} 项决策",
            structure.topic.as_deref().unwrap_or("未知"),
            structure.participants.len(),
            structure.decisions.len(),
        );
        app_for_cb.emit("pipeline_stage_done", PipelineStageDoneEvent {
            stage: 3,
            name: "结构化提取".to_string(),
            summary: s3_summary,
        }).ok();
        {
            let conn = db_state.0.lock().unwrap();
            let _ = models::upsert_meeting_structure(
                &conn, meeting_id,
                structure.topic.as_deref(),
                &structure.participants,
                &structure.key_points,
                &structure.decisions,
                &structure.risks,
            );
        }

        // Stage 4
        let summary = run_stage!(4, "会议总结",
            pipeline.stage4_summary(&organized),
            |v: &String| v.chars().take(100).collect::<String>());
        {
            let conn = db_state.0.lock().unwrap();
            if let Err(e) = models::update_meeting_summary(&conn, meeting_id, &summary) {
                log::error!("Stage 4 DB write failed (meeting_summary): {}", e);
            }
        }

        // Stage 5 (infallible)
        let action_items = pipeline.stage5_actions(&organized);
        app_for_cb.emit("pipeline_stage_done", PipelineStageDoneEvent {
            stage: 5,
            name: "行动项提取".to_string(),
            summary: format!("共 {} 项行动", action_items.len()),
        }).ok();
        {
            let conn = db_state.0.lock().unwrap();
            for item in &action_items {
                if let Err(e) = models::insert_action_item(
                    &conn, meeting_id,
                    &item.task, item.owner.as_deref(), item.deadline.as_deref(),
                ) {
                    log::error!("Stage 5 DB write failed (insert_action_item): {}", e);
                }
            }
        }

        // Stage 6
        let actions_json = match serde_json::to_string(&action_items) {
            Ok(j) => j,
            Err(e) => {
                app_for_cb.emit("pipeline_stage_failed", PipelineStageFailed {
                    stage: 6,
                    error: e.to_string(),
                }).ok();
                let _ = tx.send(Err(crate::error::AppError::Llm(e.to_string())));
                return;
            }
        };
        let report = run_stage!(6, "报告生成",
            pipeline.stage6_report(&summary, &actions_json),
            |_: &String| "报告已生成，点击查看".to_string());
        {
            let conn = db_state.0.lock().unwrap();
            if let Err(e) = models::update_meeting_summary_report(&conn, meeting_id, &summary, &report) {
                log::error!("Stage 6 DB write failed (meeting_summary_report): {}", e);
            }
        }

        // Stage 7 (optional title)
        let generated_title = if auto_titled {
            match pipeline.stage7_title(&summary) {
                Ok(t) => {
                    let conn = db_state.0.lock().unwrap();
                    let _ = models::update_meeting_title(&conn, meeting_id, &t);
                    Some(t)
                }
                Err(e) => {
                    log::warn!("Stage 7 title generation failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let _ = tx.send(Ok(PipelineOutput {
            clean_transcript: clean,
            structure,
            summary,
            action_items,
            report,
            generated_title,
        }));
    });
    let output = rx.await
        .map_err(|_| "LLM pipeline thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

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

// ─── LLM Test Command ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct LlmTestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: u64,
}

#[tauri::command]
pub async fn test_llm_connection(settings: AppConfig) -> Result<LlmTestResult, String> {
    use std::time::Instant;

    let cfg = LlmConfig {
        provider: settings.llm_provider.provider_type.clone(),
        base_url: settings.llm_provider.base_url.clone(),
        model: settings.llm_provider.model.clone(),
        api_key: settings.llm_provider.api_key.clone(),
    };
    let base_url = settings.llm_provider.base_url.clone();
    let model = settings.llm_provider.model.clone();

    tokio::task::spawn_blocking(move || {
        let client = cfg.build_client();
        let start = Instant::now();

        match client.complete("Hi") {
            Ok(_) => {
                let ms = start.elapsed().as_millis() as u64;
                Ok(LlmTestResult {
                    success: true,
                    message: format!("连接正常 ({}ms)", ms),
                    latency_ms: ms,
                })
            }
            Err(e) => {
                let msg = e.to_string();
                let friendly = if msg.contains("Connection refused") || msg.contains("connect error") {
                    format!("无法连接到 {}，请确认服务已启动", base_url)
                } else if msg.contains("401") || msg.contains("Unauthorized") {
                    "API Key 无效，请检查配置".to_string()
                } else if msg.contains("model") && msg.contains("not found") {
                    format!("模型 '{}' 不存在，请确认模型名称", model)
                } else {
                    msg
                };
                Ok(LlmTestResult {
                    success: false,
                    message: friendly,
                    latency_ms: 0,
                })
            }
        }
    })
    .await
    .map_err(|e| format!("任务执行失败: {}", e))?
}

// ─── ASR Commands ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct WhisperCheckResult {
    pub found: bool,
    pub version: Option<String>,
    pub status: String,
}

#[tauri::command]
pub fn check_whisper_cli(cli_path: String) -> Result<WhisperCheckResult, String> {
    // Step 1: check existence via -h (supported by all whisper-cli versions)
    match std::process::Command::new(&cli_path).arg("-h").output() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(WhisperCheckResult {
                found: false,
                version: None,
                status: "notFound".to_string(),
            });
        }
        Err(_) => {
            return Ok(WhisperCheckResult {
                found: false,
                version: None,
                status: "execFailed".to_string(),
            });
        }
        Ok(_) => {} // executable exists, continue
    }

    // Step 2: try --version to get version string (newer versions support it)
    let version = std::process::Command::new(&cli_path)
        .arg("--version")
        .output()
        .ok()
        .and_then(|out| {
            let text = if !out.stdout.is_empty() {
                String::from_utf8_lossy(&out.stdout).into_owned()
            } else {
                String::from_utf8_lossy(&out.stderr).into_owned()
            };
            let first_line = text.lines().next().unwrap_or("").trim().to_string();
            if first_line.is_empty()
                || first_line.starts_with("error:")
                || first_line.contains("unknown argument")
            {
                None
            } else {
                Some(first_line)
            }
        });

    Ok(WhisperCheckResult {
        found: true,
        version,
        status: "found".to_string(),
    })
}

#[derive(Serialize)]
pub struct AsrTestResult {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub fn test_asr_connection(settings: AppConfig) -> Result<AsrTestResult, String> {
    match settings.asr_provider.as_str() {
        "aliyun" => {
            match crate::asr::aliyun::test_connection(
                &settings.aliyun_asr_app_key,
                &settings.aliyun_asr_access_key_id,
                &settings.aliyun_asr_access_key_secret,
            ) {
                Ok(msg) => Ok(AsrTestResult { success: true, message: msg }),
                Err(e) => Ok(AsrTestResult { success: false, message: e }),
            }
        }
        _ => Ok(AsrTestResult {
            success: false,
            message: "当前 ASR Provider 无需测试".to_string(),
        }),
    }
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

// ─── FunASR Streaming Commands ────────────────────────────────────────────────

#[tauri::command]
pub fn start_funasr_session(
    meeting_id: i64,
    app_handle: tauri::AppHandle,
    config: State<'_, ConfigState>,
    funasr: State<'_, FunAsrState>,
    recorder: State<'_, RecordState>,
) -> Result<(), String> {
    use crate::asr::funasr::FunAsrStreamSession;

    let cfg = (*config).0.lock().unwrap().clone();

    // 检查是否已有 session 在运行，避免重复启动
    {
        let guard = (*funasr).0.lock().unwrap();
        if guard.is_some() {
            return Err("FunASR session already running. Call stop_funasr_session first.".into());
        }
    }

    if !cfg.funasr_enabled {
        return Ok(()); // 未启用，静默跳过
    }

    let server = crate::process::FunAsrServer::start(
        &cfg.funasr_ws_url,
        &cfg.funasr_server_path,
        cfg.funasr_port,
    )
    .map_err(|e| format!("FunASR server start failed: {}", e))?;

    let ws_url = server.ws_url.clone();

    // 事件转发线程：接收 WS 识别结果 → Tauri emit
    let (event_tx, event_rx) = std::sync::mpsc::sync_channel::<StreamingSegment>(128);
    let app_clone = app_handle.clone();
    std::thread::spawn(move || {
        loop {
            match event_rx.recv() {
                Ok(seg) => {
                    let ev = if seg.is_final { "asr_final" } else { "asr_partial" };
                    if let Err(e) = app_clone.emit(ev, &seg) {
                        log::warn!("Failed to emit {} event: {}", ev, e);
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 建立流式 WebSocket 会话
    let session = FunAsrStreamSession::connect(&ws_url, meeting_id, event_tx)
        .map_err(|e| format!("FunASR session connect failed: {}", e))?;

    // 获取音频 sender（直接写入 WS 音频通道，无需经由 trait）
    let audio_sender = session.audio_sender();

    // 注册 PCM 块通道到 AudioCapture
    let (pcm_tx, pcm_rx) = std::sync::mpsc::sync_channel::<Vec<i16>>(64);
    {
        let mut rec_guard = (*recorder).0.lock().unwrap();
        if let Some(ref mut capture) = *rec_guard {
            capture.set_chunk_sender(pcm_tx);
        }
    }

    // 中转线程：Vec<i16> → Vec<u8> (LE) → FunASR audio channel
    std::thread::spawn(move || {
        loop {
            match pcm_rx.recv() {
                Ok(pcm) => {
                    let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
                    if audio_sender.send(Some(bytes)).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let mut funasr_guard = (*funasr).0.lock().unwrap();
    *funasr_guard = Some(FunAsrSessionHolder {
        session: Box::new(session),
        collected_finals: Vec::new(),
        server: Some(server),
    });

    Ok(())
}

#[derive(Serialize)]
pub struct FunAsrStopResult {
    pub segments: Vec<StreamingSegment>,
}

#[tauri::command]
pub fn stop_funasr_session(
    funasr: State<'_, FunAsrState>,
) -> Result<FunAsrStopResult, String> {
    let mut guard = (*funasr).0.lock().unwrap();
    if let Some(ref mut holder) = *guard {
        let segments = holder.session.finish().map_err(|e| e.to_string())?;
        let result = FunAsrStopResult { segments };
        *guard = None;
        Ok(result)
    } else {
        Ok(FunAsrStopResult { segments: Vec::new() })
    }
}

#[derive(Serialize)]
pub struct FunAsrCheckResult {
    pub found: bool,
    pub message: String,
}

#[tauri::command]
pub fn check_funasr_server(server_path: String) -> Result<FunAsrCheckResult, String> {
    match crate::process::funasr_server::check_funasr_server(&server_path) {
        Ok(version) => Ok(FunAsrCheckResult { found: true, message: version }),
        Err(msg) => Ok(FunAsrCheckResult { found: false, message: msg }),
    }
}

// ─── Pipeline Retry ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn retry_pipeline_from_stage(
    meeting_id: i64,
    from_stage: u32,
    app_handle: tauri::AppHandle,
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

    // prompts_dir（与 run_pipeline 相同逻辑）
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let prompts_dir = {
        let exe_adjacent = exe_dir.join("prompts");
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("prompts");
        if exe_adjacent.exists() { exe_adjacent }
        else if dev_path.exists() { dev_path }
        else { PathBuf::from("prompts") }
    };

    // 读取中间结果 + 原始 transcript + auto_titled
    let (clean_opt, organized_opt, raw_transcript, auto_titled) = {
        let db_state = app_handle.state::<DbState>();
        let conn = db_state.0.lock().unwrap();
        let (c, o) = models::get_pipeline_intermediates(&conn, meeting_id)
            .map_err(|e| e.to_string())?;
        let segments = models::get_transcripts(&conn, meeting_id).map_err(|e| e.to_string())?;
        let raw = segments.iter().map(|s| {
            if let Some(ref speaker) = s.speaker {
                format!("{}：{}", speaker, s.text)
            } else {
                s.text.clone()
            }
        }).collect::<Vec<_>>().join("\n");
        let auto_t = models::get_meeting(&conn, meeting_id)
            .map(|m| m.auto_titled).unwrap_or(false);
        (c, o, raw, auto_t)
    };

    // 降级：若所需中间数据缺失，从更早阶段开始
    let actual_from_stage = if from_stage >= 3 && organized_opt.is_none() {
        if clean_opt.is_none() { 1 } else { 2 }
    } else if from_stage >= 2 && clean_opt.is_none() {
        1
    } else {
        from_stage
    };

    // 清除 actual_from_stage 及之后的旧数据
    {
        let db_state = app_handle.state::<DbState>();
        let conn = db_state.0.lock().unwrap();
        models::clear_pipeline_from_stage(&conn, meeting_id, actual_from_stage)
            .map_err(|e| e.to_string())?;
    }

    let (tx, rx) = tokio::sync::oneshot::channel();
    let app_for_thread = app_handle.clone();

    std::thread::spawn(move || {
        let db_state = app_for_thread.state::<DbState>();
        let client = llm_config.build_client();
        let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);

        let mut clean: String = clean_opt.unwrap_or_default();
        let mut organized: String = organized_opt.unwrap_or_default();

        macro_rules! emit_failed {
            ($stage:expr, $err:expr) => {{
                app_for_thread.emit("pipeline_stage_failed", PipelineStageFailed {
                    stage: $stage as u8,
                    error: $err.to_string(),
                }).ok();
                let _ = tx.send(Err(crate::error::AppError::Llm($err.to_string())));
                return;
            }};
        }

        macro_rules! emit_done {
            ($stage:expr, $name:expr, $summary:expr) => {
                app_for_thread.emit("pipeline_stage_done", PipelineStageDoneEvent {
                    stage: $stage as u8,
                    name: $name.to_string(),
                    summary: $summary.to_string(),
                }).ok();
            };
        }

        // Stage 1
        if actual_from_stage <= 1 {
            if raw_transcript.is_empty() {
                emit_failed!(1u8, "No transcript available to process");
            }
            match pipeline.stage1_clean(&raw_transcript) {
                Ok(v) => {
                    emit_done!(1u8, "文本清洗", format!("完成（共 {} 字）", v.len()));
                    let conn = db_state.0.lock().unwrap();
                    if let Err(e) = models::update_clean_transcript(&conn, meeting_id, &v) {
                        log::error!("Retry Stage 1 DB write failed: {}", e);
                    }
                    clean = v;
                }
                Err(e) => emit_failed!(1u8, e),
            }
        }

        // Stage 2
        if actual_from_stage <= 2 {
            match pipeline.stage2_speaker(&clean) {
                Ok(v) => {
                    emit_done!(2u8, "说话人整理", v.chars().take(50).collect::<String>());
                    let conn = db_state.0.lock().unwrap();
                    if let Err(e) = models::update_organized_transcript(&conn, meeting_id, &v) {
                        log::error!("Retry Stage 2 DB write failed: {}", e);
                    }
                    organized = v;
                }
                Err(e) => emit_failed!(2u8, e),
            }
        }

        // Stage 3 (infallible)
        if actual_from_stage <= 3 {
            let structure = pipeline.stage3_structure(&organized);
            let s3_summary = format!(
                "主题：{} · 参会 {} 人 · {} 项决策",
                structure.topic.as_deref().unwrap_or("未知"),
                structure.participants.len(),
                structure.decisions.len(),
            );
            emit_done!(3u8, "结构化提取", s3_summary);
            let conn = db_state.0.lock().unwrap();
            let _ = models::upsert_meeting_structure(
                &conn, meeting_id,
                structure.topic.as_deref(),
                &structure.participants,
                &structure.key_points,
                &structure.decisions,
                &structure.risks,
            );
        }

        // Stage 4: summary
        let summary = if actual_from_stage > 4 {
            // Read existing summary from DB
            let conn = db_state.0.lock().unwrap();
            models::get_meeting(&conn, meeting_id).ok()
                .and_then(|m| m.summary)
                .unwrap_or_default()
        } else {
            match pipeline.stage4_summary(&organized) {
                Ok(v) => {
                    emit_done!(4u8, "会议总结", v.chars().take(100).collect::<String>());
                    let conn = db_state.0.lock().unwrap();
                    if let Err(e) = models::update_meeting_summary(&conn, meeting_id, &v) {
                        log::error!("Retry Stage 4 DB write failed: {}", e);
                    }
                    v
                }
                Err(e) => { emit_failed!(4u8, e); }
            }
        };

        // Stage 5: action items
        let action_items = if actual_from_stage > 5 {
            // Read existing action items from DB
            let conn = db_state.0.lock().unwrap();
            models::get_action_items(&conn, meeting_id)
                .unwrap_or_default()
                .into_iter()
                .map(|a| crate::llm::pipeline::ActionItemRaw {
                    task: a.task,
                    owner: a.owner,
                    deadline: a.deadline,
                })
                .collect::<Vec<_>>()
        } else {
            let items = pipeline.stage5_actions(&organized);
            emit_done!(5u8, "行动项提取", format!("共 {} 项行动", items.len()));
            let conn = db_state.0.lock().unwrap();
            for item in &items {
                if let Err(e) = models::insert_action_item(
                    &conn, meeting_id,
                    &item.task, item.owner.as_deref(), item.deadline.as_deref(),
                ) {
                    log::error!("Retry Stage 5 DB write failed: {}", e);
                }
            }
            items
        };

        // Stage 6
        let actions_json = match serde_json::to_string(&action_items) {
            Ok(j) => j,
            Err(e) => { emit_failed!(6u8, e); }
        };
        let report = match pipeline.stage6_report(&summary, &actions_json) {
            Ok(v) => {
                emit_done!(6u8, "报告生成", "报告已生成，点击查看");
                let conn = db_state.0.lock().unwrap();
                if let Err(e) = models::update_meeting_summary_report(&conn, meeting_id, &summary, &v) {
                    log::error!("Retry Stage 6 DB write failed: {}", e);
                }
                v
            }
            Err(e) => { emit_failed!(6u8, e); }
        };

        // Stage 7 (optional)
        let generated_title = if auto_titled {
            match pipeline.stage7_title(&summary) {
                Ok(t) => {
                    let conn = db_state.0.lock().unwrap();
                    let _ = models::update_meeting_title(&conn, meeting_id, &t);
                    Some(t)
                }
                Err(e) => { log::warn!("Retry Stage 7 failed: {}", e); None }
            }
        } else {
            None
        };

        let _ = tx.send(Ok(PipelineOutput {
            clean_transcript: clean,
            structure: Default::default(),
            summary,
            action_items,
            report,
            generated_title,
        }));
    });

    let output = rx.await
        .map_err(|_| "Pipeline retry thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

    Ok(PipelineResult {
        clean_transcript: output.clean_transcript,
        summary: output.summary,
        report: output.report,
        generated_title: output.generated_title,
    })
}
