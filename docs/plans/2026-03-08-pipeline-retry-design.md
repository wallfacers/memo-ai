# Pipeline 阶段性重试设计

**日期**：2026-03-08
**状态**：待实现

---

## 背景

当 `run_pipeline` 在某个 LLM 阶段出错时，当前实现只将 `recordingPhase` 设为 `error`，用户无任何重试手段，只能重新录音。

目标：在 PipelineProgress 进度条的失败阶段旁显示重试按钮，支持从失败阶段续跑，跳过前面已成功的阶段。

---

## 方案选择

采用**方案二：存储中间结果 + 断点续跑**。

每个阶段完成后立即写 DB，新增 `retry_pipeline_from_stage` 命令从指定阶段恢复执行，前端通过 `pipeline_stage_failed` 事件获知失败阶段和错误原因。

---

## 数据层

### DB Schema 变更

`meetings` 表新增两个字段：

| 字段 | 类型 | 说明 |
|------|------|------|
| `clean_transcript` | TEXT | Stage1 输出（清洗后文本） |
| `organized_transcript` | TEXT | Stage2 输出（整理后带说话人文本） |

### 各阶段所需输入来源

| 续跑起始阶段 | 从 DB 读取的字段 |
|-------------|----------------|
| Stage 1 | `transcripts`（原始转写，已有） |
| Stage 2 | `clean_transcript` |
| Stage 3 / 4 / 5 | `organized_transcript` |
| Stage 6 | `summary` + `action_items`（已有） |

---

## 后端

### 新增事件：`pipeline_stage_failed`

```rust
#[derive(Serialize, Clone)]
pub struct PipelineStageFailed {
    pub stage: u32,
    pub error: String,
}
```

任意阶段出错时 emit 此事件，前端据此展示失败阶段和错误信息。

### `run_pipeline` 改造

**逐阶段写 DB**（而非全部完成后一次性写）：

```
Stage1 完成 → 写 clean_transcript       → emit pipeline_stage_done
Stage2 完成 → 写 organized_transcript   → emit pipeline_stage_done
Stage3 完成 → 写 structure              → emit pipeline_stage_done
Stage4 完成 → 写 summary               → emit pipeline_stage_done
Stage5 完成 → 写 action_items          → emit pipeline_stage_done
Stage6 完成 → 写 report                → emit pipeline_stage_done → status=completed
```

任意阶段失败：
```
emit pipeline_stage_failed { stage, error }
return Err(...)
```

### 新增命令：`retry_pipeline_from_stage`

```rust
#[tauri::command]
pub async fn retry_pipeline_from_stage(
    meeting_id: i64,
    from_stage: u32,
    app_handle: tauri::AppHandle,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<PipelineResult, String>
```

执行逻辑：

1. 从 DB 读取 `from_stage` 所需的输入字段
2. 清除 DB 中 `from_stage` 及之后阶段的已有结果（防止重复累积行动项等）
3. 从 `from_stage` 开始执行，逐阶段写 DB + emit `pipeline_stage_done`

**降级规则**：若所需中间数据在 DB 中缺失（如 `organized_transcript` 为空），自动从 Stage1 重新执行。

---

## 前端

### Store 变更（meetingStore）

新增字段：

```typescript
pipelineFailedStage: { stage: number; error: string } | null
setPipelineFailedStage: (info: { stage: number; error: string } | null) => void
```

在监听 `pipeline_stage_failed` 事件时设置此字段。

### PipelineProgress 组件改造

**显示时机**：在 `recordingPhase === "error"` 时也保持可见（当前只在 `"pipeline"` 和 `"done"` 时显示）。

**阶段行状态图标**：

| 状态 | 图标 | 样式 |
|------|------|------|
| 已完成 | CheckCircle2 | text-green-600 |
| 进行中 | Loader2 (animate-spin) | text-muted-foreground |
| 失败 | XCircle | text-destructive |
| 等待 | 空心圆 | border-muted-foreground/30 |

**失败行 UI**：

```
❌ 会议总结                              [从此阶段重试 ↺]
   └─ 模型 'qwen2.5' 响应超时，请检查 Ollama 是否运行
○ 行动项提取
○ 报告生成
```

- 错误信息：`text-xs text-destructive/70`，缩进显示在失败行正下方
- 重试按钮：`variant="ghost" size="sm"`，行末对齐

**重试点击流程**：

1. 调用 `setPipelineFailedStage(null)` 清除错误状态
2. 将 `recordingPhase` 设回 `"pipeline"`
3. 调用 `retry_pipeline_from_stage(meetingId, failedStage)`
4. 成功 → `recordingPhase = "done"`
5. 再次失败 → 重新设置 `pipelineFailedStage`

### Meeting 页面

`PipelineProgress` 显示条件扩展：

```typescript
{(recordingPhase === "pipeline" ||
  recordingPhase === "done" ||
  recordingPhase === "error") && (
  <PipelineProgress
    meetingId={meetingId}
    onRetryFromStage={handleRetryFromStage}
  />
)}
```

---

## i18n 新增 key

```typescript
// zh.ts
"meeting.pipeline.retryFromStage": "从此阶段重试",
"meeting.pipeline.failedStageHint": "第 {{stage}} 阶段失败",

// en.ts
"meeting.pipeline.retryFromStage": "Retry from here",
"meeting.pipeline.failedStageHint": "Stage {{stage}} failed",
```

---

## 边界情况

| 情况 | 处理 |
|------|------|
| Stage1 本身失败 | 直接从 Stage1 重跑（无需读中间结果） |
| 重试时 DB 中间数据缺失 | 降级到从 Stage1 重跑，前端无感知 |
| 连续多次失败 | 每次覆盖 `pipelineFailedStage`，始终显示最新失败阶段 |
| 用户切换页面再返回 | 事件监听随组件卸载清理；`pipelineFailedStage` 保留在 store，进度条恢复显示 |
| Stage5 写 action_items 前重试 | 先清除该 meeting 的旧 action_items，再重新 insert，避免重复 |

---

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `schema/001_init.sql` | 修改 | meetings 表新增两字段 |
| `src-tauri/src/db/models.rs` | 修改 | 新增读写 clean/organized_transcript 的方法 |
| `src-tauri/src/commands.rs` | 修改 | run_pipeline 逐阶段写 DB；新增 retry_pipeline_from_stage |
| `src-tauri/src/lib.rs` | 修改 | 注册新命令 |
| `src/types/index.ts` | 修改 | 新增 PipelineStageFailed 类型 |
| `src/store/meetingStore.ts` | 修改 | 新增 pipelineFailedStage 字段 |
| `src/components/PipelineProgress.tsx` | 修改 | 失败图标 + 错误信息 + 重试按钮 |
| `src/pages/Meeting.tsx` | 修改 | error 时显示进度条；传入 onRetryFromStage |
| `src/i18n/locales/zh.ts` | 修改 | 新增 i18n key |
| `src/i18n/locales/en.ts` | 修改 | 新增 i18n key |
