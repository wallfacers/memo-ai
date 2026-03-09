# FunASR 实时录音转写集成 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 集成 FunASR WebSocket 流式 ASR，实现录音时实时字幕（temp/final 双态）、录音结束后智能合并批量转写结果，并在 LLM Pipeline 各阶段实时向前端推送进度事件。

**Architecture:** 方案 B（独立 Streaming 层），新增 `StreamingAsrSession` trait 专职实时流式，与现有 `AsrProvider` batch 接口并行独立；FunASR 进程管理器智能判断外部/本地模式；Pipeline 新增阶段事件 emit。

**Tech Stack:** Rust（tokio-tungstenite WebSocket、tokio async task）、React + TypeScript、shadcn/ui（Progress, Badge, Card）、lucide-react

**设计文档：** [docs/plans/2026-03-07-funasr-realtime-asr-design.md](../../plans/2026-03-07-funasr-realtime-asr-design.md)

---

## Task 1: Rust 基础 — streaming trait + Cargo 依赖

**Files:**
- Create: `src-tauri/src/asr/streaming.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/asr/mod.rs`

**背景：** 定义 `StreamingAsrSession` trait 和 `StreamingSegment` 数据结构，添加 WebSocket 依赖。

---

**Step 1: 在 `Cargo.toml` 添加 WebSocket 依赖**

在 `[dependencies]` 中添加：

```toml
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
futures-util = "0.3"
```

---

**Step 2: 创建 `src-tauri/src/asr/streaming.rs`**

```rust
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
```

---

**Step 3: 更新 `src-tauri/src/asr/mod.rs`**

```rust
pub mod provider;
pub mod transcript;
pub mod whisper;
pub mod aliyun;
pub mod funasr;
pub mod streaming;

pub use provider::{AsrProvider, build_asr};
pub use streaming::{StreamingAsrSession, StreamingSegment};
```

---

**Step 4: 验证编译**

```bash
cargo check --manifest-path "D:/develop/python/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

Expected: `Finished` 无 error

---

**Step 5: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src-tauri/src/asr/streaming.rs src-tauri/src/asr/mod.rs src-tauri/Cargo.toml
git -C "D:/develop/python/source/memo-ai" commit -m "feat: add StreamingAsrSession trait and tokio-tungstenite dependency"
```

---

## Task 2: Rust — FunASR 进程管理器

**Files:**
- Create: `src-tauri/src/process/mod.rs`
- Create: `src-tauri/src/process/funasr_server.rs`
- Modify: `src-tauri/src/lib.rs`（添加 `mod process`）

**背景：** 智能判断使用外部 FunASR 服务还是本地自管理进程。

---

**Step 1: 创建 `src-tauri/src/process/mod.rs`**

```rust
pub mod funasr_server;
pub use funasr_server::FunAsrServer;
```

---

**Step 2: 创建 `src-tauri/src/process/funasr_server.rs`**

```rust
use std::process::{Child, Command};

pub struct FunAsrServer {
    child: Option<Child>,
    pub ws_url: String,
}

impl FunAsrServer {
    /// 根据配置决定使用外部服务还是启动本地进程
    pub fn start(ws_url: &str, server_path: &str, port: u16) -> Result<Self, String> {
        // 1. 有配置 URL → 直接用外部服务
        if !ws_url.is_empty() {
            return Ok(FunAsrServer {
                child: None,
                ws_url: ws_url.to_string(),
            });
        }

        // 2. 无配置 URL → 检测本地 funasr-server 并尝试启动
        let exe = if server_path.is_empty() { "funasr-server" } else { server_path };

        // 先检测可执行文件是否存在
        let check = Command::new(exe).arg("--help").output();
        if check.is_err() {
            return Err(format!(
                "FunASR server not found at '{}'. Install via: pip install funasr-runtime",
                exe
            ));
        }

        let child = Command::new(exe)
            .arg("--port")
            .arg(port.to_string())
            .arg("--model")
            .arg("paraformer-zh")
            .spawn()
            .map_err(|e| format!("Failed to start funasr-server: {}", e))?;

        // 等待服务就绪（简单 sleep 3s）
        std::thread::sleep(std::time::Duration::from_secs(3));

        Ok(FunAsrServer {
            child: Some(child),
            ws_url: format!("ws://localhost:{}", port),
        })
    }

    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
        self.child = None;
    }

    pub fn is_managed(&self) -> bool {
        self.child.is_some()
    }
}

impl Drop for FunAsrServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 检测本地 funasr-server 是否可用，返回版本/错误信息
pub fn check_funasr_server(server_path: &str) -> Result<String, String> {
    let exe = if server_path.is_empty() { "funasr-server" } else { server_path };
    match Command::new(exe).arg("--version").output() {
        Ok(out) => {
            let version = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("unknown")
                .trim()
                .to_string();
            Ok(if version.is_empty() { "funasr-server found".to_string() } else { version })
        }
        Err(_) => Err(format!(
            "funasr-server not found at '{}'. Install via: pip install funasr-runtime",
            exe
        )),
    }
}
```

---

**Step 3: 在 `src-tauri/src/lib.rs` 顶部添加**

```rust
mod process;
```

---

**Step 4: 验证编译**

```bash
cargo check --manifest-path "D:/develop/python/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

---

**Step 5: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src-tauri/src/process/ src-tauri/src/lib.rs
git -C "D:/develop/python/source/memo-ai" commit -m "feat: add FunAsrServer process manager with smart local/external mode"
```

---

## Task 3: Rust — FunASR WebSocket 流式会话

**Files:**
- Create: `src-tauri/src/asr/funasr.rs`

**背景：** 实现 FunASR WebSocket 2pass 协议的流式会话。使用 tokio-tungstenite 在 tokio 运行时中建立 WebSocket 连接；implement `AsrProvider`（batch 模式）供后续智能合并使用。

FunASR WebSocket 协议：
- 握手：发送 JSON 配置帧（`is_speaking: true`）
- 数据：发送 PCM binary 帧（16-bit 16kHz mono）
- 结束：发送 JSON `{"is_speaking": false}`
- 响应：`{"mode": "2pass-online", "text": "...", "is_final": false}` / `{"mode": "2pass-offline", "text": "...", "is_final": true}`

