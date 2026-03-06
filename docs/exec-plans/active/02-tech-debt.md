# 技术债清理 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 清理 tech-debt-tracker 中记录的 4 项技术债：Pipeline JSON 降级、Whisper 模型路径可配置、前端 ErrorBoundary、数据库迁移版本管理。

**Architecture:** Pipeline 各阶段 JSON 解析失败时返回空值而非 Err，继续后续阶段；Whisper 模型目录通过 Settings 配置；ErrorBoundary 包裹 React main 区域；DB 用 PRAGMA user_version 管理 schema 版本。

**Tech Stack:** Rust（rusqlite PRAGMA）、React Class Component（ErrorBoundary）、shadcn/ui Input

**依赖：** 建议在 01-mvp-completion.md 完成后执行（避免合并冲突）。

---

## Task 1: Pipeline JSON 解析降级 — Stage 3 / Stage 5

**Files:**
- Modify: `src-tauri/src/llm/pipeline.rs`

**背景：** `stage3_structure()` 和 `stage5_actions()` 在 LLM 返回非法 JSON 时直接返回 `Err`，导致整个 pipeline 中断。应降级为空值，继续执行后续阶段。

**Step 1: 修改 `stage3_structure` 返回 `StructuredMeeting` 而非 `AppResult<StructuredMeeting>`**

将函数签名和实现改为：

```rust
/// Stage 3: Extract structured info (JSON). Falls back to empty struct on parse failure.
pub fn stage3_structure(&self, meeting_text: &str) -> StructuredMeeting {
    let result = (|| -> AppResult<StructuredMeeting> {
        let template = self.load_prompt("03_structure.txt")?;
        let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
        log::info!("Running pipeline stage 3: structure extraction");
        let response = self.client.complete(&prompt)?;
        let json_str = extract_json(&response);
        serde_json::from_str(json_str)
            .map_err(|e| AppError::Llm(format!("Stage 3 JSON parse failed: {}. Raw: {}", e, response)))
    })();

    match result {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Stage 3 failed, using empty structure: {}", e);
            StructuredMeeting {
                topic: None,
                participants: vec![],
                key_points: vec![],
                decisions: vec![],
                risks: vec![],
            }
        }
    }
}
```

**Step 2: 修改 `stage5_actions` 返回 `Vec<ActionItemRaw>` 而非 `AppResult<Vec<ActionItemRaw>>`**

```rust
/// Stage 5: Extract action items (JSON array). Falls back to empty vec on parse failure.
pub fn stage5_actions(&self, meeting_text: &str) -> Vec<ActionItemRaw> {
    let result = (|| -> AppResult<Vec<ActionItemRaw>> {
        let template = self.load_prompt("05_actions.txt")?;
        let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
        log::info!("Running pipeline stage 5: action items");
        let response = self.client.complete(&prompt)?;
        let json_str = extract_json(&response);
        serde_json::from_str(json_str)
            .map_err(|e| AppError::Llm(format!("Stage 5 JSON parse failed: {}. Raw: {}", e, response)))
    })();

    match result {
        Ok(items) => items,
        Err(e) => {
            log::warn!("Stage 5 failed, returning empty action items: {}", e);
            vec![]
        }
    }
}
```

**Step 3: 更新 `run()` 方法调用（去掉 `?` 操作符）**

在 `run()` 函数中，将：

```rust
let structure = self.stage3_structure(&organized)?;
// ...
let action_items = self.stage5_actions(&organized)?;
```

改为（去掉 `?`，因为这两个函数不再返回 Result）：

```rust
let structure = self.stage3_structure(&organized);
// ...
let action_items = self.stage5_actions(&organized);
```

**Step 4: 验证编译**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|Finished)"
```

Expected: `Finished` 无 error

**Step 5: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/llm/pipeline.rs
git -C "D:/project/java/source/memo-ai" commit -m "fix: pipeline stage 3/5 JSON parse failure now falls back gracefully"
```

