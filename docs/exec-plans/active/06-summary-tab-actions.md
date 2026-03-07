# Summary Tab Actions Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为总结选项卡添加复制、内联编辑（自动保存）、重新生成三个操作按钮，风格参考 ChatGPT ghost 图标按钮。

**Architecture:** 新建 `SummaryTab` 组件封装所有逻辑，Rust 侧新增两个 Tauri 命令（更新总结、重新生成总结），前端通过防抖自动保存同步到 SQLite。

**Tech Stack:** React 18 + TypeScript, Tauri invoke, Rust/rusqlite, Lucide React, Tailwind CSS, shadcn/ui Button

---

### Task 1: 添加 `update_meeting_summary` DB 函数

**Files:**
- Modify: `src-tauri/src/db/models.rs`（在 `update_meeting_summary_report` 之后约第 151 行添加新函数）

**Step 1: 添加函数**

在 `src-tauri/src/db/models.rs` 的 `update_meeting_summary_report` 函数（约第 138 行）之后，添加：

```rust
pub fn update_meeting_summary(conn: &Connection, id: i64, summary: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET summary = ?1, updated_at = ?2 WHERE id = ?3",
        params![summary, now, id],
    )?;
    Ok(())
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

预期：无错误（warnings 可忽略）

**Step 3: Commit**

```bash
git add src-tauri/src/db/models.rs
git commit -m "feat(db): add update_meeting_summary function"
```

---

### Task 2: 添加 `update_meeting_summary` Tauri 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`（在 `rename_meeting` 命令之后约第 194 行）

**Step 1: 添加命令**

在 `src-tauri/src/commands.rs` 的 `rename_meeting` 函数（约第 186 行）之后，添加：

```rust
#[tauri::command]
pub fn update_meeting_summary(
    id: i64,
    summary: String,
    db: State<'_, DbState>,
) -> Result<(), String> {
    let conn = (*db).0.lock().unwrap();
    models::update_meeting_summary(&conn, id, &summary).map_err(|e| e.to_string())
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

**Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): add update_meeting_summary tauri command"
```

---

### Task 3: 添加 `regenerate_summary` Tauri 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`（在 `update_meeting_summary` 之后添加）

**Step 1: 添加命令**

在上一步新增的 `update_meeting_summary` 之后添加：

```rust
#[tauri::command]
pub async fn regenerate_summary(
    meeting_id: i64,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<String, String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let llm_config = LlmConfig {
        provider: cfg.llm_provider.provider_type,
        base_url: cfg.llm_provider.base_url,
        model: cfg.llm_provider.model,
        api_key: cfg.llm_provider.api_key,
    };

    // 复用与 run_pipeline 相同的 prompts_dir 解析逻辑
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

    // 读取转写文本（锁立即释放）
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
        return Err("No transcript available to regenerate summary".into());
    }

    // 在独立 OS 线程中运行 LLM（避免 reqwest::blocking 与 Tokio 冲突）
    let (tx, rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let client = llm_config.build_client();
        let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);
        let result = pipeline.stage1_clean(&transcript_text)
            .and_then(|clean| pipeline.stage2_speaker(&clean))
            .and_then(|organized| pipeline.stage4_summary(&organized));
        let _ = tx.send(result);
    });

    let new_summary = rx.await
        .map_err(|_| "LLM thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

    // 写库
    {
        let conn = (*db).0.lock().unwrap();
        models::update_meeting_summary(&conn, meeting_id, &new_summary)
            .map_err(|e| e.to_string())?;
    }

    Ok(new_summary)
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

**Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): add regenerate_summary tauri command"
```

---

### Task 4: 注册新命令到 lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: 添加 import**

在 `src-tauri/src/lib.rs` 的 `use commands::{...}` 块（约第 9-16 行）中，在最后一行的分号前添加两个新命令：

```rust
use commands::{
    ConfigState, DbState, RecordState, FunAsrState,
    check_whisper_cli, create_meeting, delete_meeting, export_report, get_action_items, get_meeting,
    get_settings, get_transcripts, list_meetings, rename_meeting, run_pipeline, save_settings,
    search_meetings, start_recording, stop_recording, test_asr_connection, test_llm_connection,
    transcribe_audio, update_action_item_status,
    start_funasr_session, stop_funasr_session, check_funasr_server,
    update_meeting_summary, regenerate_summary,  // 新增
};
```

**Step 2: 注册到 invoke_handler**

在 `.invoke_handler(tauri::generate_handler![...])`（约第 68-91 行）中，在 `check_funasr_server,` 之后添加：

```rust
            update_meeting_summary,
            regenerate_summary,
```

**Step 3: 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

预期：无错误

**Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(lib): register update_meeting_summary and regenerate_summary commands"
```

---

### Task 5: 添加前端 hooks

**Files:**
- Modify: `src/hooks/useTauriCommands.ts`（在文件末尾添加）

**Step 1: 添加两个 hooks**

在 `useTauriCommands.ts` 文件末尾（第 166 行之后）追加：

```typescript
// Summary commands
export function useUpdateMeetingSummary() {
  return useCallback(
    (id: number, summary: string) =>
      invoke<void>("update_meeting_summary", { id, summary }),
    []
  );
}

