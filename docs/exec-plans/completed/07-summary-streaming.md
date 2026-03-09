# Summary Streaming Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为重新生成按钮添加流式展示能力：stage1/2 显示动态状态文字，stage4 token 逐字追加到内容框。

**Architecture:** Rust 侧扩展 LlmClient trait 增加 complete_streaming 方法，新增 regenerate_summary_stream Tauri 命令通过事件推流；前端 SummaryTab 监听事件更新内容框状态机。

**Tech Stack:** Rust/reqwest::blocking（BufReader 逐行读取流），Tauri emit/listen，React useState，@tauri-apps/api/event

---

### Task 1: 扩展 LlmClient trait 增加 complete_streaming 方法

**Files:**
- Modify: `src-tauri/src/llm/client.rs`

**Step 1: 在 trait 中添加 complete_streaming**

读取 `src-tauri/src/llm/client.rs`，在 `complete` 方法之后添加：

```rust
/// Send a prompt and stream the response token by token.
/// Returns the full accumulated response when done.
fn complete_streaming(
    &self,
    prompt: &str,
    on_token: Box<dyn Fn(&str) + Send>,
) -> AppResult<String>;
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

预期：会报错说 `OllamaClient` 和 `OpenAiClient` 未实现新方法（正常，Tasks 2/3 修复）

**Step 3: Commit**

```bash
git add src-tauri/src/llm/client.rs
git commit -m "feat(llm): add complete_streaming to LlmClient trait"
```

---

### Task 2: OllamaClient 实现 complete_streaming

**Files:**
- Modify: `src-tauri/src/llm/ollama.rs`

**Step 1: 添加流式响应结构体和实现**

在 `src-tauri/src/llm/ollama.rs` 中：

1. 在现有 `OllamaResponse` 结构体之后添加：

```rust
#[derive(Deserialize)]
struct OllamaStreamChunk {
    response: String,
    done: bool,
}
```

2. 在现有 `OllamaRequest` 结构体中 `stream` 字段已存在（保持不变）。

3. 在 `impl LlmClient for OllamaClient` 块中，在 `provider_name` 之前添加：

```rust
fn complete_streaming(
    &self,
    prompt: &str,
    on_token: Box<dyn Fn(&str) + Send>,
) -> AppResult<String> {
    use std::io::BufRead;

    let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
    let req = OllamaRequest {
        model: &self.model,
        prompt,
        stream: true,
    };
    let resp = self
        .http
        .post(&url)
        .json(&req)
        .send()
        .map_err(|e| AppError::Llm(format!("Ollama streaming request failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::Llm(format!(
            "Ollama returned status {}",
            resp.status()
        )));
    }

    let mut full_text = String::new();
    let reader = std::io::BufReader::new(resp);
    for line in reader.lines() {
        let line = line.map_err(|e| AppError::Llm(format!("Stream read error: {}", e)))?;
        if line.is_empty() {
            continue;
        }
        let chunk: OllamaStreamChunk = serde_json::from_str(&line)
            .map_err(|e| AppError::Llm(format!("Failed to parse stream chunk: {}", e)))?;
        if !chunk.response.is_empty() {
            on_token(&chunk.response);
            full_text.push_str(&chunk.response);
        }
        if chunk.done {
            break;
        }
    }
    Ok(full_text)
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

预期：Ollama 相关错误消失，只剩 OpenAiClient 未实现

**Step 3: Commit**

```bash
git add src-tauri/src/llm/ollama.rs
git commit -m "feat(llm): implement complete_streaming for OllamaClient"
```

---

### Task 3: OpenAiClient 实现 complete_streaming

**Files:**
- Modify: `src-tauri/src/llm/openai.rs`

**Step 1: 添加流式响应结构体和实现**

在 `src-tauri/src/llm/openai.rs` 中：

1. 在现有结构体之后添加：

```rust
#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Serialize)]
struct ChatStreamRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
}
```

