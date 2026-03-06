# ASR 可插拔后端 + LLM 连接测试 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 ASR 层重构为可插拔 trait 架构（本地 Whisper CLI / 阿里云 ASR），新增 LLM 连接测试命令，升级 Settings UI（ASR Provider 切换面板 + Whisper 检测向导 + 阿里云配置面板 + LLM 测试按钮）。

**Architecture:** 参照现有 `LlmClient` trait 模式，新增 `AsrProvider` trait 和 `build_asr()` 工厂函数；`AppConfig` 扁平化新增 `asr_provider` 和阿里云字段；`test_llm_connection` 命令接收表单值（非已保存配置），支持保存前验证；Settings UI 动态切换 ASR 面板。

**Tech Stack:** Rust（reqwest blocking + multipart、serde_json）、React + TypeScript、shadcn/ui、lucide-react

**设计文档：** [docs/plans/2026-03-06-asr-pluggable-llm-test-design.md](../../plans/2026-03-06-asr-pluggable-llm-test-design.md)

---

## Task 1: ASR trait 定义 + WhisperAsr 迁移

**Files:**
- Create: `src-tauri/src/asr/provider.rs`
- Modify: `src-tauri/src/asr/whisper.rs`
- Modify: `src-tauri/src/asr/mod.rs`
- Modify: `src-tauri/src/commands.rs`（AppConfig 新增字段 + transcribe_audio 改用工厂）

**背景：** 当前 `WhisperAsr` 有独立的 `transcribe` 方法，未实现 trait。`commands.rs` 直接构造 `WhisperAsr`。本 Task 引入 `AsrProvider` trait 并迁移 Whisper 实现，保持现有行为不变。

---

**Step 1: 创建 `src-tauri/src/asr/provider.rs`**

```rust
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
```

---

**Step 2: 修改 `src-tauri/src/asr/whisper.rs`，实现 `AsrProvider` trait**

在文件顶部添加 trait use，并为 `WhisperAsr` 实现 `AsrProvider`，将现有 `transcribe` 方法改为 trait 实现：

```rust
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
    fn name(&self) -> &'static str {
        "local_whisper"
    }

    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
        let output = std::process::Command::new(&self.cli_path)
            .arg("-f")
            .arg(audio_path.to_str().unwrap_or(""))
            .arg("-m")
            .arg(&self.model_path)
            .arg("-l")
            .arg(&self.language)
            .arg("--output-format")
            .arg("json")
            .arg("--no-timestamps")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let json_str = String::from_utf8_lossy(&out.stdout);
                parse_whisper_json(&json_str)
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr);
                Err(AppError::Asr(format!("whisper-cli error: {}", err)))
            }
            Err(e) => Err(AppError::Asr(format!(
                "Cannot run whisper-cli at '{}': {}. Download from https://github.com/ggerganov/whisper.cpp/releases",
                self.cli_path, e
            ))),
        }
    }
}

fn parse_whisper_json(json_str: &str) -> AppResult<Vec<TranscriptSegment>> {
    #[derive(serde::Deserialize)]
    struct WSegment {
        #[serde(default)]
        start: f64,
        #[serde(default)]
        end: f64,
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
        .map(|s| TranscriptSegment {
            start: s.start,
            end: s.end,
            text: s.text.trim().to_string(),
            speaker: None,
            confidence: None,
        })
        .collect())
}
```

---

**Step 3: 更新 `src-tauri/src/asr/mod.rs`**

```rust
pub mod provider;
pub mod transcript;
pub mod whisper;
pub mod aliyun;

pub use provider::{AsrProvider, build_asr};
```

注意：`aliyun` 模块在 Task 2 中创建，此处先声明（Task 2 前编译会报错，属正常）。暂时可用空文件占位：创建 `src-tauri/src/asr/aliyun.rs` 内容为空模块体：

```rust
// placeholder — implemented in Task 2
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
```

---

