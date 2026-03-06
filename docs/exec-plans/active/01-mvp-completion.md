# MVP 补完 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 完成 MVP 完成标准中尚未实现的四项：设置持久化、Whisper CLI 真实集成、前端 invoke 合规迁移、AI 自动生成会议标题。

**Architecture:** 设置持久化写入 `app_data_dir/settings.json`；Whisper 通过可配置路径调用 whisper.cpp 的 `whisper-cli` 可执行文件；所有前端 invoke 调用迁移至 `useTauriCommands.ts`；Pipeline 完成后若会议标题为自动生成则调用 Stage 7 AI 生成标题。

**Tech Stack:** Rust（serde_json, tauri::Manager）、React + TypeScript、shadcn/ui Input、prompts/07_title.txt

---

## Task 1: 设置持久化 — 后端读写 settings.json

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`

**背景：** 当前 `save_settings` 只更新内存中的 `ConfigState`，重启后配置丢失。

**Step 1: 在 `commands.rs` 中添加 settings 文件路径辅助函数**

在 `AppConfig` 的 `impl Default` 之后添加：

```rust
fn settings_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("settings.json"))
        .map_err(|e| e.to_string())
}
```

**Step 2: 修改 `save_settings` 命令，写入磁盘**

将现有 `save_settings` 替换为：

```rust
#[tauri::command]
pub fn save_settings(
    settings: AppConfig,
    config: State<'_, ConfigState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    let path = settings_path(&app_handle)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    *(*config).0.lock().unwrap() = settings;
    Ok(())
}
```

**Step 3: 修改 `lib.rs` 的 setup 块，启动时读取 settings.json**

将 `app.manage(ConfigState(Mutex::new(commands::AppConfig::default())));` 替换为：

```rust
let settings_path = data_dir.join("settings.json");
let config = if settings_path.exists() {
    std::fs::read_to_string(&settings_path)
        .ok()
        .and_then(|s| serde_json::from_str::<commands::AppConfig>(&s).ok())
        .unwrap_or_default()
} else {
    commands::AppConfig::default()
};
app.manage(ConfigState(Mutex::new(config)));
```

**Step 4: 验证编译通过**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|warning.*unused|Finished)"
```

Expected: `Finished` 无 error

**Step 5: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/commands.rs src-tauri/src/lib.rs
git -C "D:/project/java/source/memo-ai" commit -m "feat: persist settings to app_data_dir/settings.json"
```

---

## Task 2: Whisper CLI 集成 — 可配置路径 + 真实调用

**Files:**
- Modify: `src-tauri/src/commands.rs`（AppConfig 新增字段）
- Modify: `src-tauri/src/asr/whisper.rs`
- Modify: `src/types/index.ts`（AppSettings 新增字段）
- Modify: `src/pages/Settings.tsx`（新增 Input）
- Modify: `src/hooks/useTauriCommands.ts`（无需改动，已有 useTranscribeAudio）

**Step 1: `AppConfig` 新增 `whisper_cli_path` 字段**

在 `commands.rs` 的 `AppConfig` struct 中添加字段：

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm_provider: LlmProviderConfig,
    pub whisper_model: String,
    pub language: String,
    pub whisper_cli_path: String,   // 新增
}
```

在 `impl Default for AppConfig` 中添加默认值：

```rust
whisper_cli_path: "whisper-cli".into(),  // 默认用 PATH 中的 whisper-cli
```

**Step 2: 修改 `WhisperAsr`，接收完整 CLI 路径**

将 `whisper.rs` 完整替换为：