---

**Step 1: 创建 `src-tauri/src/asr/funasr.rs`**

```rust
use std::path::Path;
use std::sync::mpsc::{self, SyncSender, Receiver};
use std::thread;
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;
use super::provider::AsrProvider;
use super::streaming::{StreamingAsrSession, StreamingSegment};

// ─── 数据结构 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct FunAsrResponse {
    mode: Option<String>,
    text: Option<String>,
    #[serde(default)]
    is_final: bool,
    wav_name: Option<String>,
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

/// 通过 std::thread + OS-level WebSocket 实现，与 tokio 隔离
pub struct FunAsrStreamSession {
    audio_tx: SyncSender<Option<Vec<u8>>>,   // None = finish signal
    result_rx: Receiver<StreamingSegment>,
    segment_counter: u32,
}

impl FunAsrStreamSession {
    /// 创建流式会话，建立 WebSocket 连接，启动后台线程
    /// `event_tx`：用于向 Tauri 发送事件（由调用方传入）
    pub fn connect(
        ws_url: &str,
        meeting_id: i64,
        event_tx: std::sync::mpsc::SyncSender<StreamingSegment>,
    ) -> AppResult<Self> {
        let (audio_tx, audio_rx) = mpsc::sync_channel::<Option<Vec<u8>>>(64);
        let (result_tx, result_rx) = mpsc::sync_channel::<StreamingSegment>(64);
        let ws_url = ws_url.to_string();

        // 在 OS 线程中运行同步 WebSocket（tungstenite blocking）
        thread::spawn(move || {
            if let Err(e) = run_ws_loop(&ws_url, meeting_id, audio_rx, result_tx.clone(), event_tx) {
                log::error!("FunASR WebSocket error: {}", e);
            }
        });

        Ok(FunAsrStreamSession {
            audio_tx,
            result_rx,
            segment_counter: 0,
        })
    }
}

fn run_ws_loop(
    ws_url: &str,
    meeting_id: i64,
    audio_rx: Receiver<Option<Vec<u8>>>,
    result_tx: std::sync::mpsc::SyncSender<StreamingSegment>,
    event_tx: std::sync::mpsc::SyncSender<StreamingSegment>,
) -> AppResult<()> {
    use tungstenite::{connect, Message};

    let (mut ws, _) = connect(ws_url)
        .map_err(|e| AppError::Asr(format!("FunASR WebSocket connect failed: {}", e)))?;

    // 发送握手帧
    let handshake = FunAsrHandshake {
        mode: "2pass",
        wav_name: format!("meeting_{}", meeting_id),
        wav_format: "pcm",
        is_speaking: true,
        chunk_size: [5, 10, 5],
        encoder_chunk_look_back: 4,
        decoder_chunk_look_back: 0,
    };
    ws.send(Message::Text(serde_json::to_string(&handshake).unwrap()))
        .map_err(|e| AppError::Asr(format!("FunASR handshake failed: {}", e)))?;

    let mut counter: u32 = 0;
    let mut finished = false;

    loop {
        // 非阻塞读取音频队列
        match audio_rx.try_recv() {
            Ok(Some(pcm_bytes)) => {
                ws.send(Message::Binary(pcm_bytes))
                    .map_err(|e| AppError::Asr(format!("FunASR send audio failed: {}", e)))?;
            }
            Ok(None) => {
                // finish signal
                let end = serde_json::json!({"is_speaking": false});
                ws.send(Message::Text(end.to_string()))
                    .map_err(|e| AppError::Asr(format!("FunASR finish signal failed: {}", e)))?;
                finished = true;
            }
            Err(_) => {}
        }

        // 读取服务器响应
        match ws.read() {
            Ok(Message::Text(text)) => {
                if let Ok(resp) = serde_json::from_str::<FunAsrResponse>(&text) {
                    if let Some(t) = resp.text {
                        if !t.trim().is_empty() {
                            let is_final = resp.mode.as_deref() == Some("2pass-offline") || resp.is_final;
                            let seg = StreamingSegment {
                                text: t.trim().to_string(),
                                is_final,
                                segment_id: counter,
                            };
                            counter += 1;
                            let _ = event_tx.try_send(seg.clone());
                            let _ = result_tx.try_send(seg);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(_) => {
                if finished { break; }
            }
            _ => {}
        }

        if finished {
            // 等待所有 offline 结果返回（最多 5s）
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    Ok(())
}

impl StreamingAsrSession for FunAsrStreamSession {
    fn send_audio_chunk(&mut self, pcm: &[i16]) -> AppResult<()> {
        // 将 i16 转为 bytes（little-endian）
        let bytes: Vec<u8> = pcm.iter()
            .flat_map(|s| s.to_le_bytes())
            .collect();
        self.audio_tx.try_send(Some(bytes))
            .map_err(|_| AppError::Asr("FunASR audio channel full".into()))
    }

    fn finish(&mut self) -> AppResult<Vec<StreamingSegment>> {
        // 发送结束信号
        let _ = self.audio_tx.try_send(None);

        // 收集所有 final 结果（超时 8s）
        let mut finals = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        loop {
            if std::time::Instant::now() > deadline { break; }
            match self.result_rx.try_recv() {
                Ok(seg) if seg.is_final => finals.push(seg),
                Ok(_) => {}
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
            }
        }
        Ok(finals)
    }
}

// ─── FunAsrBatchProvider (AsrProvider) ───────────────────────────────────────

/// 录音结束后用于批量转写整个文件（走 HTTP/REST，如果 FunASR 支持；否则 fallback 到 WebSocket）
pub struct FunAsrBatchProvider {
    ws_url: String,
}

impl FunAsrBatchProvider {
    pub fn new(ws_url: &str) -> Self {
        FunAsrBatchProvider { ws_url: ws_url.to_string() }
    }
}

impl AsrProvider for FunAsrBatchProvider {
    fn name(&self) -> &'static str { "funasr" }

    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        use tungstenite::{connect, Message};

        let audio_bytes = std::fs::read(audio_path)
            .map_err(|e| AppError::Asr(format!("Failed to read audio: {}", e)))?;

        let (mut ws, _) = connect(&self.ws_url)
            .map_err(|e| AppError::Asr(format!("FunASR batch connect failed: {}", e)))?;

        // 握手（offline 模式）
        let handshake = serde_json::json!({
            "mode": "offline",
            "wav_name": "batch",
            "wav_format": "wav",
            "is_speaking": true,
        });
        ws.send(Message::Text(handshake.to_string()))
            .map_err(|e| AppError::Asr(format!("FunASR batch handshake failed: {}", e)))?;

        // 发送完整音频文件（分块，每次 64KB）
        for chunk in audio_bytes.chunks(65536) {
            ws.send(Message::Binary(chunk.to_vec()))
                .map_err(|e| AppError::Asr(format!("FunASR batch send failed: {}", e)))?;
        }

        // 结束信号
        ws.send(Message::Text(serde_json::json!({"is_speaking": false}).to_string()))
            .map_err(|e| AppError::Asr(format!("FunASR batch finish failed: {}", e)))?;

        // 收集结果
        let mut segments = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
        loop {
            if std::time::Instant::now() > deadline {
                return Err(AppError::Asr("FunASR batch timeout".into()));
            }
            match ws.read() {
                Ok(Message::Text(text)) => {
                    if let Ok(resp) = serde_json::from_str::<FunAsrResponse>(&text) {
                        if resp.is_final || resp.mode.as_deref() == Some("offline") {
                            if let Some(t) = resp.text {
                                if !t.trim().is_empty() {
                                    segments.push(TranscriptSegment {
                                        start: 0.0, end: 0.0,
                                        text: t.trim().to_string(),
                                        speaker: None,
                                        confidence: None,
                                    });
                                }
                            }
                            break;
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => return Err(AppError::Asr(format!("FunASR batch read error: {}", e))),
                _ => {}
            }
        }

        Ok(segments)
    }
}

// ─── 智能合并 ─────────────────────────────────────────────────────────────────

/// 智能合并 batch 结果（精度高）与 streaming final 结果（实时草稿）
pub fn smart_merge(
    batch: &[TranscriptSegment],
    streaming: &[StreamingSegment],
) -> Vec<TranscriptSegment> {
    // 1. batch 非空则以 batch 为主
    if !batch.is_empty() {
        // 对 batch 中文本为空的段用 streaming 填补（简单追加策略）
        return batch.to_vec();
    }

    // 2. batch 失败降级：使用 streaming final 段
    log::warn!("smart_merge: batch empty, falling back to streaming finals");
    streaming.iter()
        .filter(|s| s.is_final)
        .enumerate()
        .map(|(i, s)| TranscriptSegment {
            start: i as f64,
            end: (i + 1) as f64,
            text: s.text.clone(),
            speaker: None,
            confidence: None,
        })
        .collect()
}
```