**Step 4: 更新 `src-tauri/src/commands.rs` — AppConfig 新增字段**

将现有 `AppConfig` struct 替换为：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm_provider: LlmProviderConfig,
    pub whisper_model: String,
    pub language: String,
    pub whisper_cli_path: String,
    pub whisper_model_dir: String,
    #[serde(default = "default_asr_provider")]
    pub asr_provider: String,
    #[serde(default)]
    pub aliyun_asr_app_key: String,
    #[serde(default)]
    pub aliyun_asr_access_key_id: String,
    #[serde(default)]
    pub aliyun_asr_access_key_secret: String,
}

fn default_asr_provider() -> String {
    "local_whisper".into()
}
```

在 `impl Default for AppConfig` 中添加新字段默认值：

```rust
asr_provider: "local_whisper".into(),
aliyun_asr_app_key: String::new(),
aliyun_asr_access_key_id: String::new(),
aliyun_asr_access_key_secret: String::new(),
```

---

**Step 5: 更新 `transcribe_audio` 命令使用 `build_asr()`**

将：
```rust
use crate::asr::whisper::WhisperAsr;
```
改为：
```rust
use crate::asr::build_asr;
```

将 `transcribe_audio` 函数体中的：
```rust
let model_path = format!("{}/ggml-{}.bin", cfg.whisper_model_dir, cfg.whisper_model);
let asr = WhisperAsr::new(&cfg.whisper_cli_path, &model_path, &cfg.language);
let path = PathBuf::from(&audio_path);
let segments = asr.transcribe(&path).map_err(|e| e.to_string())?;
```
替换为：
```rust
let asr = build_asr(&cfg);
let path = PathBuf::from(&audio_path);
let segments = asr.transcribe(&path).map_err(|e| e.to_string())?;
```

---

**Step 6: 验证编译**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

Expected: `Finished` 无 error

---

**Step 7: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/asr/
git -C "D:/project/java/source/memo-ai" add src-tauri/src/commands.rs
git -C "D:/project/java/source/memo-ai" commit -m "refactor: introduce AsrProvider trait, migrate WhisperAsr, add build_asr factory"
```

---

## Task 2: 阿里云 ASR 实现

**Files:**
- Modify: `src-tauri/src/asr/aliyun.rs`（替换 Task 1 占位实现）
- Modify: `src-tauri/Cargo.toml`（reqwest 添加 multipart feature）

**背景：** 使用阿里云「录音文件识别极速版」（Flash File Recognition）REST API。调用流程：
1. 用 AccessKeyId + AccessKeySecret 获取 NLS Token
2. POST 音频文件二进制数据到 Flash 识别接口
3. 解析返回的 JSON 结果（同步，无需轮询）

API 端点：
- Token: `POST https://nls-gateway.cn-shanghai.aliyuncs.com/token`
- Flash: `POST https://nls-gateway.cn-shanghai.aliyuncs.com/api/v1/recognition/flash?appkey={appkey}&format=wav&sample_rate=16000`

---

**Step 1: 在 `Cargo.toml` 为 reqwest 添加 multipart feature**

将：
```toml
reqwest = { version = "0.12", features = ["json", "blocking"] }
```
改为：
```toml
reqwest = { version = "0.12", features = ["json", "blocking", "multipart"] }
```

---

**Step 2: 替换 `src-tauri/src/asr/aliyun.rs` 为完整实现**

```rust
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
                ("appkey", &self.app_key),
                ("secretKey", &self.access_key_secret),
            ])
            .header("X-NLS-AccessKeyId", &self.access_key_id)
            .header("X-NLS-Timestamp", ts.to_string())
            .send()
            .map_err(|e| AppError::Asr(format!("Aliyun token request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(AppError::Asr(format!(
                "Aliyun token error HTTP {}: {}",
                status, body
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

        // Detect format from extension, default to wav
        let format = audio_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("wav");

        let url = format!(
            "https://nls-gateway.cn-shanghai.aliyuncs.com/api/v1/recognition/flash?appkey={}&format={}&sample_rate=16000",
            self.app_key, format
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
            return Err(AppError::Asr(format!(
                "Aliyun ASR error HTTP {}: {}",
                status, body
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
```