```rust
use std::path::Path;
use crate::error::{AppError, AppResult};
use super::transcript::TranscriptSegment;

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

    pub fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>> {
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

    // whisper.cpp JSON format uses "transcription" array
    let output: WOutput = serde_json::from_str(json_str)?;
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

**Step 3: 更新 `transcribe_audio` 命令使用新构造函数**

在 `commands.rs` 的 `transcribe_audio` 中，将：

```rust
let model_path = format!("models/ggml-{}.bin", cfg.whisper_model);
let asr = WhisperAsr::new(&model_path, &cfg.language);
```

替换为：

```rust
let model_path = format!("models/ggml-{}.bin", cfg.whisper_model);
let asr = WhisperAsr::new(&cfg.whisper_cli_path, &model_path, &cfg.language);
```

**Step 4: 更新前端类型和 Settings UI**

在 `src/types/index.ts` 的 `AppSettings` 接口中添加：

```typescript
export interface AppSettings {
  llm_provider: LlmProvider;
  whisper_model: string;
  language: string;
  whisper_cli_path: string;   // 新增
}
```

在 `src/pages/Settings.tsx` 的 ASR 配置 Card 中，`识别语言` Select 之后添加：

```tsx
<div className="space-y-1.5">
  <label className="text-sm font-medium text-foreground">
    Whisper CLI 路径
  </label>
  <Input
    value={local.whisper_cli_path}
    onChange={(e) =>
      setLocal({ ...local, whisper_cli_path: e.target.value })
    }
    placeholder="whisper-cli 或绝对路径"
  />
  <p className="text-[11px] text-muted-foreground">
    下载：github.com/ggerganov/whisper.cpp/releases
  </p>
</div>
```

**Step 5: 验证编译 + 类型检查**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|Finished)"
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -20
```

Expected: 两者均无 error

**Step 6: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/asr/whisper.rs src-tauri/src/commands.rs src/types/index.ts src/pages/Settings.tsx
git -C "D:/project/java/source/memo-ai" commit -m "feat: configurable whisper-cli path and real CLI integration"
```

---

## Task 3: 前端 invoke 合规 — 迁移至 useTauriCommands

**Files:**
- Modify: `src/hooks/useTauriCommands.ts`
- Modify: `src/components/Sidebar.tsx`
- Modify: `src/pages/Meeting.tsx`
- Modify: `src/pages/Settings.tsx`

**背景：** FRONTEND.md 要求所有 invoke() 通过 useTauriCommands 封装。当前 Sidebar、Meeting、Settings 直接调用 invoke()。

**Step 1: 确认 useTauriCommands.ts 已有所有命令**

检查 `src/hooks/useTauriCommands.ts`，确认以下 hook 均已存在：
- `useListMeetings` / `useCreateMeeting`
- `useGetMeeting` / `useGetTranscripts` / `useGetActionItems`
- `useTranscribeAudio` / `useRunPipeline`
- `useUpdateActionItemStatus`
- `useGetSettings` / `useSaveSettings`

如缺少则补充（当前文件已齐全，无需修改）。

**Step 2: 迁移 Sidebar.tsx**

在 `Sidebar.tsx` 顶部删除 `import { invoke } from "@tauri-apps/api/core";`，改为：

```typescript
import { useListMeetings, useCreateMeeting } from "@/hooks/useTauriCommands";
```

在 `Sidebar` 函数体内添加：

```typescript
const listMeetings = useListMeetings();
const createMeetingCmd = useCreateMeeting();
```

将 `useEffect` 内的 invoke 调用改为：

```typescript
listMeetings()
  .then(setMeetings)
  .catch((e) => setError(String(e)));
