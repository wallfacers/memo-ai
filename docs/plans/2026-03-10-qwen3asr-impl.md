# Qwen3-ASR Integration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Integrate Qwen3-ASR as a new ASR provider in memo-ai, sending WAV chunks via multipart HTTP to `/v1/audio/transcriptions` and assembling results with approximate timestamps.

**Architecture:** A new `Qwen3AsrProvider` implements the existing `AsrProvider` trait. It reads a WAV file with `hound`, splits it into 30-second chunks, sends each chunk to the Qwen3-ASR Docker API via `reqwest::blocking::multipart`, parses `language XX<asr_text>text` responses, and constructs `TranscriptSegment` with offset-based timestamps. Frontend adds the new provider option and a URL config field.

**Tech Stack:** Rust (`hound`, `reqwest::blocking::multipart`, `serde_json`), React + TypeScript (Settings page, i18n)

---

### Task 1: Add Qwen3AsrProvider (Rust backend core)

**Files:**
- Create: `src-tauri/src/asr/qwen3asr.rs`

**Step 1: Create the file with full implementation**

```rust
use std::io::Cursor;
use std::path::Path;
use hound::{WavReader, WavSpec, WavWriter, SampleFormat};
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

    /// WAV ファイルを chunk_secs 秒ごとに分割し、各チャンクのバイト列を返す
    fn split_wav(audio_path: &Path, chunk_secs: u32) -> AppResult<(Vec<Vec<u8>>, f64)> {
        let mut reader = WavReader::open(audio_path)
            .map_err(|e| AppError::Asr(format!("Qwen3-ASR: failed to open WAV: {}", e)))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        let channels = spec.channels as u32;
        let chunk_samples = (chunk_secs * sample_rate * channels) as usize;

        // 全サンプルをi32として読み込む（8/16/24/32bit対応）
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

    /// 単一チャンクをAPIに送信してテキストを返す
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

        // "language XX<asr_text>実際のテキスト" → テキスト部分を抽出
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
```

**Step 2: Verify the file compiles (partial check)**

```bash
cd src-tauri && cargo check 2>&1 | grep "qwen3asr\|error" | head -20
```