---

**Step 3: 验证编译**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
```

Expected: `Finished` 无 error

---

**Step 4: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/asr/aliyun.rs src-tauri/Cargo.toml
git -C "D:/project/java/source/memo-ai" commit -m "feat: implement AliyunAsr with Flash Recognition API"
```

---

## Task 3: LLM 连接测试命令 + Settings UI 测试按钮

**Files:**
- Modify: `src-tauri/src/commands.rs`（新增 `test_llm_connection` 命令）
- Modify: `src-tauri/src/lib.rs`（注册命令）
- Modify: `src/hooks/useTauriCommands.ts`（新增 hook）
- Modify: `src/pages/Settings.tsx`（新增测试按钮 + 状态显示）

---

**Step 1: 在 `commands.rs` 末尾添加 `test_llm_connection` 命令**

在 `get_settings` 之前插入：

```rust
#[derive(Serialize)]
pub struct LlmTestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: u64,
}

#[tauri::command]
pub fn test_llm_connection(settings: AppConfig) -> Result<LlmTestResult, String> {
    use std::time::Instant;
    use crate::llm::client::LlmConfig;

    let cfg = LlmConfig {
        provider: settings.llm_provider.provider_type.clone(),
        base_url: settings.llm_provider.base_url.clone(),
        model: settings.llm_provider.model.clone(),
        api_key: settings.llm_provider.api_key.clone(),
    };

    let client = cfg.build_client();
    let start = Instant::now();

    match client.complete("Hi") {
        Ok(_) => {
            let ms = start.elapsed().as_millis() as u64;
            Ok(LlmTestResult {
                success: true,
                message: format!("连接正常 ({}ms)", ms),
                latency_ms: ms,
            })
        }
        Err(e) => {
            let msg = e.to_string();
            // Friendly error classification
            let friendly = if msg.contains("Connection refused") || msg.contains("connect error") {
                format!("无法连接到 {}，请确认服务已启动", settings.llm_provider.base_url)
            } else if msg.contains("401") || msg.contains("Unauthorized") {
                "API Key 无效，请检查配置".to_string()
            } else if msg.contains("model") && msg.contains("not found") {
                format!("模型 '{}' 不存在，请确认模型名称", settings.llm_provider.model)
            } else {
                msg
            };
            Ok(LlmTestResult {
                success: false,
                message: friendly,
                latency_ms: 0,
            })
        }
    }
}
```

---

**Step 2: 在 `src-tauri/src/lib.rs` 的 `generate_handler!` 中注册命令**

找到 `tauri::generate_handler![...]`，添加：
```rust
commands::test_llm_connection,
```

---

**Step 3: 在 `src/hooks/useTauriCommands.ts` 末尾添加 hook**

```typescript
export interface LlmTestResult {
  success: boolean;
  message: string;
  latency_ms: number;
}

export function useTestLlmConnection() {
  return (settings: AppSettings) =>
    invoke<LlmTestResult>("test_llm_connection", { settings });
}
```

---

**Step 4: 更新 `src/pages/Settings.tsx` LLM Card — 添加测试按钮**

在文件顶部 import 区添加：
```typescript
import { useTestLlmConnection } from "@/hooks/useTauriCommands";
import type { LlmTestResult } from "@/hooks/useTauriCommands";
import { Loader2, CheckCircle2, XCircle } from "lucide-react";
```

在 `Settings` 组件内，`saveSettings` hook 之后添加状态：
```typescript
const testLlmConnection = useTestLlmConnection();
type TestStatus = "idle" | "testing" | "ok" | "fail";
const [llmTestStatus, setLlmTestStatus] = React.useState<TestStatus>("idle");
const [llmTestResult, setLlmTestResult] = React.useState<LlmTestResult | null>(null);
```