---

## Task 2: Whisper 模型目录可配置

**Files:**
- Modify: `src-tauri/src/commands.rs`（AppConfig 新增 whisper_model_dir）
- Modify: `src/types/index.ts`（AppSettings 新增字段）
- Modify: `src/pages/Settings.tsx`（新增 Input）

**注意：** 如果已执行 01-mvp-completion Task 2，`AppConfig` 已有 `whisper_cli_path`，本 Task 在此基础上新增 `whisper_model_dir`。

**Step 1: 在 `AppConfig` 中新增 `whisper_model_dir` 字段**

```rust
pub struct AppConfig {
    pub llm_provider: LlmProviderConfig,
    pub whisper_model: String,
    pub language: String,
    pub whisper_cli_path: String,
    pub whisper_model_dir: String,   // 新增
}
```

`impl Default for AppConfig` 中添加：

```rust
whisper_model_dir: "models".into(),
```

**Step 2: 更新 `transcribe_audio` 使用 `whisper_model_dir`**

将：

```rust
let model_path = format!("models/ggml-{}.bin", cfg.whisper_model);
```

改为：

```rust
let model_path = format!("{}/ggml-{}.bin", cfg.whisper_model_dir, cfg.whisper_model);
```

**Step 3: 更新前端类型**

在 `src/types/index.ts` 的 `AppSettings` 接口中添加：

```typescript
whisper_model_dir: string;
```

**Step 4: 更新 Settings UI**

在 `src/pages/Settings.tsx` 的 Whisper CLI 路径 Input 之后添加：

```tsx
<div className="space-y-1.5">
  <label className="text-sm font-medium text-foreground">
    模型文件目录
  </label>
  <Input
    value={local.whisper_model_dir}
    onChange={(e) =>
      setLocal({ ...local, whisper_model_dir: e.target.value })
    }
    placeholder="models"
  />
  <p className="text-[11px] text-muted-foreground">
    存放 ggml-*.bin 模型文件的目录路径
  </p>
</div>
```

**Step 5: 验证**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|Finished)"
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -20
```

Expected: 均无 error

**Step 6: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/commands.rs src/types/index.ts src/pages/Settings.tsx
git -C "D:/project/java/source/memo-ai" commit -m "feat: make whisper model directory configurable via Settings"
```

---

## Task 3: 前端 ErrorBoundary

**Files:**
- Create: `src/components/ErrorBoundary.tsx`
- Modify: `src/App.tsx`

**Step 1: 创建 `src/components/ErrorBoundary.tsx`**

```tsx
import { Component, ErrorInfo, ReactNode } from "react";

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-full flex-col items-center justify-center gap-4 p-8 text-center">
          <div className="text-4xl">⚠️</div>
          <div>
            <p className="text-base font-semibold text-foreground">页面渲染出错</p>
            <p className="mt-1 text-sm text-muted-foreground">
              {this.state.error?.message ?? "未知错误"}
            </p>
          </div>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            className="rounded-lg bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
          >
            重试
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
```

**Step 2: 在 `App.tsx` 中包裹 main 区域**

在 `App.tsx` 中添加 import：

```typescript
import { ErrorBoundary } from "./components/ErrorBoundary";
```

将 `<main className="flex-1 overflow-auto">` 的内容包裹：

```tsx
<main className="flex-1 overflow-auto">
  <ErrorBoundary>
    <Routes>
      <Route path="/" element={<Home />} />
      <Route path="/meeting/:id" element={<Meeting />} />
      <Route path="/settings" element={<Settings />} />
    </Routes>
  </ErrorBoundary>
</main>
```

**Step 3: 类型检查**

```bash
npm --prefix "D:/project/java/source/memo-ai" exec -- tsc --noEmit 2>&1 | head -20
```

Expected: 无 error