export function useRegenerateSummary() {
  return useCallback(
    (meetingId: number) =>
      invoke<string>("regenerate_summary", { meetingId }),
    []
  );
}
```

**Step 2: 类型检查**

```bash
npx tsc --noEmit 2>&1 | head -20
```

预期：无错误

**Step 3: Commit**

```bash
git add src/hooks/useTauriCommands.ts
git commit -m "feat(hooks): add useUpdateMeetingSummary and useRegenerateSummary hooks"
```

---

### Task 6: 创建 SummaryTab 组件

**Files:**
- Create: `src/components/SummaryTab.tsx`

**Step 1: 创建组件文件**

创建 `src/components/SummaryTab.tsx`，内容如下：

```tsx
import { useState, useRef, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Pencil, Check, Copy, RefreshCw, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useUpdateMeetingSummary, useRegenerateSummary } from "@/hooks/useTauriCommands";
import type { Meeting } from "@/types";

interface SummaryTabProps {
  meeting: Meeting;
  onSummaryUpdated: (newSummary: string) => void;
}

export function SummaryTab({ meeting, onSummaryUpdated }: SummaryTabProps) {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [editText, setEditText] = useState(meeting.summary ?? "");
  const [isCopied, setIsCopied] = useState(false);
  const [isRegenerating, setIsRegenerating] = useState(false);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const updateMeetingSummary = useUpdateMeetingSummary();
  const regenerateSummary = useRegenerateSummary();

  // 当外部 meeting.summary 更新时（如重新生成后），同步 editText
  useEffect(() => {
    if (!isEditing) {
      setEditText(meeting.summary ?? "");
    }
  }, [meeting.summary, isEditing]);

  const saveWithDebounce = useCallback((text: string) => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }
    debounceTimerRef.current = setTimeout(() => {
      void updateMeetingSummary(meeting.id, text).then(() => {
        onSummaryUpdated(text);
      });
    }, 1000);
  }, [meeting.id, updateMeetingSummary, onSummaryUpdated]);

  // 卸载时 flush 未完成的保存
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, []);

  function handleEditToggle() {
    if (isEditing) {
      // 退出编辑：立即保存当前内容
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
        debounceTimerRef.current = null;
      }
      void updateMeetingSummary(meeting.id, editText).then(() => {
        onSummaryUpdated(editText);
      });
    }
    setIsEditing((prev) => !prev);
  }

  function handleTextChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    const text = e.target.value;
    setEditText(text);
    saveWithDebounce(text);
  }

  async function handleCopy() {
    const text = isEditing ? editText : (meeting.summary ?? "");
    await navigator.clipboard.writeText(text);
    setIsCopied(true);
    setTimeout(() => setIsCopied(false), 1500);
  }

  async function handleRegenerate() {
    setIsRegenerating(true);
    try {
      const newSummary = await regenerateSummary(meeting.id);
      setEditText(newSummary);
      onSummaryUpdated(newSummary);
    } catch (e) {
      console.error("Regenerate summary failed:", e);
    } finally {
      setIsRegenerating(false);
    }
  }

  const isProcessing = meeting.status === "processing";
  const hasSummary = !!meeting.summary;

  return (
    <div className="flex flex-col gap-2">
      {/* 工具栏 */}
      <div className="flex justify-end gap-1">
        {/* 编辑 / 完成 */}
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 rounded-md text-muted-foreground hover:text-foreground"
          title={isEditing ? t("summary.actions.finishEdit") : t("summary.actions.edit")}
          disabled={isProcessing || isRegenerating || !hasSummary}
          onClick={handleEditToggle}
        >
          {isEditing
            ? <Check className="h-4 w-4 text-green-500" />
            : <Pencil className="h-4 w-4" />
          }
        </Button>

        {/* 复制 */}
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 rounded-md text-muted-foreground hover:text-foreground"
          title={t("summary.actions.copy")}
          disabled={isProcessing || isRegenerating || !hasSummary}
          onClick={handleCopy}
        >
          {isCopied
            ? <Check className="h-4 w-4 text-green-500" />
            : <Copy className="h-4 w-4" />
          }
        </Button>

        {/* 重新生成 */}
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 rounded-md text-muted-foreground hover:text-foreground"
          title={t("summary.actions.regenerate")}
          disabled={isProcessing || isRegenerating || isEditing}
          onClick={handleRegenerate}
        >
          {isRegenerating
            ? <Loader2 className="h-4 w-4 animate-spin" />
            : <RefreshCw className="h-4 w-4" />
          }
        </Button>
      </div>

      {/* 内容区域 */}
      {isEditing ? (
        <textarea
          className="w-full min-h-[200px] resize-y rounded-md border border-border bg-background px-3 py-2 text-sm font-mono leading-relaxed text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          value={editText}
          onChange={handleTextChange}
          autoFocus
        />
      ) : hasSummary ? (
        <div className="p-4 text-sm leading-relaxed text-foreground prose prose-sm max-w-none dark:prose-invert">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
            {meeting.summary!}
          </ReactMarkdown>
        </div>
      ) : (
        <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          {t("meeting.noSummary")}
        </div>
      )}
    </div>
  );
}
```

**Step 2: 类型检查**

```bash
npx tsc --noEmit 2>&1 | head -20
```

预期：无错误

**Step 3: Commit**

```bash
git add src/components/SummaryTab.tsx
git commit -m "feat(components): add SummaryTab with copy/edit/regenerate actions"
```

---

### Task 7: 添加 i18n 键

**Files:**
- Modify: `src/i18n/` 目录下的语言文件（通常是 `zh.json` 和 `en.json` 或类似文件）

**Step 1: 查找 i18n 文件**

```bash
find src -name "*.json" | grep -E "i18n|locale|lang" | head -10
```

或者：

```bash
find src -name "zh.json" -o -name "en.json" -o -name "translation.json" | head -10
```

**Step 2: 添加 i18n 键**

在对应的 JSON 文件中（中文）添加：

```json
"summary": {
  "actions": {
    "edit": "编辑总结",
    "finishEdit": "完成编辑",
    "copy": "复制总结",
    "regenerate": "重新生成总结"
  }
}
```

英文版添加：

```json
"summary": {
  "actions": {
    "edit": "Edit summary",
    "finishEdit": "Done editing",
    "copy": "Copy summary",
    "regenerate": "Regenerate summary"
  }
}
```

**Step 3: Commit**

```bash
git add src/
git commit -m "feat(i18n): add summary action button keys"
```

---

### Task 8: 将 SummaryTab 接入 Meeting.tsx

**Files:**
- Modify: `src/pages/Meeting.tsx`

**Step 1: 添加 import**

在 `Meeting.tsx` 顶部 import 区域（约第 24 行附近，其他组件 import 处），添加：

```typescript
import { SummaryTab } from "@/components/SummaryTab";
```

**Step 2: 添加状态同步函数**

在 `Meeting.tsx` 的 `handleToggleActionItem` 函数（约第 172 行）之后添加：

```typescript
function handleSummaryUpdated(newSummary: string) {
  if (currentMeeting) {
    setCurrentMeeting({ ...currentMeeting, summary: newSummary });
  }
}
```

**Step 3: 替换 summary TabsContent**

将原有的 `<TabsContent value="summary" ...>` 块（约第 255-267 行）：

```tsx
<TabsContent value="summary" className="flex-1 overflow-auto mt-4">
  {currentMeeting.summary ? (
    <div className="p-4 text-sm leading-relaxed text-foreground prose prose-sm max-w-none dark:prose-invert">
      <ReactMarkdown remarkPlugins={[remarkGfm]}>
        {currentMeeting.summary}
      </ReactMarkdown>
    </div>
  ) : (
    <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
      {t("meeting.noSummary")}
    </div>
  )}