在 Provider Select 的 `onValueChange` 回调中，添加重置测试状态：
```typescript
onValueChange={(v) => {
  setLocal({ ...local, llm_provider: { ...local.llm_provider, type: v as "ollama" | "openai" } });
  setLlmTestStatus("idle");
  setLlmTestResult(null);
}}
```

在 LLM Card 的 `</CardContent>` 之前（API Key 字段之后）添加测试行：

```tsx
<div className="flex items-center gap-3 pt-1">
  <Button
    variant="outline"
    size="sm"
    disabled={llmTestStatus === "testing"}
    onClick={async () => {
      setLlmTestStatus("testing");
      setLlmTestResult(null);
      try {
        const result = await testLlmConnection(local);
        setLlmTestResult(result);
        setLlmTestStatus(result.success ? "ok" : "fail");
      } catch (e) {
        setLlmTestResult({ success: false, message: String(e), latency_ms: 0 });
        setLlmTestStatus("fail");
      }
    }}
  >
    {llmTestStatus === "testing" ? (
      <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
    ) : null}
    测试连接
  </Button>
  {llmTestStatus === "ok" && llmTestResult && (
    <span className="flex items-center gap-1 text-xs text-green-600">
      <CheckCircle2 className="h-3.5 w-3.5" />
      {llmTestResult.message}
    </span>
  )}
  {llmTestStatus === "fail" && llmTestResult && (
    <span className="flex items-center gap-1 text-xs text-destructive">
      <XCircle className="h-3.5 w-3.5" />
      {llmTestResult.message}
    </span>
  )}
</div>
```

---

**Step 5: 验证编译 + 类型检查**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -20
```

Expected: 均无 error

---

**Step 6: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/commands.rs src-tauri/src/lib.rs src/hooks/useTauriCommands.ts src/pages/Settings.tsx
git -C "D:/project/java/source/memo-ai" commit -m "feat: add test_llm_connection command and test button in Settings"
```

---

## Task 4: ASR Settings UI — Provider 切换 + Whisper 向导 + 阿里云配置面板

**Files:**
- Modify: `src-tauri/src/commands.rs`（新增 `check_whisper_cli` 和 `test_asr_connection` 命令）
- Modify: `src-tauri/src/lib.rs`（注册命令）
- Modify: `src/types/index.ts`（AppSettings 扩展）
- Modify: `src/store/settingsStore.ts`（默认值）
- Modify: `src/hooks/useTauriCommands.ts`（新增两个 hook）
- Modify: `src/pages/Settings.tsx`（ASR Card 完整重写）

---

**Step 1: 在 `commands.rs` 添加 `check_whisper_cli` 命令**

```rust
#[derive(Serialize)]
pub struct WhisperCheckResult {
    pub found: bool,
    pub version: Option<String>,
    pub message: String,
}

#[tauri::command]
pub fn check_whisper_cli(cli_path: String) -> Result<WhisperCheckResult, String> {
    match std::process::Command::new(&cli_path)
        .arg("--version")
        .output()
    {
        Ok(out) if out.status.success() || !out.stdout.is_empty() || !out.stderr.is_empty() => {
            // whisper-cli prints version to stderr
            let version_raw = String::from_utf8_lossy(&out.stderr)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            let version = if version_raw.is_empty() {
                None
            } else {
                Some(version_raw)
            };
            Ok(WhisperCheckResult {
                found: true,
                version,
                message: format!("找到 whisper-cli: {}", cli_path),
            })
        }
        Ok(_) => Ok(WhisperCheckResult {
            found: false,
            version: None,
            message: format!("找到可执行文件但运行异常: {}", cli_path),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(WhisperCheckResult {
            found: false,
            version: None,
            message: format!(
                "未找到 whisper-cli。请从 github.com/ggerganov/whisper.cpp/releases 下载"
            ),
        }),
        Err(e) => Ok(WhisperCheckResult {
            found: false,
            version: None,
            message: format!("检测失败: {}", e),
        }),
    }
}
```