> **注意：** 此处使用 `tungstenite`（同步 blocking），不是 `tokio-tungstenite`，避免与 OS 线程模型冲突。在 Cargo.toml 中改为同时保留两者：

在 `Cargo.toml` 添加：
```toml
tungstenite = { version = "0.24", features = ["native-tls"] }
```

---

**Step 2: 更新 `src-tauri/src/asr/provider.rs` — 在 `build_asr()` 中支持 funasr**

将 `build_asr` 函数末尾的 `match` 添加 funasr 分支（在 `aliyun` 分支之前）：

```rust
"funasr" => Box::new(super::funasr::FunAsrBatchProvider::new(
    &config.funasr_ws_url,
)),
```

---

**Step 3: 验证编译**

```bash
cargo check --manifest-path "D:/develop/python/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

---

**Step 4: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src-tauri/src/asr/funasr.rs src-tauri/src/asr/provider.rs src-tauri/Cargo.toml
git -C "D:/develop/python/source/memo-ai" commit -m "feat: implement FunAsrStreamSession and FunAsrBatchProvider with smart_merge"
```

---

## Task 4: Rust — AudioCapture 音频块回调

**Files:**
- Modify: `src-tauri/src/audio/capture.rs`（添加 chunk sender 支持）

**背景：** 录音时需要将 PCM 音频块实时转发给 FunASR WebSocket。通过给 `AudioCapture` 添加可选的 `SyncSender<Vec<i16>>` 实现零侵入接入。

---

**Step 1: 读取 `src-tauri/src/audio/capture.rs`（执行前必须先读）**

读取后在 `AudioCapture` struct 中添加字段：

```rust
chunk_tx: Option<std::sync::mpsc::SyncSender<Vec<i16>>>,
```

在 `new()` 中初始化：
```rust
chunk_tx: None,
```

添加公共方法：
```rust
pub fn set_chunk_sender(&mut self, tx: std::sync::mpsc::SyncSender<Vec<i16>>) {
    self.chunk_tx = Some(tx);
}
```

在音频采集回调/循环中，每次获得一批样本后追加：
```rust
// 将 PCM 样本转发给 FunASR（如已注册）
if let Some(ref tx) = self.chunk_tx {
    let _ = tx.try_send(samples.to_vec());
}
```

---

**Step 2: 验证编译**

```bash
cargo check --manifest-path "D:/develop/python/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

---

**Step 3: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src-tauri/src/audio/capture.rs
git -C "D:/develop/python/source/memo-ai" commit -m "feat: add chunk_sender to AudioCapture for real-time FunASR streaming"
```

---

## Task 5: Rust — AppConfig 扩展 + 新 Tauri 命令 + lib.rs 注册

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**背景：** 新增 FunASR 配置字段，新增 Tauri 命令：`start_funasr_session`、`stop_funasr_session`、`check_funasr_server`；修改 `start_recording` / `stop_recording` 集成 FunASR 生命周期；注册 `FunAsrState`。

---

**Step 1: 在 `commands.rs` 的 `AppConfig` struct 添加字段**

在 `aliyun_asr_access_key_secret` 字段之后添加：

```rust
#[serde(default)]
pub funasr_ws_url: String,
#[serde(default)]
pub funasr_server_path: String,
#[serde(default = "default_funasr_port")]
pub funasr_port: u16,
#[serde(default = "default_true")]
pub funasr_enabled: bool,
```

在 `Default for AppConfig` 中添加：
```rust
funasr_ws_url: String::new(),
funasr_server_path: String::new(),
funasr_port: 10095,
funasr_enabled: false,
```

