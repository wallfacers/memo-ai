# 总结选项卡流式重新生成设计文档

**日期：** 2026-03-07
**状态：** 已确认，待实现

## 需求概述

点击"重新生成"按钮时，支持流式展示 LLM 生成过程：
- stage1/2（清洗、说话人整理）：内容框内显示动态阶段状态文字
- stage4（总结生成）：token 逐字追加到内容框，实时可见

## 方案

采用 **Tauri 事件推流 + 新增流式命令**，与现有 FunASR 流式事件模式一致。

## 事件协议

| 事件名 | Payload | 说明 |
|--------|---------|------|
| `summary_stage` | `{ stage: number, name: string }` | stage1/2 阶段变化 |
| `summary_chunk` | `{ text: string }` | stage4 每个 token |
| `summary_done` | `{ summary: string }` | 完成，携带完整内容 |
| `summary_error` | `{ message: string }` | 出错 |

## 数据流

```
前端点击"重新生成"
  → invoke("regenerate_summary_stream", { meetingId })
  → Rust OS线程
      stage1_clean()   → emit("summary_stage", { stage:1, name:"正在清洗文本..." })
      stage2_speaker() → emit("summary_stage", { stage:2, name:"正在整理说话人..." })
      stage4_streaming → emit("summary_chunk", { text: token }) × N
                       → emit("summary_done", { summary: fullText })
  → 前端 listen 事件 → 更新内容框
```

## Rust 侧改动

### 1. LlmClient trait 新增方法

```rust
fn complete_streaming(
    &self,
    prompt: &str,
    on_token: Box<dyn Fn(&str) + Send>,
) -> AppResult<String>;
```

### 2. OllamaClient 流式实现

- 请求 `stream: true`
- 用 `BufReader::new(resp).lines()` 逐行读取 NDJSON
- 每行解析 `{"response": "token", "done": false}`，提取 `response` 调用 `on_token`
- `done: true` 时结束循环

### 3. OpenAiClient 流式实现

- 请求 body 加 `"stream": true`
- 逐行读取 SSE，跳过 `data: [DONE]`
- 解析 `data: {...}` 行的 `choices[0].delta.content`，调用 `on_token`

### 4. Pipeline 新增 stage4_summary_streaming

```rust
pub fn stage4_summary_streaming(
    &self,
    meeting_text: &str,
    on_token: Box<dyn Fn(&str) + Send>,
) -> AppResult<String>
```

调用 `client.complete_streaming()`，回调转发给 `on_token`。

### 5. 新 Tauri 命令 regenerate_summary_stream

- 替代原 `regenerate_summary`（原命令保留，供非流式场景兼容）
- 在 OS 线程中：
  1. `stage1_clean` → emit `summary_stage`
  2. `stage2_speaker` → emit `summary_stage`
  3. `stage4_summary_streaming`（on_token → emit `summary_chunk`）
  4. 写库（`update_meeting_summary`）
  5. emit `summary_done`
- 出错时 emit `summary_error`
- 返回 `Result<(), String>`

### 6. lib.rs 注册新命令

## 前端改动

### SummaryTab 新增状态

```typescript
type RegeneratePhase = "idle" | "stage1" | "stage2" | "streaming" | "done";
const [regenPhase, setRegenPhase] = useState<RegeneratePhase>("idle");
const [streamingText, setStreamingText] = useState("");
```

### 内容框渲染逻辑

| 状态 | 显示 |
|------|------|
| `idle` | 正常 Markdown 渲染 / textarea 编辑 |
| `stage1` | Loader2 旋转 + "正在清洗文本..." |
| `stage2` | Loader2 旋转 + "正在整理说话人..." |
| `streaming` | 逐字追加纯文本 + 末尾光标闪烁（`animate-pulse`） |
| `done` | 切回 Markdown 渲染 streamingText |

### 事件监听

- 点击重新生成时用 `listen()` 订阅四个事件
- `summary_done` / `summary_error` 后调用 `unlisten()`
- 组件 `useEffect` cleanup 强制 `unlisten()`

### 按钮 disabled 条件

`regenPhase !== "idle"` 时三个按钮全部 disabled。

## 边界情况

| 情况 | 处理 |
|------|------|
| 流式中途报错 | emit `summary_error`，前端恢复原内容，`console.error` |
| 用户切换选项卡/关闭 | cleanup unlisten，Rust 线程继续跑完，done 时仍写库 |
| stage1/2 期间内容框 | 保持原有总结，第一个 chunk 到达时才清空 |
| 无转写文本 | 与现有 regenerate_summary 一致，emit error |

## 涉及文件

| 文件 | 变更类型 |
|------|----------|
| `src-tauri/src/llm/client.rs` | 修改（trait 新增方法） |
| `src-tauri/src/llm/ollama.rs` | 修改（实现流式） |
| `src-tauri/src/llm/openai.rs` | 修改（实现流式） |
| `src-tauri/src/llm/pipeline.rs` | 修改（新增 stage4_streaming） |
| `src-tauri/src/commands.rs` | 修改（新增命令） |
| `src-tauri/src/lib.rs` | 修改（注册命令） |
| `src/components/SummaryTab.tsx` | 修改（流式 UI 状态） |
| `src/hooks/useTauriCommands.ts` | 修改（新增 hook） |
| `src/i18n/locales/zh.ts` 和 `en.ts` | 修改（新增阶段文案） |