---

**Step 2: 在 `commands.rs` 添加 `test_asr_connection` 命令**

```rust
#[derive(Serialize)]
pub struct AsrTestResult {
    pub success: bool,
    pub message: String,
}

#[tauri::command]
pub fn test_asr_connection(settings: AppConfig) -> Result<AsrTestResult, String> {
    match settings.asr_provider.as_str() {
        "aliyun" => {
            match crate::asr::aliyun::test_connection(
                &settings.aliyun_asr_app_key,
                &settings.aliyun_asr_access_key_id,
                &settings.aliyun_asr_access_key_secret,
            ) {
                Ok(msg) => Ok(AsrTestResult { success: true, message: msg }),
                Err(e) => Ok(AsrTestResult { success: false, message: e }),
            }
        }
        _ => Ok(AsrTestResult {
            success: false,
            message: "当前 ASR Provider 无需测试".to_string(),
        }),
    }
}
```

---

**Step 3: 在 `lib.rs` 中注册两个新命令**

在 `generate_handler!` 中添加：
```rust
commands::check_whisper_cli,
commands::test_asr_connection,
```

---

**Step 4: 更新 `src/types/index.ts` — AppSettings 扩展**

将 `AppSettings` 接口替换为：

```typescript
export type AsrProviderType = "local_whisper" | "aliyun";

export interface AppSettings {
  llm_provider: LlmProvider;
  whisper_model: string;
  language: string;
  whisper_cli_path: string;
  whisper_model_dir: string;
  asr_provider: AsrProviderType;
  aliyun_asr_app_key: string;
  aliyun_asr_access_key_id: string;
  aliyun_asr_access_key_secret: string;
}
```

---

**Step 5: 更新 `src/store/settingsStore.ts` — 新增默认值**

读取 `src/store/settingsStore.ts`，在 `settings` 默认对象中添加：

```typescript
asr_provider: "local_whisper" as AsrProviderType,
aliyun_asr_app_key: "",
aliyun_asr_access_key_id: "",
aliyun_asr_access_key_secret: "",
```

同时在顶部 import 中添加 `AsrProviderType`：
```typescript
import type { AppSettings, AsrProviderType } from "@/types";
```

---

**Step 6: 在 `src/hooks/useTauriCommands.ts` 添加新 hook**

在文件末尾添加：

```typescript
export interface WhisperCheckResult {
  found: boolean;
  version: string | null;
  message: string;
}

export interface AsrTestResult {
  success: boolean;
  message: string;
}

export function useCheckWhisperCli() {
  return (cliPath: string) =>
    invoke<WhisperCheckResult>("check_whisper_cli", { cliPath });
}

export function useTestAsrConnection() {
  return (settings: AppSettings) =>
    invoke<AsrTestResult>("test_asr_connection", { settings });
}
```

---

**Step 7: 重写 `src/pages/Settings.tsx` ASR Card**

在 imports 顶部添加：
```typescript
import { useCheckWhisperCli, useTestAsrConnection } from "@/hooks/useTauriCommands";
import type { WhisperCheckResult, AsrTestResult } from "@/hooks/useTauriCommands";
import type { AsrProviderType } from "@/types";
import { Eye, EyeOff } from "lucide-react";
```

在 `Settings` 组件内（`testLlmConnection` 之后）添加新状态：

```typescript
const checkWhisperCli = useCheckWhisperCli();
const testAsrConnection = useTestAsrConnection();
const [whisperCheck, setWhisperCheck] = React.useState<WhisperCheckResult | null>(null);
const [whisperChecking, setWhisperChecking] = React.useState(false);
const [asrTestResult, setAsrTestResult] = React.useState<AsrTestResult | null>(null);
const [asrTesting, setAsrTesting] = React.useState(false);
const [showAliyunSecret, setShowAliyunSecret] = React.useState(false);
```

