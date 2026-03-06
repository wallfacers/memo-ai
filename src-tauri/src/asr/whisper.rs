use std::path::Path;
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;
use super::provider::AsrProvider;

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
}

impl AsrProvider for WhisperAsr {
    fn name(&self) -> &'static str { "local_whisper" }

    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        // whisper-cli writes JSON to a file (-oj), not stdout.
        // We use a temp path and read the resulting <tmp>.json file.
        let tmp_base = std::env::temp_dir().join(format!(
            "whisper_out_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        ));
        let tmp_json = tmp_base.with_extension("json");

        let audio_str = audio_path
            .to_str()
            .ok_or_else(|| AppError::Asr("Audio path contains non-UTF-8 characters".to_string()))?;

        let result = std::process::Command::new(&self.cli_path)
            .arg("-f").arg(audio_str)
            .arg("-m").arg(&self.model_path)
            .arg("-l").arg(&self.language)
            .arg("-oj")                               // output JSON to file
            .arg("-of").arg(tmp_base.to_str().unwrap_or("whisper_out"))  // base path (no extension)
            .arg("-nt")                               // no timestamps in text output
            .output();

        match result {
            Ok(out) if out.status.success() => {
                let json_str = std::fs::read_to_string(&tmp_json)
                    .map_err(|e| AppError::Asr(format!(
                        "whisper-cli ran but output file not found ({}): {}",
                        tmp_json.display(), e
                    )))?;
                let _ = std::fs::remove_file(&tmp_json);
                parse_whisper_json(&json_str)
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                let stdout = String::from_utf8_lossy(&out.stdout);
                Err(AppError::Asr(format!(
                    "whisper-cli exited with error (code {:?})\nstderr: {}\nstdout: {}",
                    out.status.code(), stderr, stdout
                )))
            }
            Err(e) => Err(AppError::Asr(format!(
                "Cannot run whisper-cli at '{}': {}. Download from https://github.com/ggerganov/whisper.cpp/releases",
                self.cli_path, e
            ))),
        }
    }
}

fn parse_whisper_json(json_str: &str) -> AppResult<Vec<TranscriptSegment>> {
    // whisper-cli -oj output format:
    // { "transcription": [ { "offsets": {"from": <ms>, "to": <ms>}, "text": "..." }, ... ] }
    #[derive(serde::Deserialize)]
    struct WOffsets {
        from: u64,
        to: u64,
    }
    #[derive(serde::Deserialize)]
    struct WSegment {
        #[serde(default)]
        offsets: Option<WOffsets>,
        text: String,
    }
    #[derive(serde::Deserialize)]
    struct WOutput {
        transcription: Vec<WSegment>,
    }

    let output: WOutput = serde_json::from_str(json_str)
        .map_err(|e| AppError::Asr(format!("Failed to parse whisper JSON: {}", e)))?;
    Ok(output
        .transcription
        .into_iter()
        .map(|s| {
            let (start, end) = s.offsets
                .map(|o| (o.from as f64 / 1000.0, o.to as f64 / 1000.0))
                .unwrap_or((0.0, 0.0));
            TranscriptSegment {
                start,
                end,
                text: s.text.trim().to_string(),
                speaker: None,
                confidence: None,
            }
        })
        .collect())
}
