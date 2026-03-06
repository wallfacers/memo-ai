use serde::{Deserialize, Serialize};
use crate::error::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSegment {
    pub text: String,
    pub is_final: bool,
    pub segment_id: u32,
}

pub trait StreamingAsrSession: Send {
    /// 发送一段 PCM 16-bit 16kHz mono 音频数据（每次约 160ms）
    fn send_audio_chunk(&mut self, pcm: &[i16]) -> AppResult<()>;
    /// 通知服务器录音结束，等待最后的 final 结果
    fn finish(&mut self) -> AppResult<Vec<StreamingSegment>>;
}
