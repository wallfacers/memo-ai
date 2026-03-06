# v1.0 先行特性 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现两项 v1.0 先行特性：会议报告导出为 Markdown 文件、Sidebar 会议历史搜索。

**Architecture:** 后端新增 `export_report` 和 `search_meetings` 两个 Tauri 命令；前端 Meeting 页"报告"Tab 增加导出按钮（使用 plugin-dialog save 对话框），Sidebar 顶部增加搜索 Input（300ms debounce）。

**Tech Stack:** Rust（rusqlite、std::fs）、React + TypeScript、@tauri-apps/plugin-dialog、shadcn/ui Input、useTauriCommands hook

**依赖：** 建议在 01-mvp-completion.md 完成后执行（useTauriCommands hook 需已存在）。

---

## Task 1: 后端 `export_report` 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`

**背景：** 从数据库读取指定会议的标题、总结、行动项、报告，拼装为 Markdown 并写入用户指定路径。

**Step 1: 在 `commands.rs` 中添加 `export_report` 命令**

在文件末尾（`fn get_app_config` 之前）添加：

```rust
#[tauri::command]
pub fn export_report(
    meeting_id: i64,
    path: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let db = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;

    // Fetch meeting basic info
    let (title, start_time): (String, String) = db.query_row(
        "SELECT title, start_time FROM meetings WHERE id = ?1",
        rusqlite::params![meeting_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| AppError::Database(e.to_string()))?;

    // Fetch summary and report from meetings table (may be NULL)
    let (summary, report): (Option<String>, Option<String>) = db.query_row(
        "SELECT summary, report FROM meetings WHERE id = ?1",
        rusqlite::params![meeting_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|e| AppError::Database(e.to_string()))?;

    // Fetch action items
    let mut stmt = db.prepare(
        "SELECT task, owner, deadline, done FROM action_items WHERE meeting_id = ?1 ORDER BY id"
    ).map_err(|e| AppError::Database(e.to_string()))?;

    let action_lines: Vec<String> = stmt.query_map(
        rusqlite::params![meeting_id],
        |row| {
            let task: String = row.get(0)?;
            let owner: Option<String> = row.get(1)?;
            let deadline: Option<String> = row.get(2)?;
            let done: bool = row.get::<_, i64>(3).map(|v| v != 0)?;
            Ok((task, owner, deadline, done))
        },
    ).map_err(|e| AppError::Database(e.to_string()))?
    .filter_map(|r| r.ok())
    .map(|(task, owner, deadline, done)| {
        let checkbox = if done { "[x]" } else { "[ ]" };
        let meta = match (owner, deadline) {
            (Some(o), Some(d)) => format!("（{} / {}）", o, d),
            (Some(o), None) => format!("（{}）", o),
            (None, Some(d)) => format!("（{}）", d),
            (None, None) => String::new(),
        };
        format!("- {} {}{}", checkbox, task, meta)
    })
    .collect();

    // Assemble Markdown
    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", title));
    md.push_str(&format!("**日期：** {}\n\n", start_time));

    md.push_str("## 会议总结\n\n");
    md.push_str(summary.as_deref().unwrap_or("（暂无总结）"));
    md.push_str("\n\n");

    md.push_str("## 行动项\n\n");
    if action_lines.is_empty() {
        md.push_str("（暂无行动项）\n");
    } else {
        for line in &action_lines {
            md.push_str(line);
            md.push('\n');
        }
    }
    md.push('\n');

    md.push_str("## 完整报告\n\n");
    md.push_str(report.as_deref().unwrap_or("（暂无报告）"));
    md.push('\n');

    // Write to file
    std::fs::write(&path, md).map_err(|e| AppError::Internal(format!("Write file failed: {}", e)))?;

    log::info!("Report exported to: {}", path);
    Ok(())
}
```

**Step 2: 在 `lib.rs` 的 `invoke_handler` 中注册命令**

找到 `tauri::generate_handler![...]`，添加 `commands::export_report`：

```rust
tauri::generate_handler![
    // ... 已有命令 ...
    commands::export_report,
]
```

