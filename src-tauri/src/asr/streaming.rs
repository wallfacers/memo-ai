use serde::{Deserialize, Serialize};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSegment {
    pub text: String,
    pub is_final: bool,
    pub segment_id: u32,
    /// 句子开始时间（毫秒），interim 结果为 None
    #[serde(default)]
    pub start_ms: Option<u32>,
    /// 句子结束时间（毫秒），interim 结果为 None
    #[serde(default)]
    pub end_ms: Option<u32>,
}

pub trait StreamingAsrSession: Send {
    /// 发送一段 PCM 16-bit 16kHz mono 音频数据（每次约 160ms）
    #[allow(dead_code)]
    fn send_audio_chunk(&mut self, pcm: &[i16]) -> AppResult<()>;
    /// 通知服务器录音结束，阻塞等待所有 final 结果返回（最长 8 秒超时）
    fn finish(&mut self) -> AppResult<Vec<StreamingSegment>>;
}