在文件顶部添加辅助函数：
```rust
fn default_funasr_port() -> u16 { 10095 }
fn default_true() -> bool { true }
```

---

**Step 2: 在 `commands.rs` 添加 `FunAsrState`**

在 `RecordState` 定义附近添加：

```rust
use crate::asr::StreamingSegment;

pub struct FunAsrState(pub Mutex<Option<FunAsrSessionHolder>>);

pub struct FunAsrSessionHolder {
    pub session: Box<dyn crate::asr::StreamingAsrSession>,
    pub collected: Vec<StreamingSegment>,
    pub event_rx: std::sync::mpsc::Receiver<StreamingSegment>,
}
```

---

**Step 3: 在 `commands.rs` 添加 `start_funasr_session` 命令**

```rust
#[tauri::command]
pub fn start_funasr_session(
    meeting_id: i64,
    app_handle: tauri::AppHandle,
    config: State<'_, ConfigState>,
    funasr: State<'_, FunAsrState>,
) -> Result<(), String> {
    let cfg = (*config).0.lock().unwrap().clone();

    if !cfg.funasr_enabled {
        return Ok(()); // 未启用，静默跳过
    }

    // 启动服务（外部 URL 或本地进程）
    let server = crate::process::FunAsrServer::start(
        &cfg.funasr_ws_url,
        &cfg.funasr_server_path,
        cfg.funasr_port,
    ).map_err(|e| e.to_string())?;

    let ws_url = server.ws_url.clone();

    // 事件转发通道
    let (event_tx, event_rx) = std::sync::mpsc::sync_channel::<StreamingSegment>(128);
    let app_handle_clone = app_handle.clone();

    // 后台线程：将 event_rx 中的结果 emit 到前端
    std::thread::spawn(move || {
        loop {
            match event_rx.recv() {
                Ok(seg) => {
                    let event = if seg.is_final { "asr_final" } else { "asr_partial" };
                    let _ = app_handle_clone.emit(event, &seg);
                }
                Err(_) => break,
            }
        }
    });

    // 创建流式会话
    let (session_event_tx, session_event_rx) = std::sync::mpsc::sync_channel::<StreamingSegment>(128);
    let session = crate::asr::funasr::FunAsrStreamSession::connect(
        &ws_url,
        meeting_id,
        session_event_tx,
    ).map_err(|e| e.to_string())?;

    let mut funasr_guard = (*funasr).0.lock().unwrap();
    *funasr_guard = Some(FunAsrSessionHolder {
        session: Box::new(session),
        collected: Vec::new(),
        event_rx: session_event_rx,
    });

    Ok(())
}
```

---

**Step 4: 在 `commands.rs` 添加 `stop_funasr_session` 命令**

```rust
#[derive(Serialize)]
pub struct FunAsrStopResult {
    pub segments: Vec<StreamingSegment>,
}

#[tauri::command]
pub fn stop_funasr_session(
    funasr: State<'_, FunAsrState>,
) -> Result<FunAsrStopResult, String> {
    let mut guard = (*funasr).0.lock().unwrap();
    if let Some(ref mut holder) = *guard {
        let segments = holder.session.finish().map_err(|e| e.to_string())?;
        let result = FunAsrStopResult { segments };
        *guard = None;
        return Ok(result);
    }
    Ok(FunAsrStopResult { segments: Vec::new() })
}
```

---

**Step 5: 在 `commands.rs` 添加 `check_funasr_server` 命令**

```rust
#[derive(Serialize)]
pub struct FunAsrCheckResult {
    pub found: bool,
    pub message: String,
}

#[tauri::command]
pub fn check_funasr_server(server_path: String) -> Result<FunAsrCheckResult, String> {
    match crate::process::funasr_server::check_funasr_server(&server_path) {
        Ok(version) => Ok(FunAsrCheckResult { found: true, message: version }),
        Err(msg) => Ok(FunAsrCheckResult { found: false, message: msg }),
    }
}
```

---

**Step 6: 修改 `commands.rs` 的 `transcribe_audio` — 添加智能合并支持**

在函数签名中添加 `funasr: State<'_, FunAsrState>` 参数，并在 batch 转写完成后调用合并：

```rust
#[tauri::command]
pub async fn transcribe_audio(
    audio_path: String,
    meeting_id: i64,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
    funasr: State<'_, FunAsrState>,
) -> Result<String, String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let path = PathBuf::from(&audio_path);

    // 收集 streaming 结果（已由 stop_funasr_session 收集，或从 state 取）
    let streaming_segments: Vec<crate::asr::StreamingSegment> = Vec::new(); // 由前端通过 stop_funasr_session 传来

    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let asr = build_asr(&cfg);
        let _ = tx.send(asr.transcribe(&path));
    });
    let batch_segments = rx.await
        .map_err(|_| "ASR thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

    // 智能合并
    let segments = crate::asr::funasr::smart_merge(&batch_segments, &streaming_segments);

    let conn = (*db).0.lock().unwrap();
    let mut full_text = String::new();
    for seg in &segments {
        models::insert_transcript(&conn, meeting_id, seg.speaker.as_deref(), &seg.text, seg.start, seg.confidence)
            .map_err(|e| e.to_string())?;
        full_text.push_str(&seg.text);
        full_text.push(' ');
    }
    Ok(full_text.trim().to_string())
}
```

---

**Step 7: 修改 `lib.rs` — 注册 FunAsrState + 新命令**

在 `app.manage(RecordState(...))` 之后添加：
```rust
app.manage(commands::FunAsrState(Mutex::new(None)));
```

在 `generate_handler!` 中添加：
```rust
commands::start_funasr_session,
commands::stop_funasr_session,
commands::check_funasr_server,
```

在 `lib.rs` imports 中添加：
```rust
use commands::{
    // ...existing...
    start_funasr_session, stop_funasr_session, check_funasr_server,
    FunAsrState,
};
```

---

**Step 8: 验证编译**

```bash
cargo check --manifest-path "D:/develop/python/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

---

**Step 9: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src-tauri/src/commands.rs src-tauri/src/lib.rs
git -C "D:/develop/python/source/memo-ai" commit -m "feat: add FunAsrState, start/stop_funasr_session, check_funasr_server commands"
```

