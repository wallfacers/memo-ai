use std::io::Cursor;
use std::path::Path;
use hound::{WavReader, WavWriter, SampleFormat};
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;
use super::provider::AsrProvider;

const CHUNK_SECS: u32 = 30;

pub struct Qwen3AsrProvider {
    api_url: String,
}

impl Qwen3AsrProvider {
    pub fn new(api_url: &str) -> Self {
        Qwen3AsrProvider {
            api_url: api_url.trim_end_matches('/').to_string(),
        }
    }

    fn split_wav(audio_path: &Path, chunk_secs: u32) -> AppResult<(Vec<Vec<u8>>, f64)> {
        let mut reader = WavReader::open(audio_path)
            .map_err(|e| AppError::Asr(format!("Qwen3-ASR: failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels as u32;
        let chunk_samples = (chunk_secs * sample_rate * channels) as usize;

        let all_samples: Vec<i32> = match spec.sample_format {
            SampleFormat::Int => reader
                .samples::<i32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV read error: {}", e)))?,
            SampleFormat::Float => reader
                .samples::<f32>()
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV read float error: {}", e)))?
                .into_iter()
                .map(|s| (s * i32::MAX as f32) as i32)
                .collect(),
        };

        let total_samples = all_samples.len();
        let frames_per_channel = total_samples / channels as usize;
        let total_secs = frames_per_channel as f64 / sample_rate as f64;

        let mut chunks = Vec::new();
        let mut offset = 0usize;

        while offset < total_samples {
            let end = (offset + chunk_samples).min(total_samples);
            let chunk_slice = &all_samples[offset..end];

            let buf: Vec<u8> = Vec::new();
            let cursor = Cursor::new(buf);
            let mut writer = WavWriter::new(cursor, spec)
                .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV writer error: {}", e)))?;

            for &s in chunk_slice {
                writer.write_sample(s)
                    .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV write sample error: {}", e)))?;
            }
            let cursor = writer.into_inner()
                .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV finalize error: {}", e)))?;

            chunks.push(cursor.into_inner());
            offset = end;
        }

        Ok((chunks, total_secs))
    }

    fn transcribe_chunk(&self, wav_bytes: Vec<u8>) -> AppResult<String> {
        let endpoint = format!("{}/v1/audio/transcriptions", self.api_url);

        let part = reqwest::blocking::multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AppError::Asr(format!("Qwen3-ASR: multipart error: {}", e)))?;

        let form = reqwest::blocking::multipart::Form::new()
            .part("file", part)
            .text("model", self.api_url.clone());

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| AppError::Asr(format!("Qwen3-ASR: client build error: {}", e)))?;

        let resp = client
            .post(&endpoint)
            .multipart(form)
            .send()
            .map_err(|e| AppError::Asr(format!("Qwen3-ASR: request failed ({}): {}", endpoint, e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(AppError::Asr(format!("Qwen3-ASR: HTTP {} - {}", status, body)));
        }

        let json: serde_json::Value = resp.json()
            .map_err(|e| AppError::Asr(format!("Qwen3-ASR: JSON parse error: {}", e)))?;

        let raw = json["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let text = if let Some(pos) = raw.find("<asr_text>") {
            raw[pos + "<asr_text>".len()..].trim().to_string()
        } else {
            raw.trim().to_string()
        };

        Ok(text)
    }
}

impl AsrProvider for Qwen3AsrProvider {
    fn name(&self) -> &'static str {
        "qwen3_asr"
    }

    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        let (chunks, total_secs) = Self::split_wav(audio_path, CHUNK_SECS)?;
        let num_chunks = chunks.len();
        let mut segments = Vec::new();

        for (i, wav_bytes) in chunks.into_iter().enumerate() {
            let start = (i as u32 * CHUNK_SECS) as f64;
            let end = ((i as u32 + 1) * CHUNK_SECS) as f64;
            let end = end.min(total_secs);

            match self.transcribe_chunk(wav_bytes) {
                Ok(text) if !text.is_empty() => {
                    segments.push(TranscriptSegment {
                        start,
                        end,
                        text,
                        speaker: None,
                        confidence: None,
                    });
                }
                Ok(_) => {
                    log::debug!("Qwen3-ASR: chunk {}/{} returned empty text, skipping", i + 1, num_chunks);
                }
                Err(e) => {
                    log::warn!("Qwen3-ASR: chunk {}/{} failed: {}", i + 1, num_chunks, e);
                }
            }
        }

        if segments.is_empty() {
            return Err(AppError::Asr("Qwen3-ASR: all chunks returned empty or failed".into()));
        }

        Ok(segments)
    }
}