Expected: errors about `qwen3asr` module not registered (we'll fix in Task 2). No type/syntax errors within the file itself.

**Step 3: Commit**

```bash
git add src-tauri/src/asr/qwen3asr.rs
git commit -m "feat(asr): add Qwen3AsrProvider with WAV chunking"
```

---

### Task 2: Register module and wire provider

**Files:**
- Modify: `src-tauri/src/asr/mod.rs`
- Modify: `src-tauri/src/asr/provider.rs`

**Step 1: Add module to mod.rs**

In `src-tauri/src/asr/mod.rs`, add after line 5 (`pub mod streaming;`):

```rust
pub mod qwen3asr;
```

**Step 2: Wire in provider.rs**

In `src-tauri/src/asr/provider.rs`, add a new match arm before the `_` fallback (after the `"aliyun"` arm, around line 22):

```rust
"qwen3_asr" => Box::new(super::qwen3asr::Qwen3AsrProvider::new(
    &config.qwen3_asr_url,
)),
```

**Step 3: Cargo check**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

Expected: error about `qwen3_asr_url` field not existing on `AppConfig` (fix in Task 3).

**Step 4: Commit**

```bash
git add src-tauri/src/asr/mod.rs src-tauri/src/asr/provider.rs
git commit -m "feat(asr): register Qwen3AsrProvider in module tree"
```

---

### Task 3: Extend AppConfig with qwen3_asr_url

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: Add field to AppConfig struct**

In `src-tauri/src/commands.rs`, after the `funasr_enabled` field (line 51), add:

```rust
#[serde(default = "default_qwen3_asr_url")]
pub qwen3_asr_url: String,
```

**Step 2: Add default function**

After `fn default_funasr_port()` (line 58), add:

```rust
fn default_qwen3_asr_url() -> String {
    "http://localhost:8000".into()
}
```

**Step 3: Add to Default impl**

In the `Default for AppConfig` impl block, after `funasr_enabled: false,` (line 89), add:

```rust
qwen3_asr_url: "http://localhost:8000".into(),
```

**Step 4: Add qwen3_asr to test_asr_connection**

In `test_asr_connection` (around line 1057), add a new match arm before `_`:

```rust
"qwen3_asr" => {
    let url = format!("{}/health", settings.qwen3_asr_url.trim_end_matches('/'));
    match reqwest::blocking::get(&url) {
        Ok(r) if r.status().is_success() => Ok(AsrTestResult {
            success: true,
            message: format!("Qwen3-ASR 连接成功（{}）", settings.qwen3_asr_url),
        }),
        Ok(r) => Ok(AsrTestResult {
            success: false,
            message: format!("Qwen3-ASR 返回 HTTP {}", r.status()),
        }),
        Err(e) => Ok(AsrTestResult {
            success: false,
            message: format!("Qwen3-ASR 连接失败：{}", e),
        }),
    }
}
```

**Step 5: Cargo check — expect clean**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

Expected: no errors.

**Step 6: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(config): add qwen3_asr_url field to AppConfig"
```

---

### Task 4: Frontend — types and i18n

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/i18n/locales/zh.ts`
- Modify: `src/i18n/locales/en.ts`

**Step 1: Update AsrProviderType in types/index.ts**

Find line 55:
```typescript
export type AsrProviderType = "local_whisper" | "aliyun" | "funasr";
```

Replace with:
```typescript
export type AsrProviderType = "local_whisper" | "aliyun" | "funasr" | "qwen3_asr";
```

Also add `qwen3_asr_url` to `AppSettings` interface (after `funasr_port`):
```typescript
qwen3_asr_url: string;
```

**Step 2: Add zh.ts i18n entries**

In `src/i18n/locales/zh.ts`, after the `funasrProvider` line (line 108), add:

```typescript
qwen3AsrProvider: "Qwen3-ASR（本地）",
qwen3AsrUrl: "API 地址",
qwen3AsrUrlPlaceholder: "http://localhost:8000",
qwen3AsrUrlHint: "需先启动 Docker 容器：docker start qwen3-asr",
```

**Step 3: Add en.ts i18n entries**

In `src/i18n/locales/en.ts`, after the `funasrProvider` line (line 108), add:

```typescript
qwen3AsrProvider: "Qwen3-ASR (Local)",
qwen3AsrUrl: "API URL",
qwen3AsrUrlPlaceholder: "http://localhost:8000",
qwen3AsrUrlHint: "Start Docker container first: docker start qwen3-asr",
```

**Step 4: TypeScript check**

```bash
npx tsc --noEmit 2>&1 | grep "error" | head -20
```

Expected: errors about `qwen3_asr_url` missing in Settings.tsx (fix in Task 5).

**Step 5: Commit**

```bash
git add src/types/index.ts src/i18n/locales/zh.ts src/i18n/locales/en.ts
git commit -m "feat(types): add qwen3_asr provider type and i18n strings"
```

---

### Task 5: Frontend — Settings panel

**Files:**
- Modify: `src/pages/Settings.tsx`

**Step 1: Add Qwen3-ASR option to SelectContent**

Find the block with `<SelectItem value="aliyun">` (around line 305). After that line, add:

```tsx
<SelectItem value="qwen3_asr">{t("settings.asr.qwen3AsrProvider")}</SelectItem>
```

**Step 2: Add Qwen3-ASR config panel**

After the Aliyun ASR panel closing `</>` (around line 504), add:

```tsx
{/* Qwen3-ASR 面板 */}
{local.asr_provider === "qwen3_asr" && (
  <>
    <div className="space-y-1.5">
      <label className="text-sm font-medium text-foreground">
        {t("settings.asr.qwen3AsrUrl")}
      </label>
      <Input
        value={local.qwen3_asr_url}
        onChange={(e) => setLocal({ ...local, qwen3_asr_url: e.target.value })}
        placeholder={t("settings.asr.qwen3AsrUrlPlaceholder")}
      />
      <p className="text-[11px] text-muted-foreground">
        {t("settings.asr.qwen3AsrUrlHint")}
      </p>
    </div>
  </>
)}
```

**Step 3: Ensure qwen3_asr_url is included in settingsStore default**

Check `src/store/settingsStore.ts` — find the default settings object and add:
```typescript
qwen3_asr_url: "http://localhost:8000",
```

**Step 4: TypeScript check — expect clean**

```bash
npx tsc --noEmit 2>&1 | grep "error" | head -20
```

Expected: no errors.

**Step 5: Commit**

```bash
git add src/pages/Settings.tsx src/store/settingsStore.ts
git commit -m "feat(settings): add Qwen3-ASR provider option and config panel"
```

---

### Task 6: End-to-end build verification

**Step 1: Full Rust build**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^error" | head -20
```

Expected: `Finished` with no errors.

**Step 2: Frontend build**

```bash
npm run build 2>&1 | grep -E "error|Error" | head -20
```

Expected: no errors.

**Step 3: Manual smoke test**

1. Start Qwen3-ASR Docker: `docker start qwen3-asr`, then inside container run `qwen-asr-serve /data/shared/Qwen3-ASR --port 80 --host 0.0.0.0 --gpu-memory-utilization 0.85 --max-model-len 45392`
2. Run app: `npm run tauri:dev`
3. Open Settings → ASR 引擎 → 选择 "Qwen3-ASR（本地）"
4. 点击"测试配置" → 应显示连接成功
5. 录制一段短会议 → 停止 → 等待转写完成 → 验证 Transcript 内容非空

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete Qwen3-ASR integration with WAV chunking"
```
