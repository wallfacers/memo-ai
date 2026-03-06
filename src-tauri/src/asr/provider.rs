use std::path::Path;
use crate::error::AppResult;
use super::transcript::TranscriptSegment;
use crate::commands::AppConfig;

pub trait AsrProvider: Send {
    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>>;
    fn name(&self) -> &'static str;
}

pub fn build_asr(config: &AppConfig) -> Box<dyn AsrProvider> {
    match config.asr_provider.as_str() {
        "aliyun" => Box::new(super::aliyun::AliyunAsr::new(
            &config.aliyun_asr_app_key,
            &config.aliyun_asr_access_key_id,
            &config.aliyun_asr_access_key_secret,
            &config.language,
        )),
        _ => {
            let model_path = format!(
                "{}/ggml-{}.bin",
                config.whisper_model_dir, config.whisper_model
            );
            Box::new(super::whisper::WhisperAsr::new(
                &config.whisper_cli_path,
                &model_path,
                &config.language,
            ))
        }
    }
}