```

将 `createMeeting` 函数内的 invoke 调用改为：

```typescript
const meeting = await createMeetingCmd(title);
```

**Step 3: 迁移 Meeting.tsx**

删除 `import { invoke } from "@tauri-apps/api/core";`，改为：

```typescript
import {
  useGetMeeting,
  useGetTranscripts,
  useGetActionItems,
  useTranscribeAudio,
  useRunPipeline,
  useUpdateActionItemStatus,
} from "@/hooks/useTauriCommands";
```

在 `Meeting` 函数体内初始化所有 hook：

```typescript
const getMeeting = useGetMeeting();
const getTranscripts = useGetTranscripts();
const getActionItems = useGetActionItems();
const transcribeAudio = useTranscribeAudio();
const runPipeline = useRunPipeline();
const updateActionItemStatus = useUpdateActionItemStatus();
```

将各 `loadXxx` 函数和 `handleStopAndProcess` 内的直接 invoke 替换为对应 hook 调用：

```typescript
async function loadMeeting() {
  const meeting = await getMeeting(meetingId!);
  setCurrentMeeting(meeting);
}
async function loadTranscripts() {
  const data = await getTranscripts(meetingId!);
  setTranscripts(data);
}
async function loadActionItems() {
  const data = await getActionItems(meetingId!);
  setActionItems(data);
}
// handleStopAndProcess 中：
await transcribeAudio(audioPath, meetingId!);
const result = await runPipeline(meetingId!);
// handleToggleActionItem 中：
await updateActionItemStatus(itemId, status);
```

**Step 4: 迁移 Settings.tsx**

删除直接 invoke，改用 hook：

```typescript
import { useGetSettings, useSaveSettings } from "@/hooks/useTauriCommands";
// ...
const getSettings = useGetSettings();
const saveSettings = useSaveSettings();
// useEffect 中：
getSettings().then((s) => { ... })
// handleSave 中：
await saveSettings(local);
```

**Step 5: 类型检查**

```bash
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -30
```

Expected: 无 error

**Step 6: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src/components/Sidebar.tsx src/pages/Meeting.tsx src/pages/Settings.tsx
git -C "D:/project/java/source/memo-ai" commit -m "refactor: migrate all invoke() calls to useTauriCommands hooks"
```

---

## Task 4: AI 自动生成会议标题 — Schema + Pipeline + 前端

**Files:**
- Modify: `schema/init.sql`（新增 auto_titled 字段）
- Modify: `src-tauri/src/db/models.rs`（新增 update_meeting_title）
- Modify: `src-tauri/src/db/connection.rs`（migration）
- Modify: `src-tauri/src/commands.rs`（create_meeting 接收 auto_titled；run_pipeline 调用 stage7）
- Modify: `src-tauri/src/llm/pipeline.rs`（新增 stage7_title；PipelineOutput 新增字段）
- Create: `prompts/07_title.txt`
- Modify: `src-tauri/src/lib.rs`（注册新命令 — 如有）
- Modify: `src/hooks/useTauriCommands.ts`（useCreateMeeting 加参数）
- Modify: `src/types/index.ts`（PipelineResult 新增字段）
- Modify: `src/components/Sidebar.tsx`（传 autoTitled）
- Modify: `src/pages/Meeting.tsx`（接收 generated_title 并更新）

**Step 1: 新增 prompt 文件**

创建 `prompts/07_title.txt`，内容：

```
你是会议助手。请根据以下会议摘要，生成一个简洁的会议标题。

要求：
- 10个汉字以内
- 直接输出标题，不加引号、不加任何前缀
- 能体现会议核心议题

会议摘要：
{{summary}}
```

**Step 2: 更新 schema — 新增 auto_titled 字段**

在 `schema/init.sql` 的 meetings 表定义中添加字段（在 `updated_at` 之前）：

```sql
    auto_titled INTEGER NOT NULL DEFAULT 0,
```

同时在末尾添加 ALTER TABLE 语句（用于已有数据库的在线迁移）：

```sql
-- Migration: add auto_titled column if not exists
-- Executed idempotently via connection.rs migration logic
```

**Step 3: 在 `connection.rs` 添加迁移逻辑**

将 `init_db` 函数修改为：

```rust
pub fn init_db(db_path: &Path) -> AppResult<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    conn.execute_batch(SCHEMA)?;

    // Inline migration: add auto_titled if missing (idempotent)
    let has_col: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='auto_titled'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);
    if has_col == 0 {
        conn.execute_batch("ALTER TABLE meetings ADD COLUMN auto_titled INTEGER NOT NULL DEFAULT 0;")?;
        log::info!("Migration: added auto_titled column to meetings");
    }

    Ok(conn)
}
```