**Step 4: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src/components/ErrorBoundary.tsx src/App.tsx
git -C "D:/project/java/source/memo-ai" commit -m "feat: add ErrorBoundary to main content area"
```

---

## Task 4: 数据库迁移版本管理

**Files:**
- Modify: `src-tauri/src/db/connection.rs`
- Modify: `src-tauri/src/db/migrations.rs`

**背景：** 当前 `migrations.rs` 是空文件，`connection.rs` 直接执行 `SCHEMA`（CREATE TABLE IF NOT EXISTS）。引入 `PRAGMA user_version` 管理版本，为未来 schema 变更奠定基础。

**注意：** 如果已执行 01-mvp-completion Task 4，`connection.rs` 已有 auto_titled 的 inline migration 逻辑。本 Task 将其纳入版本化系统中。

**Step 1: 重写 `migrations.rs`**

```rust
use rusqlite::Connection;
use crate::error::AppResult;

/// Current schema version. Increment when adding migrations.
const CURRENT_VERSION: u32 = 2;

pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    let version: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(0);

    log::info!("DB schema version: {} -> target: {}", version, CURRENT_VERSION);

    if version < 1 {
        migrate_v1(conn)?;
    }
    if version < 2 {
        migrate_v2(conn)?;
    }

    // Update user_version
    conn.execute_batch(&format!("PRAGMA user_version = {}", CURRENT_VERSION))?;
    Ok(())
}

/// v1: initial schema (already created via init.sql, mark as migrated)
fn migrate_v1(_conn: &Connection) -> AppResult<()> {
    log::info!("DB migration: v1 (baseline, no-op)");
    Ok(())
}

/// v2: add auto_titled column to meetings
fn migrate_v2(conn: &Connection) -> AppResult<()> {
    log::info!("DB migration: v2 - add auto_titled to meetings");
    let has_col: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='auto_titled'",
        [],
        |row| row.get(0),
    ).unwrap_or(0);
    if has_col == 0 {
        conn.execute_batch(
            "ALTER TABLE meetings ADD COLUMN auto_titled INTEGER NOT NULL DEFAULT 0;"
        )?;
    }
    Ok(())
}
```

**Step 2: 更新 `connection.rs`，调用 `run_migrations`**

将 `init_db` 改为：

```rust
use rusqlite::Connection;
use std::path::Path;
use crate::error::AppResult;
use crate::db::migrations::run_migrations;

const SCHEMA: &str = include_str!("../../../schema/init.sql");

pub fn init_db(db_path: &Path) -> AppResult<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    // Create base tables if they don't exist
    conn.execute_batch(SCHEMA)?;
    // Apply versioned migrations
    run_migrations(&conn)?;
    Ok(conn)
}
```

**Step 3: 确保 `db/mod.rs` 导出 migrations**

读取 `src-tauri/src/db/mod.rs`，确认包含：

```rust
pub mod migrations;
```

若无则添加。

**Step 4: 验证编译**

```bash
cargo check --manifest-path "D:/project/java/source/memo-ai/src-tauri/Cargo.toml" 2>&1 | grep -E "(error|Finished)"
```

Expected: `Finished` 无 error

**Step 5: Commit**

```bash
git -C "D:/project/java/source/memo-ai" add src-tauri/src/db/migrations.rs src-tauri/src/db/connection.rs src-tauri/src/db/mod.rs
git -C "D:/project/java/source/memo-ai" commit -m "feat: versioned DB migrations with PRAGMA user_version"
```

---

## 完成标准

- [ ] Pipeline Stage 3 / Stage 5 JSON 解析失败时，Stage 4 / Stage 6 / Stage 7 正常继续
- [ ] 打包后的应用可通过 Settings 配置 Whisper 模型目录
- [ ] 前端渲染异常不白屏，ErrorBoundary 显示重试按钮
- [ ] 新数据库 `PRAGMA user_version` 返回 `2`
- [ ] `cargo check` 和 `tsc --noEmit` 均无 error