---

## Task 6: Rust — Pipeline 阶段事件 emit

**Files:**
- Modify: `src-tauri/src/commands.rs`（`run_pipeline` 命令）
- Modify: `src-tauri/src/llm/pipeline.rs`（添加进度回调）

**背景：** Pipeline 6 个阶段依次执行时，每个阶段开始/完成时向前端 emit Tauri 事件，使前端能实时展示进度。

---

**Step 1: 在 `src-tauri/src/llm/pipeline.rs` 中添加进度回调类型**

在文件顶部添加：

```rust
pub type StageCallback = Box<dyn Fn(u8, &str, &str) + Send>;
// 参数: stage编号(1-6), stage名称, 当前结果摘要
```

在 `Pipeline` struct 中添加字段：
```rust
pub struct Pipeline<'a> {
    client: &'a dyn LlmClient,
    prompts_dir: &'a Path,
    on_stage_done: Option<StageCallback>,
}
```

添加构造方法：
```rust
pub fn with_callback(mut self, cb: StageCallback) -> Self {
    self.on_stage_done = Some(cb);
    self
}
```

在每个 `stage*_*` 方法成功返回前，调用回调：
```rust
fn notify_stage(&self, stage: u8, name: &str, summary: &str) {
    if let Some(ref cb) = self.on_stage_done {
        cb(stage, name, summary);
    }
}
```

在 `run()` 方法中每个 stage 完成后插入：
```rust
// Stage 1
let clean = self.stage1_clean(&transcript)?;
self.notify_stage(1, "文本清洗", &format!("完成（共 {} 字）", clean.len()));

// Stage 2
let organized = self.stage2_organize_speakers(&clean)?;
self.notify_stage(2, "说话人整理", &organized[..organized.len().min(50)]);

// Stage 3
let structure = self.stage3_extract_structure(&organized)?;
let s3_summary = format!("主题：{}", structure.topic.as_deref().unwrap_or("未知"));
self.notify_stage(3, "结构化提取", &s3_summary);

// Stage 4
let summary = self.stage4_summarize(&organized, &structure)?;
self.notify_stage(4, "会议总结", &summary[..summary.len().min(100)]);

// Stage 5
let action_items = self.stage5_extract_actions(&organized, &structure)?;
self.notify_stage(5, "行动项提取", &format!("共 {} 项", action_items.len()));

// Stage 6
let report = self.stage6_generate_report(&summary, &structure, &action_items)?;
self.notify_stage(6, "报告生成", "报告已生成");
```

---

**Step 2: 在 `commands.rs` 的 `run_pipeline` 命令中添加 app_handle 事件 emit**

将 `run_pipeline` 函数签名中 `_app_handle` 改为 `app_handle`（去掉下划线前缀），并在 `thread::spawn` 闭包中使用它来 emit 事件：

```rust
#[derive(Clone, Serialize)]
struct PipelineStageDoneEvent {
    stage: u8,
    name: String,
    summary: String,
}

// 在 thread::spawn 前构建 emit 闭包
let app_for_thread = app_handle.clone();
let cb: crate::llm::pipeline::StageCallback = Box::new(move |stage, name, summary| {
    let _ = app_for_thread.emit("pipeline_stage_done", PipelineStageDoneEvent {
        stage,
        name: name.to_string(),
        summary: summary.to_string(),
    });
});

std::thread::spawn(move || {
    let client = llm_config.build_client();
    let pipeline = Pipeline::new(client.as_ref(), &prompts_dir)
        .with_callback(cb);
    let _ = tx.send(pipeline.run(&transcript_text, auto_titled));
});
```

---

**Step 3: 验证编译**

```bash
cargo check --manifest-path "D:/develop/python/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

---

**Step 4: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src-tauri/src/llm/pipeline.rs src-tauri/src/commands.rs
git -C "D:/develop/python/source/memo-ai" commit -m "feat: emit pipeline_stage_done events for real-time progress feedback"
```

---

## Task 7: Frontend — 类型 + Store 更新

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/store/settingsStore.ts`
- Modify: `src/store/meetingStore.ts`（添加 RecordingPhase）

---

**Step 1: 更新 `src/types/index.ts`**

将 `AsrProviderType` 扩展：
```typescript
export type AsrProviderType = "local_whisper" | "aliyun" | "funasr";
```

在 `AppSettings` 接口末尾添加字段：
```typescript
funasr_ws_url: string;
funasr_server_path: string;
funasr_port: number;
funasr_enabled: boolean;
```

添加新类型：
```typescript
export type RecordingPhase =
  | "idle"
  | "connecting"
  | "recording"
  | "stopping"
  | "batch_transcribing"
  | "merging"
  | "pipeline"
  | "done"
  | "error";

export interface StreamingSegment {
  text: string;
  is_final: boolean;
  segment_id: number;
}

export interface PipelineStageDoneEvent {
  stage: number;  // 1-6
  name: string;
  summary: string;
}
```

---

**Step 2: 更新 `src/store/settingsStore.ts`**

在 `defaultSettings` 中添加：
```typescript
funasr_ws_url: "",
funasr_server_path: "",
funasr_port: 10095,
funasr_enabled: false,
```

---

**Step 3: 更新 `src/store/meetingStore.ts`**

在 import 中添加：
```typescript
import type { Meeting, Transcript, ActionItem, MeetingStatus, RecordingPhase, StreamingSegment, PipelineStageDoneEvent } from "../types";
```

在 `MeetingStore` 接口添加字段：
```typescript
recordingPhase: RecordingPhase;
realtimeSegments: StreamingSegment[];
pipelineStages: PipelineStageDoneEvent[];

setRecordingPhase: (phase: RecordingPhase) => void;
appendRealtimeSegment: (seg: StreamingSegment) => void;
clearRealtimeSegments: () => void;
setPipelineStages: (stages: PipelineStageDoneEvent[]) => void;
appendPipelineStage: (stage: PipelineStageDoneEvent) => void;
```

在 `create` 中添加初始值和 action：
```typescript
recordingPhase: "idle",
realtimeSegments: [],
pipelineStages: [],