**Step 4: 在 `models.rs` 新增 update_meeting_title**

在 `update_meeting_summary_report` 之后添加：

```rust
pub fn update_meeting_title(conn: &Connection, id: i64, title: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET title = ?1, auto_titled = 0, updated_at = ?2 WHERE id = ?3",
        params![title, now, id],
    )?;
    Ok(())
}
```

同时更新 `Meeting` struct 加入字段（在 `created_at` 之前）：

```rust
pub auto_titled: bool,
```

更新 `row_to_meeting` 函数，最后一行 `created_at: row.get(8)?` 改为 column 索引递增（meetings 表现在有 11 列），并加入：

```rust
pub fn row_to_meeting(row: &Row<'_>) -> rusqlite::Result<Meeting> {
    Ok(Meeting {
        id: row.get(0)?,
        title: row.get(1)?,
        start_time: row.get(2)?,
        end_time: row.get(3)?,
        status: row.get(4)?,
        summary: row.get(5)?,
        report: row.get(6)?,
        audio_path: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        auto_titled: row.get::<_, i64>(10).map(|v| v != 0).unwrap_or(false),
    })
}
```

更新所有 SELECT 查询，在 `updated_at` 后加 `auto_titled`：

```sql
SELECT id, title, start_time, end_time, status, summary, report, audio_path, created_at, updated_at, auto_titled FROM meetings ...
```

**Step 5: 更新 `create_meeting` 命令接收 auto_titled**

在 `commands.rs` 修改：

```rust
#[tauri::command]
pub fn create_meeting(
    title: String,
    auto_titled: bool,
    db: State<'_, DbState>,
) -> Result<Meeting, String> {
    let conn = (*db).0.lock().unwrap();
    models::create_meeting(&conn, &title, auto_titled).map_err(|e| e.to_string())
}
```

同时更新 `models::create_meeting` 接受 `auto_titled: bool`：

```rust
pub fn create_meeting(conn: &Connection, title: &str, auto_titled: bool) -> AppResult<Meeting> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO meetings (title, start_time, status, auto_titled, created_at, updated_at) VALUES (?1, ?2, 'idle', ?3, ?2, ?2)",
        params![title, now, auto_titled as i64],
    )?;
    let id = conn.last_insert_rowid();
    get_meeting(conn, id)
}
```

**Step 6: 在 pipeline.rs 新增 stage7_title 并集成到 run()**

在 `Pipeline` impl 中添加：

```rust
pub fn stage7_title(&self, summary: &str) -> AppResult<String> {
    let template = self.load_prompt("07_title.txt")?;
    let prompt = Self::fill_template(&template, "summary", summary);
    log::info!("Running pipeline stage 7: title generation");
    let title = self.client.complete(&prompt)?;
    Ok(title.trim().to_string())
}
```

在 `PipelineOutput` struct 中添加字段：

```rust
pub generated_title: Option<String>,
```

修改 `run()` 函数，在返回前新增：

```rust
// run() 方法签名增加 auto_titled 参数
pub fn run(&self, raw_transcript: &str, auto_titled: bool) -> AppResult<PipelineOutput> {
    let clean = self.stage1_clean(raw_transcript)?;
    let organized = self.stage2_speaker(&clean)?;
    let structure = self.stage3_structure(&organized)?;
    let summary = self.stage4_summary(&organized)?;
    let action_items = self.stage5_actions(&organized)?;
    let actions_json = serde_json::to_string(&action_items)?;
    let report = self.stage6_report(&summary, &actions_json)?;

    let generated_title = if auto_titled {
        match self.stage7_title(&summary) {
            Ok(t) => Some(t),
            Err(e) => {
                log::warn!("Stage 7 title generation failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    Ok(PipelineOutput {
        clean_transcript: clean,
        structure,
        summary,
        action_items,
        report,
        generated_title,
    })
}
```

**Step 7: 更新 `run_pipeline` 命令**