将现有 ASR Card（从 `<Card>` 到 `</Card>` 的 ASR 配置部分）完整替换为：

```tsx
<Card>
  <CardHeader>
    <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
      ASR 配置
    </CardTitle>
  </CardHeader>
  <Separator />
  <CardContent className="space-y-4">
    {/* Provider 选择 */}
    <div className="space-y-1.5">
      <label className="text-sm font-medium text-foreground">ASR 引擎</label>
      <Select
        value={local.asr_provider}
        onValueChange={(v) => {
          setLocal({ ...local, asr_provider: v as AsrProviderType });
          setWhisperCheck(null);
          setAsrTestResult(null);
        }}
      >
        <SelectTrigger>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="local_whisper">本地 Whisper</SelectItem>
          <SelectItem value="aliyun">阿里云 ASR</SelectItem>
        </SelectContent>
      </Select>
    </div>

    {/* 识别语言（公共） */}
    <div className="space-y-1.5">
      <label className="text-sm font-medium text-foreground">识别语言</label>
      <Select
        value={local.language}
        onValueChange={(v) => setLocal({ ...local, language: v })}
      >
        <SelectTrigger>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="zh">中文</SelectItem>
          <SelectItem value="en">English</SelectItem>
          <SelectItem value="auto">自动检测</SelectItem>
        </SelectContent>
      </Select>
    </div>

    {/* 本地 Whisper 面板 */}
    {local.asr_provider === "local_whisper" && (
      <>
        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">Whisper 模型</label>
          <Select
            value={local.whisper_model}
            onValueChange={(v) => setLocal({ ...local, whisper_model: v })}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="tiny">tiny（最快）</SelectItem>
              <SelectItem value="base">base（推荐）</SelectItem>
              <SelectItem value="small">small</SelectItem>
              <SelectItem value="medium">medium</SelectItem>
              <SelectItem value="large">large（最准）</SelectItem>
            </SelectContent>
          </Select>
        </div>

        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">whisper-cli 路径</label>
          <div className="flex gap-2">
            <Input
              value={local.whisper_cli_path}
              onChange={(e) => {
                setLocal({ ...local, whisper_cli_path: e.target.value });
                setWhisperCheck(null);
              }}
              placeholder="whisper-cli 或绝对路径"
              className="flex-1"
            />
            <Button
              variant="outline"
              size="sm"
              disabled={whisperChecking}
              onClick={async () => {
                setWhisperChecking(true);
                setWhisperCheck(null);
                try {
                  const result = await checkWhisperCli(local.whisper_cli_path);
                  setWhisperCheck(result);
                } catch (e) {
                  setWhisperCheck({ found: false, version: null, message: String(e) });
                } finally {
                  setWhisperChecking(false);
                }
              }}
            >
              {whisperChecking ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : "检测"}
            </Button>
          </div>
          {whisperCheck && (
            <p className={`flex items-center gap-1 text-xs ${whisperCheck.found ? "text-green-600" : "text-destructive"}`}>
              {whisperCheck.found ? (
                <CheckCircle2 className="h-3.5 w-3.5" />
              ) : (
                <XCircle className="h-3.5 w-3.5" />
              )}
              {whisperCheck.found && whisperCheck.version
                ? `${whisperCheck.version}`
                : whisperCheck.message}
            </p>
          )}
          {!whisperCheck && (
            <p className="text-[11px] text-muted-foreground">
              下载：github.com/ggerganov/whisper.cpp/releases
            </p>
          )}
        </div>

        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">模型文件目录</label>
          <Input
            value={local.whisper_model_dir}
            onChange={(e) => setLocal({ ...local, whisper_model_dir: e.target.value })}
            placeholder="models"
          />
          <p className="text-[11px] text-muted-foreground">
            存放 ggml-*.bin 模型文件的目录路径
          </p>
        </div>
      </>
    )}

    {/* 阿里云 ASR 面板 */}
    {local.asr_provider === "aliyun" && (
      <>
        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">AppKey</label>
          <Input
            value={local.aliyun_asr_app_key}
            onChange={(e) => setLocal({ ...local, aliyun_asr_app_key: e.target.value })}
            placeholder="项目 AppKey"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">AccessKey ID</label>
          <Input
            value={local.aliyun_asr_access_key_id}
            onChange={(e) => setLocal({ ...local, aliyun_asr_access_key_id: e.target.value })}
            placeholder="AccessKey ID"
          />
        </div>

        <div className="space-y-1.5">
          <label className="text-sm font-medium text-foreground">AccessKey Secret</label>
          <div className="flex gap-2">
            <Input
              type={showAliyunSecret ? "text" : "password"}
              value={local.aliyun_asr_access_key_secret}
              onChange={(e) => setLocal({ ...local, aliyun_asr_access_key_secret: e.target.value })}
              placeholder="AccessKey Secret"
              className="flex-1"
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowAliyunSecret((v) => !v)}
            >
              {showAliyunSecret ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </Button>
          </div>
          <p className="text-[11px] text-muted-foreground">
            在阿里云控制台 → 智能语音交互 → 项目管理 获取 AppKey；在账号中心获取 AccessKey
          </p>
        </div>

        <div className="flex items-center gap-3 pt-1">
          <Button
            variant="outline"
            size="sm"
            disabled={asrTesting}
            onClick={async () => {
              setAsrTesting(true);
              setAsrTestResult(null);
              try {
                const result = await testAsrConnection(local);
                setAsrTestResult(result);
              } catch (e) {
                setAsrTestResult({ success: false, message: String(e) });
              } finally {
                setAsrTesting(false);
              }
            }}
          >
            {asrTesting ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : null}
            测试配置
          </Button>
          {asrTestResult && (
            <span className={`flex items-center gap-1 text-xs ${asrTestResult.success ? "text-green-600" : "text-destructive"}`}>
              {asrTestResult.success ? (
                <CheckCircle2 className="h-3.5 w-3.5" />
              ) : (
                <XCircle className="h-3.5 w-3.5" />
              )}
              {asrTestResult.message}
            </span>
          )}
        </div>
      </>
    )}
  </CardContent>
</Card>
```