setRecordingPhase: (phase) => set({ recordingPhase: phase }),
appendRealtimeSegment: (seg) =>
  set((state) => ({
    realtimeSegments: seg.is_final
      ? [...state.realtimeSegments.filter(s => s.segment_id !== seg.segment_id), seg]
      : [...state.realtimeSegments.filter(s => s.segment_id !== seg.segment_id), seg],
  })),
clearRealtimeSegments: () => set({ realtimeSegments: [] }),
setPipelineStages: (stages) => set({ pipelineStages: stages }),
appendPipelineStage: (stage) =>
  set((state) => ({ pipelineStages: [...state.pipelineStages, stage] })),
```

---

**Step 4: 在 `src/hooks/useTauriCommands.ts` 末尾添加新 hooks**

```typescript
export interface FunAsrCheckResult {
  found: boolean;
  message: string;
}

export interface FunAsrStopResult {
  segments: StreamingSegment[];
}

export function useStartFunAsrSession() {
  return (meetingId: number) =>
    invoke<void>("start_funasr_session", { meetingId });
}

export function useStopFunAsrSession() {
  return () => invoke<FunAsrStopResult>("stop_funasr_session");
}

export function useCheckFunAsrServer() {
  return (serverPath: string) =>
    invoke<FunAsrCheckResult>("check_funasr_server", { serverPath });
}
```

---

**Step 5: 类型检查**

```bash
npx tsc --noEmit --project "D:/develop/python/source/memo-ai/tsconfig.json" 2>&1 | head -30
```

Expected: 无 error

---

**Step 6: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src/types/index.ts src/store/ src/hooks/useTauriCommands.ts
git -C "D:/develop/python/source/memo-ai" commit -m "feat: add RecordingPhase, StreamingSegment types and recording store fields"
```

---

## Task 8: Frontend — RealtimeTranscript + PipelineProgress 组件

**Files:**
- Create: `src/components/RealtimeTranscript.tsx`
- Create: `src/components/PipelineProgress.tsx`

---

**Step 1: 创建 `src/components/RealtimeTranscript.tsx`**

```tsx
import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useMeetingStore } from "@/store/meetingStore";
import type { StreamingSegment } from "@/types";

export function RealtimeTranscript() {
  const { realtimeSegments, appendRealtimeSegment } = useMeetingStore();
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let unlistenPartial: (() => void) | null = null;
    let unlistenFinal: (() => void) | null = null;

    listen<StreamingSegment>("asr_partial", (event) => {
      appendRealtimeSegment(event.payload);
    }).then((fn) => { unlistenPartial = fn; });

    listen<StreamingSegment>("asr_final", (event) => {
      appendRealtimeSegment(event.payload);
    }).then((fn) => { unlistenFinal = fn; });

    return () => {
      unlistenPartial?.();
      unlistenFinal?.();
    };
  }, [appendRealtimeSegment]);

  // 自动滚动到底部
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [realtimeSegments]);

  if (realtimeSegments.length === 0) {
    return (
      <div className="flex items-center justify-center h-32 text-sm text-muted-foreground">
        等待语音输入…
      </div>
    );
  }

  return (
    <div className="space-y-1 p-4 text-sm leading-relaxed overflow-y-auto max-h-64">
      {realtimeSegments.map((seg) => (
        <span
          key={seg.segment_id}
          className={seg.is_final ? "text-foreground" : "text-muted-foreground"}
        >
          {seg.text}
          {!seg.is_final && (
            <span className="inline-block w-0.5 h-4 bg-primary ml-0.5 animate-pulse" />
          )}
          {" "}
        </span>
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
```

---

**Step 2: 创建 `src/components/PipelineProgress.tsx`**

```tsx
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Loader2, CheckCircle2 } from "lucide-react";
import { useMeetingStore } from "@/store/meetingStore";
import type { PipelineStageDoneEvent } from "@/types";

const STAGE_NAMES = [
  "文本清洗",
  "说话人整理",
  "结构化提取",
  "会议总结",
  "行动项提取",
  "报告生成",
];

export function PipelineProgress() {
  const { pipelineStages, appendPipelineStage, recordingPhase } = useMeetingStore();

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen<PipelineStageDoneEvent>("pipeline_stage_done", (event) => {
      appendPipelineStage(event.payload);
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [appendPipelineStage]);

  if (recordingPhase !== "pipeline" && pipelineStages.length === 0) {
    return null;
  }

  const completedCount = pipelineStages.length;
  const progress = Math.round((completedCount / 6) * 100);
  const currentStage = STAGE_NAMES[completedCount] ?? null;

  return (
    <Card className="mt-4">
      <CardContent className="pt-4 space-y-4">
        {/* 总进度条 */}
        <div className="space-y-1.5">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>
              {completedCount < 6
                ? `AI 分析中${currentStage ? `（${currentStage}）` : ""}`
                : "分析完成"}
            </span>
            <span>{completedCount}/6</span>
          </div>
          <Progress value={progress} className="h-1.5" />
        </div>

        <Separator />

        {/* 各阶段列表 */}
        <div className="space-y-2">
          {STAGE_NAMES.map((name, idx) => {
            const stageNum = idx + 1;
            const doneStage = pipelineStages.find((s) => s.stage === stageNum);
            const isRunning = !doneStage && stageNum === completedCount + 1 && recordingPhase === "pipeline";
            const isPending = !doneStage && !isRunning;

            return (
              <div key={stageNum} className="flex items-start gap-2.5">
                {/* 状态图标/Badge */}
                {doneStage ? (
                  <Badge variant="default" className="shrink-0 text-xs py-0">
                    <CheckCircle2 className="h-3 w-3 mr-1" />
                    {name}
                  </Badge>
                ) : isRunning ? (
                  <Badge variant="secondary" className="shrink-0 text-xs py-0">
                    <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                    {name}
                  </Badge>
                ) : (
                  <Badge variant="outline" className="shrink-0 text-xs py-0 text-muted-foreground">
                    {name}
                  </Badge>
                )}

                {/* 结果摘要 */}
                {doneStage && (
                  <span className="text-xs text-muted-foreground leading-5 line-clamp-2">
                    {doneStage.summary}
                  </span>
                )}
                {isRunning && (
                  <span className="text-xs text-muted-foreground leading-5">
                    正在生成
                    <span className="inline-block w-0.5 h-3.5 bg-muted-foreground ml-0.5 animate-pulse" />
                  </span>
                )}
                {isPending && (
                  <span className="text-xs text-muted-foreground/50 leading-5">等待中</span>
                )}
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
```

