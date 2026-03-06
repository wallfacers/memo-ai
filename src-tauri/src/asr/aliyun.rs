use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Deserialize;
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;
use super::provider::AsrProvider;

pub struct AliyunAsr {
    app_key: String,
    access_key_id: String,
    access_key_secret: String,
    language: String,
}

impl AliyunAsr {
    pub fn new(app_key: &str, ak_id: &str, ak_secret: &str, language: &str) -> Self {
        AliyunAsr {
            app_key: app_key.to_string(),
            access_key_id: ak_id.to_string(),
            access_key_secret: ak_secret.to_string(),
            language: language.to_string(),
        }
    }

    fn get_token(&self) -> AppResult<String> {
        #[derive(Deserialize)]
        struct TokenResp {
            #[serde(rename = "Token")]
            token: Option<TokenData>,
        }
        #[derive(Deserialize)]
        #[allow(non_snake_case)]
        struct TokenData {
            Id: String,
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| AppError::Asr(e.to_string()))?;

        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let resp = client
            .post("https://nls-gateway.cn-shanghai.aliyuncs.com/token")
            .form(&[
                ("grant_type", "client_credentials"),
                ("appkey", self.app_key.as_str()),
                ("secretKey", self.access_key_secret.as_str()),
            ])
            .header("X-NLS-AccessKeyId", &self.access_key_id)
            .header("X-NLS-Timestamp", ts.to_string())
            .send()
            .map_err(|e| AppError::Asr(format!("Aliyun token request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            let safe_body = if body.len() > 200 { &body[..200] } else { &body };
            return Err(AppError::Asr(format!(
                "Aliyun token error HTTP {}: {}",
                status, safe_body
            )));
        }

        let token_resp: TokenResp = resp
            .json()
            .map_err(|e| AppError::Asr(format!("Aliyun token parse failed: {}", e)))?;

        token_resp
            .token
            .map(|t| t.Id)
            .ok_or_else(|| AppError::Asr("Aliyun token response missing Token field".into()))
    }
}

#[derive(Deserialize)]
struct FlashResult {
    flash_result: Option<Vec<FlashSentence>>,
    status: Option<i32>,
    message: Option<String>,
}

#[derive(Deserialize)]
struct FlashSentence {
    text: String,
    begin_time: Option<u64>,
    end_time: Option<u64>,
}

impl AsrProvider for AliyunAsr {
    fn name(&self) -> &'static str {
        "aliyun"
    }

    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        let token = self.get_token()?;

        let audio_bytes = std::fs::read(audio_path)
            .map_err(|e| AppError::Asr(format!("Failed to read audio file: {}", e)))?;

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| AppError::Asr(e.to_string()))?;

        let format = audio_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("wav");

        let url = format!(
            "https://nls-gateway.cn-shanghai.aliyuncs.com/api/v1/recognition/flash?appkey={}&format={}&sample_rate=16000",
            url_encode(&self.app_key),
            url_encode(format)
        );

        let resp = client
            .post(&url)
            .header("X-NLS-Token", &token)
            .header("Content-Type", "application/octet-stream")
            .body(audio_bytes)
            .send()
            .map_err(|e| AppError::Asr(format!("Aliyun flash recognition failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            let safe_body = if body.len() > 200 { &body[..200] } else { &body };
            return Err(AppError::Asr(format!(
                "Aliyun ASR error HTTP {}: {}",
                status, safe_body
            )));
        }

        let result: FlashResult = resp
            .json()
            .map_err(|e| AppError::Asr(format!("Aliyun ASR response parse failed: {}", e)))?;

        if let Some(status) = result.status {
            if status != 20000000 {
                return Err(AppError::Asr(format!(
                    "Aliyun ASR failed (status {}): {}",
                    status,
                    result.message.unwrap_or_default()
                )));
            }
        }

        let segments = result
            .flash_result
            .unwrap_or_default()
            .into_iter()
            .map(|s| TranscriptSegment {
                start: s.begin_time.unwrap_or(0) as f64 / 1000.0,
                end: s.end_time.unwrap_or(0) as f64 / 1000.0,
                text: s.text.trim().to_string(),
                speaker: None,
                confidence: None,
            })
            .collect();

        Ok(segments)
    }
}

/// Test credentials by obtaining a token only (no audio upload needed).
pub fn test_connection(app_key: &str, ak_id: &str, ak_secret: &str) -> Result<String, String> {
    let asr = AliyunAsr::new(app_key, ak_id, ak_secret, "zh");
    asr.get_token()
        .map(|_| "阿里云 ASR 鉴权成功".to_string())
        .map_err(|e| e.to_string())
}

fn url_encode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_alphanumeric() || "-_.~".contains(c) {
                vec![c]
            } else {
                format!("%{:02X}", c as u32).chars().collect::<Vec<_>>()
            }
        })
        .collect()
}