在 `commands.rs` 中，`run_pipeline` 命令：
1. 从 DB 查出当前会议的 `auto_titled` 值
2. 传给 `pipeline.run()`
3. 若返回 `generated_title`，调用 `models::update_meeting_title`
4. 在 `PipelineResult` 中增加 `generated_title: Option<String>`

```rust
#[derive(Serialize)]
pub struct PipelineResult {
    pub clean_transcript: String,
    pub summary: String,
    pub report: String,
    pub generated_title: Option<String>,   // 新增
}
```

在 `run_pipeline` 函数体中，`let transcript_text = {...}` 之前添加：

```rust
// Get auto_titled flag
let auto_titled = {
    let conn = (*db).0.lock().unwrap();
    models::get_meeting(&conn, meeting_id)
        .map(|m| m.auto_titled)
        .unwrap_or(false)
};
```

将 `pipeline.run(&transcript_text)` 改为 `pipeline.run(&transcript_text, auto_titled)`。

在保存 action items 之后，title 更新：

```rust
if let Some(ref title) = output.generated_title {
    models::update_meeting_title(&conn, meeting_id, title)
        .map_err(|e| e.to_string())?;
}
```

最后 `Ok(PipelineResult {...})` 中加入：

```rust
generated_title: output.generated_title,
```

**Step 8: 更新前端类型**

在 `src/types/index.ts` 的 `PipelineResult` 接口中添加：

```typescript
export interface PipelineResult {
  clean_transcript: string;
  structure: MeetingStructure;
  summary: string;
  action_items: ActionItem[];
  report: string;
  generated_title?: string;   // 新增
}
```

在 `Meeting` 接口中添加：

```typescript
auto_titled: boolean;
```

**Step 9: 更新 useTauriCommands — useCreateMeeting 传 autoTitled**

```typescript
export function useCreateMeeting() {
  return (title: string, autoTitled: boolean = false) =>
    invoke<Meeting>("create_meeting", { title, autoTitled });
}
```

**Step 10: 更新 Sidebar — 传 autoTitled**

在 `createMeeting` 函数中：

```typescript
const autoTitled = newTitle.trim() === "";
const title = newTitle.trim() || `会议 ${new Date().toLocaleString("zh-CN")}`;
const meeting = await createMeetingCmd(title, autoTitled);
```

**Step 11: 更新 Meeting.tsx — 收到 generated_title 后更新标题**

在 `handleStopAndProcess` 中，`run_pipeline` 完成后：

```typescript
const result = await runPipeline(meetingId!);
setCurrentMeetingStatus("completed");
await loadMeeting();
await loadActionItems();
// 若有 AI 生成的标题，更新当前显示
if (result.generated_title) {
  setCurrentMeeting((prev) =>
    prev ? { ...prev, title: result.generated_title! } : prev
  );
}
```

注意：`useMeetingStore` 的 `setCurrentMeeting` 接受完整 Meeting 对象，需确认该函数签名支持此用法。若不支持，改为：

```typescript
await loadMeeting();  // reload 会自动拿到更新后的标题
```

**Step 12: 验证编译 + 类型检查**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|Finished)"
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -30
```

Expected: 均无 error

**Step 13: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add prompts/07_title.txt schema/init.sql src-tauri/src/ src/
git -C "D:/project/java/source/memo-ai" commit -m "feat: AI auto-generate meeting title via pipeline stage 7"
```

---

## 完成标准

- [ ] 重启应用后，LLM 配置和 Whisper 路径保持
- [ ] Settings 页可配置 whisper-cli 路径，保存后生效
- [ ] 配置真实 whisper-cli 可执行文件路径后，转写返回真实内容
- [ ] 所有页面/组件不再有 `import { invoke }` 直接调用（通过 grep 确认）
- [ ] 用户未填写标题创建的会议，pipeline 完成后标题自动更新为 AI 生成内容
- [ ] `cargo check` 和 `tsc --noEmit` 均无 error