**Step 3: 验证编译**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|Finished)"
```

Expected: `Finished` 无 error

**Step 4: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/commands.rs src-tauri/src/lib.rs
git -C "D:/project/java/source/memo-ai" commit -m "feat: add export_report Tauri command"
```

---

## Task 2: 后端 `search_meetings` 命令

**Files:**
- Modify: `src-tauri/src/commands.rs`

**背景：** 按标题或总结字段模糊搜索会议列表，返回与 `list_meetings` 相同的 `Meeting` 结构体列表。

**Step 1: 在 `commands.rs` 中添加 `search_meetings` 命令**

```rust
#[tauri::command]
pub fn search_meetings(
    query: String,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<Meeting>> {
    let db = state.db.lock().map_err(|_| AppError::Internal("DB lock poisoned".into()))?;
    let pattern = format!("%{}%", query);
    let mut stmt = db.prepare(
        "SELECT id, title, start_time, end_time, audio_path, status, summary, report
         FROM meetings
         WHERE title LIKE ?1 OR summary LIKE ?1
         ORDER BY start_time DESC"
    ).map_err(|e| AppError::Database(e.to_string()))?;

    let meetings = stmt.query_map(
        rusqlite::params![pattern],
        |row| Ok(Meeting {
            id: row.get(0)?,
            title: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
            audio_path: row.get(4)?,
            status: row.get(5)?,
            summary: row.get(6)?,
            report: row.get(7)?,
        }),
    ).map_err(|e| AppError::Database(e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(meetings)
}
```

**Step 2: 注册命令**

在 `lib.rs` 的 `invoke_handler` 中添加 `commands::search_meetings`。

**Step 3: 验证编译**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|Finished)"
```

Expected: `Finished` 无 error

**Step 4: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/commands.rs src-tauri/src/lib.rs
git -C "D:/project/java/source/memo-ai" commit -m "feat: add search_meetings Tauri command"
```

---

## Task 3: 前端 useTauriCommands 新增两个命令

**Files:**
- Modify: `src/hooks/useTauriCommands.ts`
- Modify: `src/types/index.ts`（如 `export_report` 需要的类型）

**Step 1: 在 `useTauriCommands.ts` 中添加 `exportReport` 和 `searchMeetings`**

```typescript
export async function exportReport(meetingId: number, path: string): Promise<void> {
  await invoke<void>("export_report", { meetingId, path });
}

export async function searchMeetings(query: string): Promise<Meeting[]> {
  return await invoke<Meeting[]>("search_meetings", { query });
}
```

**Step 2: 类型检查**

```bash
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -20
```

Expected: 无 error

**Step 3: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src/hooks/useTauriCommands.ts
git -C "D:/project/java/source/memo-ai" commit -m "feat: expose exportReport and searchMeetings in useTauriCommands"
```

---

## Task 4: 前端 Meeting 页「导出 .md」按钮

**Files:**
- Modify: `src/pages/Meeting.tsx`

**背景：** 在"报告"Tab 内容区顶部右侧添加「导出 .md」按钮。点击后弹出系统文件保存对话框，选择路径后调用 `exportReport`。

**Step 1: 安装 plugin-dialog（如未安装）**

读取 `src-tauri/Cargo.toml`，确认是否已有：

```toml
tauri-plugin-dialog = "2"
```

若无，在 `[dependencies]` 中添加。同时确认 `package.json` 中有：

```json
"@tauri-apps/plugin-dialog": "^2.0.0"
```

若无，执行：

```bash
npm --prefix "D:/project/java/source/memo-ai" install @tauri-apps/plugin-dialog
```

在 `src-tauri/src/lib.rs` 的 `tauri::Builder` 中注册插件（若未注册）：

```rust
.plugin(tauri_plugin_dialog::init())
```

**Step 2: 在 `Meeting.tsx` 的"报告"Tab 中添加导出按钮**

在文件顶部添加 import：

```typescript
import { save } from "@tauri-apps/plugin-dialog";
import { exportReport } from "@/hooks/useTauriCommands";
```

在"报告"Tab 的 `TabsContent` 内，顶部添加导出按钮区域：

```tsx
<TabsContent value="report" className="p-4">
  <div className="mb-3 flex justify-end">
    <button
      onClick={async () => {
        const filePath = await save({
          defaultPath: `${meeting?.title ?? "report"}.md`,
          filters: [{ name: "Markdown", extensions: ["md"] }],
        });
        if (filePath && meeting) {
          try {
            await exportReport(meeting.id, filePath);
          } catch (e) {
            console.error("Export failed:", e);
          }
        }
      }}
      className="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
    >
      导出 .md
    </button>
  </div>
  {/* 原有报告内容 */}
  <div className="prose prose-sm max-w-none whitespace-pre-wrap text-sm text-foreground">
    {meeting?.report ?? "报告尚未生成"}
  </div>
