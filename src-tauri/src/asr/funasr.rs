use std::path::Path;
use std::sync::mpsc::{self, SyncSender, Receiver};
use std::thread;
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;
use super::provider::AsrProvider;
use super::streaming::{StreamingAsrSession, StreamingSegment};

// ─── FunASR WebSocket 响应/请求数据结构 ──────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FunAsrResponse {
    mode: Option<String>,
    text: Option<String>,
    #[serde(default)]
    is_final: bool,
    /// 时间戳（毫秒），FunASR offline 结果可能携带
    timestamp: Option<Vec<[u32; 2]>>,
}

#[derive(Debug, Serialize)]
struct FunAsrHandshake {
    mode: &'static str,
    wav_name: String,
    wav_format: &'static str,
    is_speaking: bool,
    chunk_size: [u32; 3],
    encoder_chunk_look_back: u32,
    decoder_chunk_look_back: u32,
}

// ─── FunAsrStreamSession ──────────────────────────────────────────────────────

/// 实时流式 ASR 会话，通过 OS 线程 + 同步 tungstenite WebSocket 实现，与 Tokio 隔离
pub struct FunAsrStreamSession {
    /// 向 WS 线程发送音频数据（None = 结束信号）
    audio_tx: SyncSender<Option<Vec<u8>>>,
    /// 接收 WS 线程返回的 final 段落
    result_rx: Receiver<StreamingSegment>,
}

impl FunAsrStreamSession {
    /// 建立 WebSocket 连接，启动后台处理线程
    ///
    /// `event_tx`：向调用方转发所有识别结果（partial + final），用于 Tauri 事件 emit
    pub fn connect(
        ws_url: &str,
        meeting_id: i64,
        event_tx: SyncSender<StreamingSegment>,
    ) -> AppResult<Self> {
        let (audio_tx, audio_rx) = mpsc::sync_channel::<Option<Vec<u8>>>(64);
        let (result_tx, result_rx) = mpsc::sync_channel::<StreamingSegment>(64);
        let ws_url = ws_url.to_string();

        thread::spawn(move || {
            if let Err(e) = run_ws_loop(
                &ws_url,
                meeting_id,
                audio_rx,
                result_tx,
                event_tx,
            ) {
                log::error!("FunASR WebSocket loop error: {}", e);
            }
        });

        Ok(FunAsrStreamSession { audio_tx, result_rx })
    }
}

/// 从 tungstenite MaybeTlsStream 内层设置 TCP read timeout
///
/// tungstenite 0.24 的 `get_mut()` 返回 `MaybeTlsStream<TcpStream>`，
/// 通过 `get_ref()` 可拿到内层 `TcpStream`。
fn set_ws_read_timeout(
    ws: &mut tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
    timeout: Option<std::time::Duration>,
) {
    match ws.get_mut() {
        tungstenite::stream::MaybeTlsStream::Plain(tcp) => {
            let _ = tcp.set_read_timeout(timeout);
        }
        tungstenite::stream::MaybeTlsStream::NativeTls(tls) => {
            let _ = tls.get_ref().set_read_timeout(timeout);
        }
        _ => {
            // 其他 TLS 变体不支持，忽略
        }
    }
}