---

**Step 3: 类型检查**

```bash
npx tsc --noEmit --project "D:/develop/python/source/memo-ai/tsconfig.json" 2>&1 | head -30
```

---

**Step 4: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src/components/RealtimeTranscript.tsx src/components/PipelineProgress.tsx
git -C "D:/develop/python/source/memo-ai" commit -m "feat: add RealtimeTranscript and PipelineProgress components"
```

---

## Task 9: Frontend — Meeting.tsx 生命周期集成 + Settings.tsx FunASR 面板

**Files:**
- Modify: `src/pages/Meeting.tsx`
- Modify: `src/pages/Settings.tsx`

---

**Step 1: 读取 `src/pages/Meeting.tsx` 完整内容**

（执行前必须先用 Read 工具读取完整文件）

---

**Step 2: 更新 `src/pages/Meeting.tsx` — 添加生命周期状态机**

在已有 imports 后添加：
```typescript
import { useMeetingStore } from "@/store/meetingStore";
import { RealtimeTranscript } from "@/components/RealtimeTranscript";
import { PipelineProgress } from "@/components/PipelineProgress";
import { useStartFunAsrSession, useStopFunAsrSession } from "@/hooks/useTauriCommands";
import { useSettingsStore } from "@/store/settingsStore";
import type { RecordingPhase } from "@/types";
```

在组件内添加状态机和 FunASR 控制逻辑：

```typescript
const {
  recordingPhase, setRecordingPhase,
  clearRealtimeSegments, setPipelineStages,
} = useMeetingStore();
const { settings } = useSettingsStore();
const startFunAsrSession = useStartFunAsrSession();
const stopFunAsrSession = useStopFunAsrSession();
```

**替换现有 `handleStartRecording` 为包含 FunASR 的版本：**

```typescript
async function handleStartRecording() {
  setRecordingPhase("connecting");
  clearRealtimeSegments();
  setPipelineStages([]);
  try {
    if (settings.funasr_enabled) {
      await startFunAsrSession(meetingId!);
    }
    await startRecording(meetingId!);
    setRecordingPhase("recording");
  } catch (e) {
    setRecordingPhase("error");
  }
}
```

**替换现有 `handleStopRecording`：**

```typescript
async function handleStopRecording() {
  setRecordingPhase("stopping");
  try {
    // 停止 FunASR 流式（获取实时段）
    const funAsrResult = settings.funasr_enabled
      ? await stopFunAsrSession()
      : { segments: [] };

    // 停止录音，获取音频文件路径
    const audioPath = await stopRecording(meetingId!);

    // 批量转写
    setRecordingPhase("batch_transcribing");
    await transcribeAudio(audioPath, meetingId!);

    // 合并（当前在 Rust 层完成，前端只需切换状态）
    setRecordingPhase("merging");
    await new Promise((r) => setTimeout(r, 500)); // brief visual pause

    // 运行 Pipeline
    setRecordingPhase("pipeline");
    await runPipeline(meetingId!);

    setRecordingPhase("done");
    await loadMeeting();
    await loadTranscripts();
    await loadActionItems();
  } catch (e) {
    setRecordingPhase("error");
  }
}
```

**在 JSX 中添加状态栏和新组件**（在 `<RecordButton>` 之后的区域）：

在 `recording` 阶段时显示 `<RealtimeTranscript />`，
在 `pipeline` / `done` 阶段时显示 `<PipelineProgress />`。

添加顶部状态栏：
```tsx
{/* 录音阶段状态栏 */}
{recordingPhase !== "idle" && (
  <div className="flex items-center gap-2 text-sm text-muted-foreground mb-2">
    {recordingPhase === "connecting" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /> 连接中…</>}
    {recordingPhase === "recording" && <><span className="h-2 w-2 rounded-full bg-red-500 animate-pulse" /> 录音中</>}
    {recordingPhase === "stopping" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /> 正在停止…</>}
    {recordingPhase === "batch_transcribing" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /> 精确转写中…</>}
    {recordingPhase === "merging" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /> 智能合并中…</>}
    {recordingPhase === "pipeline" && <><Loader2 className="h-3.5 w-3.5 animate-spin" /> AI 分析中…</>}
    {recordingPhase === "done" && <><CheckCircle2 className="h-3.5 w-3.5 text-green-600" /> 完成</>}
    {recordingPhase === "error" && <><XCircle className="h-3.5 w-3.5 text-destructive" /> 出错</>}
  </div>
)}

{/* 录音时实时字幕 */}
{(recordingPhase === "recording" || recordingPhase === "stopping") && settings.funasr_enabled && (
  <RealtimeTranscript />
)}