</TabsContent>
```

替换为：

```tsx
<TabsContent value="summary" className="flex-1 overflow-auto mt-4">
  <SummaryTab
    meeting={currentMeeting}
    onSummaryUpdated={handleSummaryUpdated}
  />
</TabsContent>
```

**Step 4: 检查 ReactMarkdown/remarkGfm import 是否仍被使用**

查看 `Meeting.tsx` 中是否还有其他地方使用 `ReactMarkdown` 和 `remarkGfm`（报告选项卡也用了，所以无需删除 import）。

**Step 5: 类型检查**

```bash
npx tsc --noEmit 2>&1 | head -20
```

预期：无错误

**Step 6: Commit**

```bash
git add src/pages/Meeting.tsx
git commit -m "feat(meeting): integrate SummaryTab component into summary tab"
```

---

### Task 9: 验证功能

**Step 1: 启动开发模式**

```bash
npm run tauri:dev
```

**Step 2: 验证清单**

- [ ] 总结选项卡顶部显示三个小图标按钮（铅笔、复制、刷新）
- [ ] 无总结时，复制和编辑按钮为 disabled，重新生成可点击
- [ ] 点击铅笔图标，Markdown 渲染切换为 textarea
- [ ] 编辑时图标变为绿色 ✓，点击退出编辑并保存
- [ ] 编辑内容 1 秒后自动保存（刷新页面后内容保留）
- [ ] 点击复制后图标变为绿色 ✓，1.5 秒后恢复
- [ ] 点击重新生成后显示旋转 Loader，完成后更新内容
- [ ] 会议处理中时三个按钮全部 disabled

**Step 3: 最终 commit（如有遗漏修复）**

```bash
git add -p
git commit -m "fix(summary-tab): address review findings"
```