/// WebSocket 主循环（运行在 OS 线程，使用同步 tungstenite）
fn run_ws_loop(
    ws_url: &str,
    meeting_id: i64,
    audio_rx: Receiver<Option<Vec<u8>>>,
    result_tx: SyncSender<StreamingSegment>,
    event_tx: SyncSender<StreamingSegment>,
) -> AppResult<()> {
    use tungstenite::{connect, Message};

    let (mut ws, _) = connect(ws_url)
        .map_err(|e| AppError::Asr(format!("FunASR WebSocket connect failed ({}): {}", ws_url, e)))?;

    // 设置非阻塞读取超时（10ms），使主循环可以持续处理音频发送
    set_ws_read_timeout(&mut ws, Some(std::time::Duration::from_millis(10)));

    // 握手帧
    let handshake = FunAsrHandshake {
        mode: "2pass",
        wav_name: format!("meeting_{}", meeting_id),
        wav_format: "pcm",
        is_speaking: true,
        chunk_size: [5, 10, 5],
        encoder_chunk_look_back: 4,
        decoder_chunk_look_back: 0,
    };
    ws.send(Message::Text(
        serde_json::to_string(&handshake)
            .map_err(|e| AppError::Asr(format!("FunASR handshake serialize failed: {}", e)))?,
    ))
    .map_err(|e| AppError::Asr(format!("FunASR handshake send failed: {}", e)))?;

    let mut segment_counter: u32 = 0;
    let mut finish_sent = false;

    loop {
        // 1. 非阻塞读取音频队列并发送
        match audio_rx.try_recv() {
            Ok(Some(pcm_bytes)) => {
                if let Err(e) = ws.send(Message::Binary(pcm_bytes)) {
                    log::warn!("FunASR audio send failed: {}", e);
                    break;
                }
            }
            Ok(None) => {
                // 结束信号：通知服务器停止说话
                let end_msg = serde_json::json!({"is_speaking": false});
                let _ = ws.send(Message::Text(end_msg.to_string()));
                finish_sent = true;
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
            Err(mpsc::TryRecvError::Empty) => {}
        }

        // 2. 非阻塞读取 WS 响应（超时 10ms，由 set_read_timeout 保证）
        match ws.read() {
            Ok(Message::Text(text)) => {
                if let Ok(resp) = serde_json::from_str::<FunAsrResponse>(&text) {
                    if let Some(t) = resp.text {
                        let trimmed = t.trim().to_string();
                        if !trimmed.is_empty() {
                            let is_final = resp.mode.as_deref() == Some("2pass-offline")
                                || resp.is_final;

                            // 从 timestamp 字段提取第一个和最后一个时间戳
                            let (start_ms, end_ms) = resp.timestamp
                                .and_then(|ts| {
                                    let first = ts.first().map(|t| t[0]);
                                    let last = ts.last().map(|t| t[1]);
                                    first.zip(last)
                                })
                                .map(|(s, e)| (Some(s), Some(e)))
                                .unwrap_or((None, None));

                            let seg = StreamingSegment {
                                text: trimmed,
                                is_final,
                                segment_id: segment_counter,
                                start_ms,
                                end_ms,
                            };
                            segment_counter += 1;

                            // 转发给 event channel（Tauri emit）
                            let _ = event_tx.try_send(seg.clone());

                            // final 结果也存入 result channel（供 finish() 收集）
                            if is_final {
                                let _ = result_tx.try_send(seg);
                            }
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(tungstenite::Error::Io(e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // 读取超时，正常情况，继续循环
            }
            Err(e) => {
                if finish_sent {
                    // 发送结束信号后连接关闭是正常的
                    break;
                }
                log::warn!("FunASR WS read error: {}", e);
                break;
            }
            _ => {}
        }

        // 3. 发送结束信号后，稍作等待避免忙轮询
        if finish_sent {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    Ok(())
}

impl StreamingAsrSession for FunAsrStreamSession {
    fn send_audio_chunk(&mut self, pcm: &[i16]) -> AppResult<()> {
        let bytes: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        self.audio_tx
            .try_send(Some(bytes))
            .map_err(|_| AppError::Asr("FunASR audio channel full or disconnected".into()))
    }

    fn finish(&mut self) -> AppResult<Vec<StreamingSegment>> {
        // 发送结束信号（None 触发 is_speaking=false）
        let _ = self.audio_tx.try_send(None);

        // 收集所有 final 结果，最长等待 8 秒
        let mut finals = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            if std::time::Instant::now() > deadline {
                log::warn!("FunASR finish() timed out after 8s, collected {} finals", finals.len());
                break;
            }
            match self.result_rx.try_recv() {
                Ok(seg) => finals.push(seg),
                Err(mpsc::TryRecvError::Disconnected) => break,
                Err(mpsc::TryRecvError::Empty) => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        Ok(finals)
    }
}

// ─── FunAsrBatchProvider（实现 AsrProvider，用于录音后批量精确转写）─────────

pub struct FunAsrBatchProvider {
    ws_url: String,
}

impl FunAsrBatchProvider {
    pub fn new(ws_url: &str) -> Self {
        FunAsrBatchProvider { ws_url: ws_url.to_string() }
    }
}

impl AsrProvider for FunAsrBatchProvider {
    fn name(&self) -> &'static str {
        "funasr"
    }

    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        use tungstenite::{connect, Message};

        let audio_bytes = std::fs::read(audio_path)
            .map_err(|e| AppError::Asr(format!("Failed to read audio file: {}", e)))?;

        let (mut ws, _) = connect(&self.ws_url)
            .map_err(|e| AppError::Asr(format!("FunASR batch connect failed: {}", e)))?;

        // offline 模式握手
        let handshake = serde_json::json!({
            "mode": "offline",
            "wav_name": "batch",
            "wav_format": "wav",
            "is_speaking": true,
        });
        ws.send(Message::Text(handshake.to_string()))
            .map_err(|e| AppError::Asr(format!("FunASR batch handshake failed: {}", e)))?;

        // 分块发送音频（每次 64KB）
        for chunk in audio_bytes.chunks(65536) {
            ws.send(Message::Binary(chunk.to_vec()))
                .map_err(|e| AppError::Asr(format!("FunASR batch send audio failed: {}", e)))?;
        }

        // 发送结束信号
        ws.send(Message::Text(
            serde_json::json!({"is_speaking": false}).to_string(),
        ))
        .map_err(|e| AppError::Asr(format!("FunASR batch finish signal failed: {}", e)))?;

        // 等待 offline 最终结果（最长 5 分钟）
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
        let mut segments = Vec::new();

        loop {
            if std::time::Instant::now() > deadline {
                return Err(AppError::Asr("FunASR batch transcription timed out (300s)".into()));
            }
            match ws.read() {
                Ok(Message::Text(text)) => {
                    if let Ok(resp) = serde_json::from_str::<FunAsrResponse>(&text) {
                        let is_done = resp.is_final
                            || resp.mode.as_deref() == Some("offline");
                        if let Some(t) = resp.text {
                            let trimmed = t.trim().to_string();
                            if !trimmed.is_empty() {
                                // 从 timestamp 提取时间戳（毫秒转秒）
                                let (start, end) = resp.timestamp
                                    .and_then(|ts| {
                                        let first = ts.first().map(|t| t[0]);
                                        let last = ts.last().map(|t| t[1]);
                                        first.zip(last)
                                    })
                                    .map(|(s, e)| (s as f64 / 1000.0, e as f64 / 1000.0))
                                    .unwrap_or((0.0, 0.0));

                                segments.push(TranscriptSegment {
                                    start,
                                    end,
                                    text: trimmed,
                                    speaker: None,
                                    confidence: None,
                                });
                            }
                        }
                        if is_done {
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    return Err(AppError::Asr(format!("FunASR batch read error: {}", e)));
                }
                _ => {}
            }
        }

        Ok(segments)
    }
}

// ─── 智能合并（batch 为主，streaming 兜底）────────────────────────────────────

/// 智能合并批量转写结果（高精度）与实时流式 final 段落（草稿兜底）
///
/// 策略：
/// 1. batch 非空 → 直接使用 batch（精度更高）
/// 2. batch 为空（转写失败/超时）→ 降级使用 streaming final 段
pub fn smart_merge(
    batch: &[TranscriptSegment],
    streaming: &[StreamingSegment],
) -> Vec<TranscriptSegment> {
    if !batch.is_empty() {
        return batch.to_vec();
    }

    // batch 失败降级
    log::warn!(
        "smart_merge: batch results empty, falling back to {} streaming final segments",
        streaming.len()
    );
    streaming
        .iter()
        .filter(|s| s.is_final)
        .map(|s| TranscriptSegment {
            start: s.start_ms.unwrap_or(0) as f64 / 1000.0,
            end: s.end_ms.unwrap_or(0) as f64 / 1000.0,
            text: s.text.clone(),
            speaker: None,
            confidence: None,
        })
        .collect()
}