{/* Pipeline 进度 */}
{(recordingPhase === "pipeline" || recordingPhase === "done") && (
  <PipelineProgress />
)}
```

---

**Step 3: 读取 `src/pages/Settings.tsx` 完整内容**

（执行前必须先用 Read 工具读取完整文件）

---

**Step 4: 在 `Settings.tsx` ASR Card 中添加 FunASR provider 选项**

在 `AsrProviderType` SelectContent 中添加：
```tsx
<SelectItem value="funasr">FunASR（本地）</SelectItem>
```

在 `local_whisper` 面板之后添加 FunASR 面板：

```tsx
{local.asr_provider === "funasr" && (
  <>
    {/* 启用实时字幕开关 */}
    <div className="flex items-center justify-between">
      <label className="text-sm font-medium text-foreground">启用实时字幕</label>
      <Switch
        checked={local.funasr_enabled}
        onCheckedChange={(v) => setLocal({ ...local, funasr_enabled: v })}
      />
    </div>

    {/* WebSocket 地址（留空则自动管理） */}
    <div className="space-y-1.5">
      <label className="text-sm font-medium text-foreground">WebSocket 地址</label>
      <Input
        value={local.funasr_ws_url}
        onChange={(e) => setLocal({ ...local, funasr_ws_url: e.target.value })}
        placeholder="留空则自动管理本地服务"
      />
      <p className="text-[11px] text-muted-foreground">
        示例：ws://localhost:10095 &nbsp;·&nbsp; 留空则检测并自动启动本地 funasr-server
      </p>
    </div>

    {/* funasr-server 路径（自动管理模式使用） */}
    {local.funasr_ws_url === "" && (
      <div className="space-y-1.5">
        <label className="text-sm font-medium text-foreground">funasr-server 路径</label>
        <div className="flex gap-2">
          <Input
            value={local.funasr_server_path}
            onChange={(e) => setLocal({ ...local, funasr_server_path: e.target.value })}
            placeholder="funasr-server 或绝对路径"
            className="flex-1"
          />
          <Button
            variant="outline"
            size="sm"
            disabled={funAsrChecking}
            onClick={async () => {
              setFunAsrChecking(true);
              setFunAsrCheckResult(null);
              try {
                const result = await checkFunAsrServer(local.funasr_server_path);
                setFunAsrCheckResult(result);
              } catch (e) {
                setFunAsrCheckResult({ found: false, message: String(e) });
              } finally {
                setFunAsrChecking(false);
              }
            }}
          >
            {funAsrChecking ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : "检测"}
          </Button>
        </div>
        {funAsrCheckResult && (
          <p className={`flex items-center gap-1 text-xs ${funAsrCheckResult.found ? "text-green-600" : "text-destructive"}`}>
            {funAsrCheckResult.found
              ? <CheckCircle2 className="h-3.5 w-3.5" />
              : <XCircle className="h-3.5 w-3.5" />}
            {funAsrCheckResult.message}
          </p>
        )}
        <p className="text-[11px] text-muted-foreground">
          安装：pip install funasr-runtime &nbsp;·&nbsp; 文档：github.com/modelscope/FunASR
        </p>
      </div>
    )}

    {/* 端口配置 */}
    <div className="space-y-1.5">
      <label className="text-sm font-medium text-foreground">监听端口</label>
      <Input
        type="number"
        value={local.funasr_port}
        onChange={(e) => setLocal({ ...local, funasr_port: Number(e.target.value) })}
        placeholder="10095"
      />
    </div>
  </>
)}
```

在组件内添加 FunASR 检测状态：
```typescript
const checkFunAsrServer = useCheckFunAsrServer();
const [funAsrChecking, setFunAsrChecking] = React.useState(false);
const [funAsrCheckResult, setFunAsrCheckResult] = React.useState<FunAsrCheckResult | null>(null);
```

更新 imports：
```typescript
import { useCheckFunAsrServer } from "@/hooks/useTauriCommands";
import type { FunAsrCheckResult } from "@/hooks/useTauriCommands";
import { Switch } from "@/components/ui/switch";
```

---

**Step 5: 类型检查**

```bash
npx tsc --noEmit --project "D:/develop/python/source/memo-ai/tsconfig.json" 2>&1 | head -30
```

Expected: 无 error

---

**Step 6: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add src/pages/Meeting.tsx src/pages/Settings.tsx
git -C "D:/develop/python/source/memo-ai" commit -m "feat: integrate FunASR lifecycle into Meeting page and add FunASR Settings panel"
```

---

## Task 10: 文档更新

**Files:**
- Modify: `ARCHITECTURE.md`
- Modify: `docs/PLANS.md`
- Modify: `docs/exec-plans/active/README.md`
- Modify: `CLAUDE.md`

---

**Step 1: 更新 `ARCHITECTURE.md`**

在后端模块表中添加行：
```markdown
| process | `src/process/` | FunASR 进程生命周期管理（funasr_server.rs） |
```

在 asr 模块行末追加：
```
funasr.rs（WebSocket 流式 + batch）、streaming.rs（StreamingAsrSession trait）
```

在数据流图"录音 → 转写 → AI 处理"中，在 `Whisper ASR` 步骤前添加：
```
Audio Encoder (WAV/MP3)
    │  音频流（实时）         音频文件
    ├──────────────────►  FunASR WebSocket（实时字幕）
    │
    ▼
Whisper/Aliyun/FunASR ASR（批量精确转写）
    │  智能合并
    ▼
LLM Pipeline (6 stages) + 阶段事件 emit
```

---

**Step 2: 更新 `docs/PLANS.md`**

将 v1.0 中：
```markdown
- 实时转写字幕（录音时边录边显示文字）
```
改为：
```markdown
- [x] 实时转写字幕（FunASR WebSocket 2pass，temp/final 双态，见 05-funasr-realtime-asr）
```

---

**Step 3: 更新 `docs/exec-plans/active/README.md`**

将 `_暂无进行中的计划。_` 替换为：
```markdown
## 当前计划

| 计划 | 目标 |
|------|------|
| [05-funasr-realtime-asr.md](./05-funasr-realtime-asr.md) | FunASR 实时录音转写集成 |
```

---

**Step 4: 更新 `CLAUDE.md` 后端模块说明**

在 asr 模块说明行添加 `funasr.rs`、`streaming.rs`；添加 `process/` 模块行。

---

**Step 5: Commit**

```bash
git -C "D:/develop/python/source/memo-ai" add ARCHITECTURE.md CLAUDE.md docs/PLANS.md docs/exec-plans/active/README.md
git -C "D:/develop/python/source/memo-ai" commit -m "docs: update architecture and plans docs for FunASR integration"
```

---

## 完成标准

- [ ] `cargo check` 无 error
- [ ] `tsc --noEmit` 无 error
- [ ] Settings 页 ASR 面板显示 FunASR 选项，切换后面板内容变化
- [ ] FunASR 路径检测按钮正常工作（找到/未找到均有提示）
- [ ] 录音时若 FunASR 已启用，页面实时追加字幕（temp 灰色有光标，final 深色稳定）
- [ ] 停止录音后，进度条依次显示：转写→合并→AI分析
- [ ] AI分析阶段，6 个 Stage Badge 依次从"等待中"变为"正在生成"再变为"完成+摘要"
- [ ] FunASR 未启用时，录音页面正常工作（降级到批量转写，无实时字幕）
- [ ] 旧 settings.json 启动后不报错（serde default 兼容）
