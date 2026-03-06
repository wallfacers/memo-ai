use std::path::Path;
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;

pub struct WhisperAsr {
    cli_path: String,
    model_path: String,
    language: String,
}

impl WhisperAsr {
    pub fn new(cli_path: &str, model_path: &str, language: &str) -> Self {
        WhisperAsr {
            cli_path: cli_path.to_string(),
            model_path: model_path.to_string(),
            language: language.to_string(),
        }
    }

    pub fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        let output = std::process::Command::new(&self.cli_path)
            .arg("-f")
            .arg(audio_path.to_str().unwrap_or(""))
            .arg("-m")
            .arg(&self.model_path)
            .arg("-l")
            .arg(&self.language)
            .arg("--output-format")
            .arg("json")
            .arg("--no-timestamps")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                parse_whisper_json(&json_str)
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                Err(AppError::Asr(format!("whisper-cli error: {}", err)))
            }
            Err(e) => Err(AppError::Asr(format!(
                "Cannot run whisper-cli at '{}': {}. Download from https://github.com/ggerganov/whisper.cpp/releases",
                self.cli_path, e
            ))),
        }
    }
}

fn parse_whisper_json(json_str: &str) -> AppResult<Vec<TranscriptSegment>> {
    #[derive(serde::Deserialize)]
    struct WSegment {
        #[serde(default)]
        start: f64,
        #[serde(default)]
        end: f64,
        text: String,
    }
    #[derive(serde::Deserialize)]
    struct WOutput {
        transcription: Vec<WSegment>,
    }

    // whisper.cpp JSON format uses "transcription" array
    let output: WOutput = serde_json::from_str(json_str)
        .map_err(|e| AppError::Asr(format!("Failed to parse whisper JSON: {}", e)))?;
    Ok(output
        .transcription
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