</TabsContent>
```

**Step 3: 类型检查**

```bash
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -20
```

Expected: 无 error

**Step 4: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src/pages/Meeting.tsx src-tauri/Cargo.toml src-tauri/src/lib.rs package.json
git -C "D:/project/java/source/memo-ai" commit -m "feat: add export report button to Meeting page"
```

---

## Task 5: 前端 Sidebar 会议搜索

**Files:**
- Modify: `src/components/Sidebar.tsx`

**背景：** 在新建会议输入框上方添加搜索 Input，输入时 debounce 300ms 调用 `searchMeetings`，结果替换会议列表；清空时恢复全量列表。

**Step 1: 在 `Sidebar.tsx` 中添加搜索状态和 debounce 逻辑**

添加 import：

```typescript
import { useRef, useState, useEffect, useCallback } from "react";
import { Search } from "lucide-react";
import { searchMeetings } from "@/hooks/useTauriCommands";
```

在组件内添加状态：

```typescript
const [searchQuery, setSearchQuery] = useState("");
const [searchResults, setSearchResults] = useState<Meeting[] | null>(null);
const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
```

添加搜索处理函数：

```typescript
const handleSearch = useCallback((q: string) => {
  setSearchQuery(q);
  if (debounceRef.current) clearTimeout(debounceRef.current);
  if (!q.trim()) {
    setSearchResults(null);
    return;
  }
  debounceRef.current = setTimeout(async () => {
    try {
      const results = await searchMeetings(q.trim());
      setSearchResults(results);
    } catch (e) {
      console.error("Search failed:", e);
    }
  }, 300);
}, []);
```

cleanup effect：

```typescript
useEffect(() => {
  return () => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
  };
}, []);
```

**Step 2: 添加搜索 Input UI**

在新建会议 Input 之前插入：

```tsx
{/* 搜索框 */}
<div className="relative">
  <Search className="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
  <input
    type="text"
    value={searchQuery}
    onChange={(e) => handleSearch(e.target.value)}
    placeholder="搜索会议..."
    className="w-full rounded-md border border-input bg-background py-1.5 pl-8 pr-3 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
  />
</div>
```

**Step 3: 会议列表使用搜索结果**

将渲染列表的数据源从 `meetings` 改为：

```typescript
const displayedMeetings = searchResults ?? meetings;
```

在列表为空时的 empty state 区分搜索模式：

```tsx
{displayedMeetings.length === 0 && (
  <p className="px-2 py-4 text-center text-xs text-muted-foreground">
    {searchQuery ? "无匹配会议" : "暂无会议"}
  </p>
)}
```

**Step 4: 类型检查**

```bash
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -20
```

Expected: 无 error

**Step 5: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src/components/Sidebar.tsx
git -C "D:/project/java/source/memo-ai" commit -m "feat: add meeting search with debounce to Sidebar"
```

---

## 完成标准

- [ ] Meeting 页"报告"Tab 显示「导出 .md」按钮
- [ ] 点击按钮弹出系统文件保存对话框，选择路径后生成包含标题/总结/行动项/报告的 Markdown 文件
- [ ] Sidebar 顶部有搜索框，输入后 300ms 内过滤会议列表
- [ ] 搜索无结果时显示"无匹配会议"，清空后恢复完整列表
- [ ] `cargo check` 和 `tsc --noEmit` 均无 error
