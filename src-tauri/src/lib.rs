mod audio;
mod asr;
mod commands;
mod db;
mod error;
mod llm;

use commands::{
    ConfigState, DbState, RecordState,
    create_meeting, delete_meeting, get_action_items, get_meeting, get_settings,
    get_transcripts, list_meetings, run_pipeline, save_settings, start_recording,
    stop_recording, transcribe_audio, update_action_item_status,
};
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize SQLite database
            let data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let db_path = data_dir.join("memo-ai.db");
            log::info!("Database path: {:?}", db_path);

            let conn = db::connection::init_db(&db_path)
                .expect("Failed to initialize database");

            app.manage(DbState(Mutex::new(conn)));
            app.manage(RecordState(Mutex::new(None)));

            let settings_path = commands::settings_path(&app.handle())
                .unwrap_or_else(|_| data_dir.join("settings.json"));
            let config = if settings_path.exists() {
                match std::fs::read_to_string(&settings_path) {
                    Ok(s) => match serde_json::from_str::<commands::AppConfig>(&s) {
                        Ok(c) => c,
                        Err(e) => {
                            log::warn!("Failed to parse settings.json: {}. Using defaults.", e);
                            commands::AppConfig::default()
                        }
                    },
                    Err(e) => {
                        log::warn!("Failed to read settings.json: {}. Using defaults.", e);
                        commands::AppConfig::default()
                    }
                }
            } else {
                commands::AppConfig::default()
            };
            app.manage(ConfigState(Mutex::new(config)));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_meetings,
            get_meeting,
            create_meeting,
            delete_meeting,
            start_recording,
            stop_recording,
            get_transcripts,
            transcribe_audio,
            run_pipeline,
            get_action_items,
            update_action_item_status,
            get_settings,
            save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