---

**Step 8: 验证编译 + 类型检查**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(^error|Finished)"
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -30
```

Expected: 均无 error

---

**Step 9: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/commands.rs src-tauri/src/lib.rs
git -C "D:/project/java/source/memo-ai" add src/types/index.ts src/store/settingsStore.ts
git -C "D:/project/java/source/memo-ai" add src/hooks/useTauriCommands.ts src/pages/Settings.tsx
git -C "D:/project/java/source/memo-ai" commit -m "feat: ASR provider selector, Whisper wizard, Aliyun config panel in Settings"
```

---

## 完成标准

- [ ] `cargo check` 和 `tsc --noEmit` 均无 error
- [ ] Settings 页 ASR Card 顶部显示 Provider 选择器，切换后面板内容变化
- [ ] 本地 Whisper 面板：点击「检测」按钮，已安装时显示版本，未安装时显示下载链接
- [ ] 阿里云 ASR 面板：配置 AppKey/AccessKey 后点击「测试配置」，鉴权失败时显示具体错误
- [ ] LLM 配置区：点击「测试连接」，Ollama 未启动时显示友好错误；OpenAI Key 错误时提示「API Key 无效」
- [ ] 切换 LLM Provider 时测试状态重置为 idle
- [ ] 旧 settings.json 启动后不报错（serde default 兼容）
- [ ] `transcribe_audio` 命令根据 `asr_provider` 字段自动路由到对应实现
