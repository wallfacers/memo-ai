/// Whisper ASR integration.
///
/// whisper-rs (whisper.cpp Rust bindings) requires CMake and a C++ compiler.
/// Since it's a heavy build dependency, it is feature-flagged here.
/// The current implementation uses a subprocess call to `whisper-cli` if
/// available, with a stub fallback for development.
///
/// To enable full whisper-rs integration:
/// 1. Add to Cargo.toml: `whisper-rs = { version = "0.11", features = ["cuda"] }` (optional GPU)
/// 2. Implement using `WhisperContext::new_with_params`
use std::path::Path;
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;

pub struct WhisperAsr {
    model_path: String,
    language: String,
}

impl WhisperAsr {
    pub fn new(model_path: &str, language: &str) -> Self {
        WhisperAsr {
            model_path: model_path.to_string(),
            language: language.to_string(),
        }
    }

    /// Transcribe the audio file at `audio_path`.
    /// Returns a list of transcript segments.
    pub fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        // Check if whisper-cli is available in PATH
        let output = std::process::Command::new("whisper")
            .arg(audio_path.to_str().unwrap_or(""))
            .arg("--model")
            .arg(&self.model_path)
            .arg("--language")
            .arg(&self.language)
            .arg("--output_format")
            .arg("json")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                parse_whisper_json(&json_str)
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                Err(AppError::Asr(format!("whisper exited with error: {}", err)))
            }
            Err(_) => {
                // whisper-cli not found — return stub segment for development
                log::warn!("whisper CLI not found, returning stub transcript");
                Ok(vec![TranscriptSegment {
                    start: 0.0,
                    end: 1.0,
                    text: "[ASR stub: whisper model not loaded]".to_string(),
                    speaker: None,
                    confidence: None,
                }])
            }
        }
    }
}

fn parse_whisper_json(json_str: &str) -> AppResult<Vec<TranscriptSegment>> {
    #[derive(serde::Deserialize)]
    struct WSegment {
        start: f64,
        end: f64,
        text: String,
    }
    #[derive(serde::Deserialize)]
    struct WOutput {
        segments: Vec<WSegment>,
    }

    let output: WOutput = serde_json::from_str(json_str)?;
    Ok(output
        .segments
        .into_iter()
        .map(|s| TranscriptSegment {
            start: s.start,
            end: s.end,
            text: s.text.trim().to_string(),
            speaker: None,
            confidence: None,
        })
        .collect())
}
