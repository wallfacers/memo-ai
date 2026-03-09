use std::io::Cursor;
use std::path::Path;
use hound::{WavReader, WavWriter, SampleFormat};
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;
use super::provider::AsrProvider;

const CHUNK_SECS: u32 = 30;

pub struct Qwen3AsrProvider {
    api_url: String,
    model_name: String,
    client: reqwest::blocking::Client,
}

impl Qwen3AsrProvider {
    pub fn new(api_url: &str) -> Self {
        let api_url = api_url.trim_end_matches('/').to_string();
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        let model_name = Self::resolve_model_name(&api_url, &client);
        Qwen3AsrProvider { api_url, model_name, client }
    }

    fn resolve_model_name(api_url: &str, client: &reqwest::blocking::Client) -> String {
        let url = format!("{}/v1/models", api_url);
        match client.get(&url).send() {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(json) = resp.json::<serde_json::Value>() {
                    if let Some(id) = json["data"][0]["id"].as_str() {
                        log::info!("Qwen3-ASR: resolved model name: {}", id);
                        return id.to_string();
                    }
                }
                log::warn!("Qwen3-ASR: could not parse model name from /v1/models");
                String::new()
            }
            Ok(resp) => {
                log::warn!("Qwen3-ASR: /v1/models returned HTTP {}", resp.status());
                String::new()
            }
            Err(e) => {
                log::warn!("Qwen3-ASR: /v1/models query failed: {}", e);
                String::new()
            }
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

            let mut buf = Cursor::new(Vec::<u8>::new());
            {
                let mut writer = WavWriter::new(&mut buf, spec)
                    .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV writer error: {}", e)))?;
                for &s in chunk_slice {
                    writer.write_sample(s)
                        .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV write sample error: {}", e)))?;
                }
                writer.finalize()
                    .map_err(|e| AppError::Asr(format!("Qwen3-ASR: WAV finalize error: {}", e)))?;
            }
            chunks.push(buf.into_inner());
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

        let form = if self.model_name.is_empty() {
            reqwest::blocking::multipart::Form::new().part("file", part)
        } else {
            reqwest::blocking::multipart::Form::new()
                .part("file", part)
                .text("model", self.model_name.clone())
        };

        let resp = self.client
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
            let start = i as f64 * CHUNK_SECS as f64;
            let end = ((i + 1) as f64 * CHUNK_SECS as f64).min(total_secs);

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