2. 在 `impl LlmClient for OpenAiClient` 块中，在 `provider_name` 之前添加：

```rust
fn complete_streaming(
    &self,
    prompt: &str,
    on_token: Box<dyn Fn(&str) + Send>,
) -> AppResult<String> {
    use std::io::BufRead;

    let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
    let req = ChatStreamRequest {
        model: &self.model,
        messages: vec![ChatMessage { role: "user", content: prompt }],
        stream: true,
    };
    let resp = self
        .http
        .post(&url)
        .bearer_auth(&self.api_key)
        .json(&req)
        .send()
        .map_err(|e| AppError::Llm(format!("OpenAI streaming request failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(AppError::Llm(format!(
            "OpenAI returned status {}: {}",
            status, body
        )));
    }

    let mut full_text = String::new();
    let reader = std::io::BufReader::new(resp);
    for line in reader.lines() {
        let line = line.map_err(|e| AppError::Llm(format!("Stream read error: {}", e)))?;
        if line.is_empty() || line == "data: [DONE]" {
            continue;
        }
        let data = line.strip_prefix("data: ").unwrap_or(&line);
        let chunk: StreamChunk = match serde_json::from_str(data) {
            Ok(c) => c,
            Err(_) => continue, // 跳过非 JSON 行（如注释行）
        };
        if let Some(choice) = chunk.choices.into_iter().next() {
            if let Some(content) = choice.delta.content {
                if !content.is_empty() {
                    on_token(&content);
                    full_text.push_str(&content);
                }
            }
        }
    }
    Ok(full_text)
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

预期：无错误

**Step 3: Commit**

```bash
git add src-tauri/src/llm/openai.rs
git commit -m "feat(llm): implement complete_streaming for OpenAiClient"
```

---

### Task 4: Pipeline 新增 stage4_summary_streaming

**Files:**
- Modify: `src-tauri/src/llm/pipeline.rs`

**Step 1: 在 stage4_summary 之后添加流式版本**

在 `src-tauri/src/llm/pipeline.rs` 中，`stage4_summary` 方法（约第 111-116 行）之后添加：

```rust
/// Stage 4 (streaming): Generate meeting summary, calling on_token for each token.
/// Returns the full summary when done.
pub fn stage4_summary_streaming(
    &self,
    meeting_text: &str,
    on_token: Box<dyn Fn(&str) + Send>,
) -> AppResult<String> {
    let template = self.load_prompt("04_summary.txt")?;
    let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
    log::info!("Running pipeline stage 4 (streaming): summary");
    self.client.complete_streaming(&prompt, on_token)
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

预期：无错误

**Step 3: Commit**

```bash
git add src-tauri/src/llm/pipeline.rs
git commit -m "feat(pipeline): add stage4_summary_streaming method"
```

---

### Task 5: 新增 regenerate_summary_stream Tauri 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: 添加事件结构体**

在 `src-tauri/src/commands.rs` 中，`PipelineStageDoneEvent` 结构体（约第 247 行）之后添加：

```rust
#[derive(Clone, Serialize)]
pub struct SummaryStageEvent {
    pub stage: u8,
    pub name: String,
}

#[derive(Clone, Serialize)]
pub struct SummaryChunkEvent {
    pub text: String,
}

#[derive(Clone, Serialize)]
pub struct SummaryDoneEvent {
    pub summary: String,
}

#[derive(Clone, Serialize)]
pub struct SummaryErrorEvent {
    pub message: String,
}
```

**Step 2: 在 regenerate_summary 命令（约第 207 行）之后添加新命令**

```rust
#[tauri::command]
pub async fn regenerate_summary_stream(
    meeting_id: i64,
    app_handle: tauri::AppHandle,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<(), String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let llm_config = LlmConfig {
        provider: cfg.llm_provider.provider_type,
        base_url: cfg.llm_provider.base_url,
        model: cfg.llm_provider.model,
        api_key: cfg.llm_provider.api_key,
    };

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let prompts_dir = {
        let exe_adjacent = exe_dir.join("prompts");
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("prompts");
        if exe_adjacent.exists() {
            exe_adjacent
        } else if dev_path.exists() {
            dev_path
        } else {
            PathBuf::from("prompts")
        }
    };

    let transcript_text = {
        let conn = (*db).0.lock().unwrap();
        let segments = models::get_transcripts(&conn, meeting_id).map_err(|e| e.to_string())?;
        segments
            .iter()
            .map(|s| {
                if let Some(ref speaker) = s.speaker {
                    format!("{}：{}", speaker, s.text)
                } else {
                    s.text.clone()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    if transcript_text.is_empty() {
        let _ = app_handle.emit("summary_error", SummaryErrorEvent {
            message: "No transcript available".into(),
        });
        return Ok(());
    }

    let app_for_cb = app_handle.clone();
    let (tx, rx) = tokio::sync::oneshot::channel::<Result<String, String>>();

    std::thread::spawn(move || {
        let client = llm_config.build_client();
        let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);

        // Stage 1
        let _ = app_for_cb.emit("summary_stage", SummaryStageEvent {
            stage: 1,
            name: "正在清洗文本...".into(),
        });
        let clean = match pipeline.stage1_clean(&transcript_text) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_for_cb.emit("summary_error", SummaryErrorEvent { message: e.to_string() });
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };

        // Stage 2
        let _ = app_for_cb.emit("summary_stage", SummaryStageEvent {
            stage: 2,
            name: "正在整理说话人...".into(),
        });
        let organized = match pipeline.stage2_speaker(&clean) {
            Ok(o) => o,
            Err(e) => {
                let _ = app_for_cb.emit("summary_error", SummaryErrorEvent { message: e.to_string() });
                let _ = tx.send(Err(e.to_string()));
                return;
            }
        };

        // Stage 4 (streaming)
        let app_for_token = app_for_cb.clone();
        let on_token: Box<dyn Fn(&str) + Send> = Box::new(move |token: &str| {
            let _ = app_for_token.emit("summary_chunk", SummaryChunkEvent {
                text: token.to_string(),
            });
        });

        let result = pipeline.stage4_summary_streaming(&organized, on_token);
        match result {
            Ok(summary) => {
                let _ = app_for_cb.emit("summary_done", SummaryDoneEvent {
                    summary: summary.clone(),
                });
                let _ = tx.send(Ok(summary));
            }
            Err(e) => {
                let _ = app_for_cb.emit("summary_error", SummaryErrorEvent { message: e.to_string() });
                let _ = tx.send(Err(e.to_string()));
            }
        }
    });

    // 等待线程完成，将结果写库
    match rx.await {
        Ok(Ok(summary)) => {
            let conn = (*db).0.lock().unwrap();
            models::update_meeting_summary(&conn, meeting_id, &summary)
                .map_err(|e| e.to_string())?;
        }
        Ok(Err(e)) => return Err(e),
        Err(_) => return Err("Stream thread panicked".into()),
    }

    Ok(())
}
```

**Step 3: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | grep "error" | head -20
```

预期：无错误

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): add regenerate_summary_stream command with event emission"
```

---

### Task 6: 注册新命令到 lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: 在 use commands::{...} 中添加**

在现有导入块末尾（`update_meeting_summary, regenerate_summary,` 之后）添加 `regenerate_summary_stream`：

```rust
update_meeting_summary, regenerate_summary, regenerate_summary_stream,
```

**Step 2: 在 generate_handler! 中添加**

在 `regenerate_summary,` 之后添加：

```rust
            regenerate_summary_stream,
```

**Step 3: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

预期：`Finished dev profile` 无错误

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(lib): register regenerate_summary_stream command"
```

---

### Task 7: 前端添加 hook 和 i18n 键

**Files:**
- Modify: `src/hooks/useTauriCommands.ts`
- Modify: `src/i18n/locales/zh.ts`
- Modify: `src/i18n/locales/en.ts`

**Step 1: 在 useTauriCommands.ts 末尾添加 hook**

```typescript
export function useRegenerateSummaryStream() {
  return useCallback(
    (meetingId: number) =>
      invoke<void>("regenerate_summary_stream", { meetingId }),
    []
  );
}
```

**Step 2: 在 zh.ts 的 summary.actions 中添加阶段文案**

找到 `summary.actions` 对象（约第 26-32 行），在其中添加：

```typescript
stage1: "正在清洗文本...",
stage2: "正在整理说话人...",
```

最终 summary.actions 应包含：edit, finishEdit, copy, regenerate, stage1, stage2

**Step 3: 在 en.ts 的 summary.actions 中添加**

```typescript
stage1: "Cleaning transcript...",
stage2: "Organizing speakers...",
```

**Step 4: 类型检查**

```bash
npx tsc --noEmit 2>&1 | head -20
```

**Step 5: Commit**

```bash
git add src/hooks/useTauriCommands.ts src/i18n/locales/zh.ts src/i18n/locales/en.ts
git commit -m "feat(frontend): add regenerate stream hook and i18n stage keys"
```

---

### Task 8: 更新 SummaryTab 组件支持流式 UI

**Files:**
- Modify: `src/components/SummaryTab.tsx`

**Step 1: 添加必要 import**

在文件顶部添加：

```typescript
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRegenerateSummaryStream } from "@/hooks/useTauriCommands";
```

移除原有的 `useRegenerateSummary` import（不再直接使用）。

**Step 2: 替换组件内状态和逻辑**

将原有的：
```typescript
const [isRegenerating, setIsRegenerating] = useState(false);
```
替换为：
```typescript
type RegeneratePhase = "idle" | "stage1" | "stage2" | "streaming" | "done";
const [regenPhase, setRegenPhase] = useState<RegeneratePhase>("idle");
const [streamingText, setStreamingText] = useState("");
const unlistenRef = useRef<UnlistenFn[]>([]);
```

将原有的 `regenerateSummary` 调用改为：
```typescript
const regenerateSummaryStream = useRegenerateSummaryStream();
```

**Step 3: 替换事件监听卸载逻辑**

将原有的 cleanup useEffect（仅清 debounce timer）更新为同时 unlisten：

```typescript
useEffect(() => {
  return () => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }
    unlistenRef.current.forEach((fn) => fn());
    unlistenRef.current = [];
  };
}, []);
```

**Step 4: 替换 handleRegenerate 函数**

将原有的 `handleRegenerate` 完整替换为：

```typescript
async function handleRegenerate() {
  // 清空上次监听
  unlistenRef.current.forEach((fn) => fn());
  unlistenRef.current = [];
  setStreamingText("");
  setRegenPhase("stage1");

  const unlistenStage = await listen<{ stage: number; name: string }>(
    "summary_stage",
    (event) => {
      if (event.payload.stage === 1) setRegenPhase("stage1");
      if (event.payload.stage === 2) setRegenPhase("stage2");
    }
  );

  const unlistenChunk = await listen<{ text: string }>(
    "summary_chunk",
    (event) => {
      setRegenPhase("streaming");
      setStreamingText((prev) => prev + event.payload.text);
    }
  );

  const unlistenDone = await listen<{ summary: string }>(
    "summary_done",
    (event) => {
      setRegenPhase("done");
      setStreamingText(event.payload.summary);
      onSummaryUpdated(event.payload.summary);
      unlistenRef.current.forEach((fn) => fn());
      unlistenRef.current = [];
    }
  );

  const unlistenError = await listen<{ message: string }>(
    "summary_error",
    (event) => {
      console.error("Regenerate summary failed:", event.payload.message);
      setRegenPhase("idle");
      setStreamingText("");
      unlistenRef.current.forEach((fn) => fn());
      unlistenRef.current = [];
    }
  );

  unlistenRef.current = [unlistenStage, unlistenChunk, unlistenDone, unlistenError];

  try {
    await regenerateSummaryStream(meeting.id);
  } catch (e) {
    console.error("Failed to invoke regenerate_summary_stream:", e);
    setRegenPhase("idle");
    unlistenRef.current.forEach((fn) => fn());
    unlistenRef.current = [];
  }
}
```

**Step 5: 更新 disabled 条件**

将所有 `isRegenerating` 替换为 `regenPhase !== "idle"`：

```tsx
disabled={isProcessing || regenPhase !== "idle" || !hasSummary}  // 编辑、复制按钮
disabled={isProcessing || regenPhase !== "idle" || isEditing}    // 重新生成按钮
```

重新生成按钮的图标逻辑：
```tsx
{regenPhase !== "idle"
  ? <Loader2 className="h-4 w-4 animate-spin" />
  : <RefreshCw className="h-4 w-4" />
}
```

**Step 6: 更新内容框渲染逻辑**

将内容区域（`{/* 内容区域 */}` 注释之后的 JSX）替换为：

```tsx
{/* 内容区域 */}
{regenPhase === "stage1" || regenPhase === "stage2" ? (
  <div className="flex items-center gap-2 px-4 py-12 text-sm text-muted-foreground">
    <Loader2 className="h-4 w-4 animate-spin shrink-0" />
    <span>
      {regenPhase === "stage1"
        ? t("summary.actions.stage1")
        : t("summary.actions.stage2")}
    </span>
  </div>
) : regenPhase === "streaming" ? (
  <div className="px-4 py-2 text-sm font-mono leading-relaxed text-foreground whitespace-pre-wrap">
    {streamingText}
    <span className="inline-block w-0.5 h-4 bg-foreground ml-0.5 animate-pulse" />
  </div>
) : isEditing ? (
  <textarea
    className="w-full min-h-[200px] resize-y rounded-md border border-border bg-background px-3 py-2 text-sm font-mono leading-relaxed text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
    value={editText}
    onChange={handleTextChange}
    autoFocus
  />
) : (regenPhase === "done" ? streamingText : meeting.summary) ? (
  <div className="p-4 text-sm leading-relaxed text-foreground prose prose-sm max-w-none dark:prose-invert">
    <ReactMarkdown remarkPlugins={[remarkGfm]}>
      {regenPhase === "done" ? streamingText : meeting.summary!}
    </ReactMarkdown>
  </div>
) : (
  <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
    {t("meeting.noSummary")}
  </div>
)}
```

**Step 7: 类型检查**

```bash
npx tsc --noEmit 2>&1 | head -30
```

预期：无错误

**Step 8: Commit**

```bash
git add src/components/SummaryTab.tsx
git commit -m "feat(SummaryTab): add streaming UI with stage status and token-by-token display"
```

---

### Task 9: 最终验证

**Step 1: Rust 编译检查**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

预期：`Finished dev profile` 无错误

**Step 2: TypeScript 类型检查**

```bash
npx tsc --noEmit 2>&1
```

预期：无输出（无错误）

**Step 3: 验证所有提交**

```bash
git log --oneline -12
```

预期：包含本次所有 feat commit

**Step 4: 手动验证清单**

运行 `npm run tauri:dev` 后：

- [ ] 有转写内容的会议，点击重新生成按钮
- [ ] 内容框出现旋转图标 + "正在清洗文本..."
- [ ] 片刻后变为 "正在整理说话人..."
- [ ] 随后内容框开始逐字出现文字，末尾有光标闪烁
- [ ] 生成完成后切换为 Markdown 渲染
- [ ] 刷新页面后新总结已持久化
- [ ] 生成过程中三个按钮全部 disabled
- [ ] 无转写内容时重新生成不崩溃（console.error）
