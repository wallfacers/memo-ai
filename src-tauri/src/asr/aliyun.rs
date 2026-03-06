// placeholder — 完整实现在 Task 2
use std::path::Path;
use crate::error::AppResult;
use super::transcript::TranscriptSegment;
use super::provider::AsrProvider;

pub struct AliyunAsr;

impl AliyunAsr {
    pub fn new(_app_key: &str, _ak_id: &str, _ak_secret: &str, _language: &str) -> Self {
        AliyunAsr
    }
}

impl AsrProvider for AliyunAsr {
    fn name(&self) -> &'static str { "aliyun" }
    fn transcribe(&self, _audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        Err(crate::error::AppError::Asr("Aliyun ASR not yet implemented".into()))
    }
}

pub fn test_connection(_app_key: &str, _ak_id: &str, _ak_secret: &str) -> Result<String, String> {
    Err("Aliyun ASR not yet implemented".into())
}
