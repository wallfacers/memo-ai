# Pipeline 阶段性重试 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 当 run_pipeline 在某个 LLM 阶段失败时，在 PipelineProgress 进度条失败行旁显示重试按钮，支持从失败阶段续跑。

**Architecture:** 每个阶段完成后立即写 DB（新增 clean_transcript / organized_transcript 字段），Rust 发送 pipeline_stage_failed 事件通知前端失败阶段和原因，前端展示具体错误信息和重试按钮，点击后调用新命令 retry_pipeline_from_stage 从失败阶段续跑。

**Tech Stack:** Rust / rusqlite / Tauri 2.x events / React / Zustand / TypeScript

**Design doc:** `docs/plans/2026-03-08-pipeline-retry-design.md`

---

## Task 1: DB Migration v3 — 新增中间结果字段

**Files:**
- Modify: `src-tauri/src/db/migrations.rs`

在 meetings 表新增两个字段存储 pipeline 中间产物，并把 `CURRENT_VERSION` 从 2 改为 3。

**Step 1: 添加 migrate_v3 函数，并更新 run_migrations**

在 `migrations.rs` 中：
1. 将 `CURRENT_VERSION` 从 `2` 改为 `3`
2. 在 `run_migrations` 中添加 `if version < 3 { migrate_v3(conn)?; }`
3. 添加函数：

```rust
/// v3: add pipeline intermediate columns to meetings
fn migrate_v3(conn: &Connection) -> AppResult<()> {
    log::info!("DB migration: v3 - add clean_transcript / organized_transcript to meetings");
    for col in &["clean_transcript", "organized_transcript"] {
        let has_col: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM pragma_table_info('meetings') WHERE name='{}'", col),
            [],
            |row| row.get(0),
        ).unwrap_or(0);
        if has_col == 0 {
            conn.execute_batch(
                &format!("ALTER TABLE meetings ADD COLUMN {} TEXT;", col)
            )?;
        }
    }
    Ok(())
}
```

**Step 2: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: `Finished` 无 error。

**Step 3: Commit**

```bash
git add src-tauri/src/db/migrations.rs
git commit -m "feat(db): migration v3 — add clean_transcript/organized_transcript to meetings"
```

---

## Task 2: DB Model 函数 — 读写中间字段 + 清除旧数据

**Files:**
- Modify: `src-tauri/src/db/models.rs`

**Step 1: 新增写入函数（写 clean_transcript）**

在 `models.rs` 末尾添加：

```rust
// ─── Pipeline Intermediate State ──────────────────────────────────────────────

pub fn update_clean_transcript(conn: &Connection, id: i64, text: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET clean_transcript = ?1, updated_at = ?2 WHERE id = ?3",
        params![text, now, id],
    )?;
    Ok(())
}

pub fn update_organized_transcript(conn: &Connection, id: i64, text: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE meetings SET organized_transcript = ?1, updated_at = ?2 WHERE id = ?3",
        params![text, now, id],
    )?;
    Ok(())
}

pub fn get_pipeline_intermediates(
    conn: &Connection,
    id: i64,
) -> AppResult<(Option<String>, Option<String>)> {
    let result = conn.query_row(
        "SELECT clean_transcript, organized_transcript FROM meetings WHERE id = ?1",
        params![id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(result)
}

pub fn delete_action_items_for_meeting(conn: &Connection, meeting_id: i64) -> AppResult<()> {
    conn.execute(
        "DELETE FROM action_items WHERE meeting_id = ?1",
        params![meeting_id],
    )?;
    Ok(())
}

/// Clear pipeline results from a given stage onward.
/// from_stage: 1=clean, 2=organized, 3=structure, 4=summary, 5=actions, 6=report
pub fn clear_pipeline_from_stage(conn: &Connection, meeting_id: i64, from_stage: u32) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    if from_stage <= 1 {
        conn.execute(
            "UPDATE meetings SET clean_transcript = NULL, organized_transcript = NULL,
             summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
        conn.execute("DELETE FROM meeting_structures WHERE meeting_id = ?1", params![meeting_id])?;
    } else if from_stage <= 2 {
        conn.execute(
            "UPDATE meetings SET organized_transcript = NULL, summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
        conn.execute("DELETE FROM meeting_structures WHERE meeting_id = ?1", params![meeting_id])?;
    } else if from_stage <= 3 {
        conn.execute(
            "UPDATE meetings SET summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
        conn.execute("DELETE FROM meeting_structures WHERE meeting_id = ?1", params![meeting_id])?;
    } else if from_stage <= 4 {
        conn.execute(
            "UPDATE meetings SET summary = NULL, report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
    } else if from_stage <= 5 {
        conn.execute(
            "UPDATE meetings SET report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
        conn.execute("DELETE FROM action_items WHERE meeting_id = ?1", params![meeting_id])?;
    } else {
        // from_stage == 6: only clear report
        conn.execute(
            "UPDATE meetings SET report = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, meeting_id],
        )?;
    }
    Ok(())
}
```

**Step 2: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```
Expected: `Finished` 无 error。

**Step 3: Commit**

```bash
git add src-tauri/src/db/models.rs
git commit -m "feat(db): add pipeline intermediate read/write and clear_pipeline_from_stage"
```

---

## Task 3: 新增 pipeline_stage_failed 事件结构

**Files:**
- Modify: `src-tauri/src/commands.rs`

**Step 1: 在 commands.rs 中找到 `PipelineStageDoneEvent` 结构，紧接其后添加**

```rust
#[derive(Serialize, Clone)]
pub struct PipelineStageFailed {
    pub stage: u32,
    pub error: String,
}
```

**Step 2: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | tail -5
```

**Step 3: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(commands): add PipelineStageFailed event struct"
```

---

## Task 4: 改造 run_pipeline — 逐阶段写 DB + emit 失败事件

**Files:**
- Modify: `src-tauri/src/commands.rs`（`run_pipeline` 函数）

**Step 1: 理解当前结构**

当前 `run_pipeline`（约 507-630 行）：
- 读 transcripts → `std::thread::spawn` 跑完整 pipeline → 一次性写 DB

改造目标：在线程内每个阶段完成后立即写 DB，失败时 emit `pipeline_stage_failed`。

**Step 2: 替换线程内的执行逻辑**

找到 `std::thread::spawn(move || {` 块，将其替换为逐阶段执行版本。关键点：
- `db` 是 `State<'_, DbState>`，不能 move 进线程，需要在 spawn 前提取所需数据（meeting_id 已有）
- 线程内用 `oneshot::channel` 传回结果（已有模式）
- 但逐阶段写 DB 需要在线程内访问 DB，需要把 `Arc<Mutex<Connection>>` 传进去

改造步骤：

在 `std::thread::spawn` 前，提取 db 的 Arc 引用：
```rust
let db_arc = (*db).0.clone(); // Arc<Mutex<Connection>>
```

将线程体替换为：

```rust
std::thread::spawn(move || {
    let client = llm_config.build_client();
    let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);

    macro_rules! write_db {
        ($op:expr) => {{
            let conn = db_arc.lock().unwrap();
            if let Err(e) = $op(&conn) {
                let _ = app_for_cb.emit("pipeline_stage_failed", PipelineStageFailed {
                    stage: 0,
                    error: format!("DB write failed: {}", e),
                });
                let _ = tx.send(Err(crate::error::AppError::Db(e.to_string())));
                return;
            }
        }};
    }

    macro_rules! run_stage {
        ($stage_num:expr, $name:expr, $result:expr, $summary:expr) => {
            match $result {
                Ok(val) => {
                    app_for_cb.emit("pipeline_stage_done", PipelineStageDoneEvent {
                        stage: $stage_num,
                        name: $name.to_string(),
                        summary: $summary(&val),
                    }).ok();
                    val
                }
                Err(e) => {
                    app_for_cb.emit("pipeline_stage_failed", PipelineStageFailed {
                        stage: $stage_num,
                        error: e.to_string(),
                    }).ok();
                    let _ = tx.send(Err(e));
                    return;
                }
            }
        };
    }

    // Stage 1
    let clean = run_stage!(1, "文本清洗",
        pipeline.stage1_clean(&transcript_text),
        |v: &String| format!("完成（共 {} 字）", v.len()));
    write_db!(|conn| models::update_clean_transcript(conn, meeting_id, &clean));

    // Stage 2
    let organized = run_stage!(2, "说话人整理",
        pipeline.stage2_speaker(&clean),
        |v: &String| v.chars().take(50).collect::<String>());
    write_db!(|conn| models::update_organized_transcript(conn, meeting_id, &organized));

    // Stage 3 (infallible, no ?)
    let structure = pipeline.stage3_structure(&organized);
    let s3_summary = format!(
        "主题：{} · 参会 {} 人 · {} 项决策",
        structure.topic.as_deref().unwrap_or("未知"),
        structure.participants.len(),
        structure.decisions.len(),
    );
    app_for_cb.emit("pipeline_stage_done", PipelineStageDoneEvent {
        stage: 3,
        name: "结构化提取".to_string(),
        summary: s3_summary,
    }).ok();
    {
        let conn = db_arc.lock().unwrap();
        let _ = models::upsert_meeting_structure(
            &conn, meeting_id,
            structure.topic.as_deref(),
            &structure.participants,
            &structure.key_points,
            &structure.decisions,
            &structure.risks,
        );
    }

    // Stage 4
    let summary = run_stage!(4, "会议总结",
        pipeline.stage4_summary(&organized),
        |v: &String| v.chars().take(100).collect::<String>());
    {
        let conn = db_arc.lock().unwrap();
        let _ = models::update_meeting_summary(&conn, meeting_id, &summary);
    }

    // Stage 5 (infallible)
    let action_items = pipeline.stage5_actions(&organized);
    app_for_cb.emit("pipeline_stage_done", PipelineStageDoneEvent {
        stage: 5,
        name: "行动项提取".to_string(),
        summary: format!("共 {} 项行动", action_items.len()),
    }).ok();
    {
        let conn = db_arc.lock().unwrap();
        for item in &action_items {
            let _ = models::insert_action_item(
                &conn, meeting_id,
                &item.task, item.owner.as_deref(), item.deadline.as_deref(),
            );
        }
    }

    // Stage 6
    let actions_json = match serde_json::to_string(&action_items) {
        Ok(j) => j,
        Err(e) => {
            app_for_cb.emit("pipeline_stage_failed", PipelineStageFailed {
                stage: 6,
                error: e.to_string(),
            }).ok();
            let _ = tx.send(Err(crate::error::AppError::Llm(e.to_string())));
            return;
        }
    };
    let report = run_stage!(6, "报告生成",
        pipeline.stage6_report(&summary, &actions_json),
        |_: &String| "报告已生成，点击查看".to_string());
    {
        let conn = db_arc.lock().unwrap();
        let _ = models::update_meeting_summary_report(&conn, meeting_id, &summary, &report);
    }

    // Stage 7 (optional title, same as before)
    let generated_title = if auto_titled {
        match pipeline.stage7_title(&summary) {
            Ok(t) => {
                let conn = db_arc.lock().unwrap();
                let _ = models::update_meeting_title(&conn, meeting_id, &t);
                Some(t)
            }
            Err(e) => {
                log::warn!("Stage 7 title generation failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    let _ = tx.send(Ok(PipelineOutput {
        clean_transcript: clean,
        structure,
        summary,
        action_items,
        report,
        generated_title,
    }));
});
```

**注意**：原来末尾的 DB 写入块（`let conn = (*db).0.lock()...`）要**删除**，因为已在线程内逐阶段写入了。

**Step 3: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | tail -10
```
Expected: `Finished` 无 error。若有 `db_arc` 类型错误，确认 `(*db).0` 是 `Arc<Mutex<Connection>>`（见 `db/mod.rs`）。

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(pipeline): emit per-stage events and write DB incrementally"
```

---

## Task 5: 新增命令 retry_pipeline_from_stage

**Files:**
- Modify: `src-tauri/src/commands.rs`（末尾添加新命令）
- Modify: `src-tauri/src/lib.rs`（注册命令）

**Step 1: 在 commands.rs 末尾添加新命令**

```rust
// ─── Pipeline Retry ───────────────────────────────────────────────────────────

#[tauri::command]
pub async fn retry_pipeline_from_stage(
    meeting_id: i64,
    from_stage: u32,
    app_handle: tauri::AppHandle,
    db: State<'_, DbState>,
    config: State<'_, ConfigState>,
) -> Result<PipelineResult, String> {
    let cfg = (*config).0.lock().unwrap().clone();
    let llm_config = LlmConfig {
        provider: cfg.llm_provider.provider_type,
        base_url: cfg.llm_provider.base_url,
        model: cfg.llm_provider.model,
        api_key: cfg.llm_provider.api_key,
    };

    // prompts_dir（与 run_pipeline 相同逻辑）
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let prompts_dir = {
        let exe_adjacent = exe_dir.join("prompts");
        let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("prompts");
        if exe_adjacent.exists() { exe_adjacent }
        else if dev_path.exists() { dev_path }
        else { PathBuf::from("prompts") }
    };

    // 读取中间结果 + 决定实际起始阶段
    let (clean_opt, organized_opt, raw_transcript, auto_titled) = {
        let conn = (*db).0.lock().unwrap();
        let (c, o) = models::get_pipeline_intermediates(&conn, meeting_id)
            .map_err(|e| e.to_string())?;
        let segments = models::get_transcripts(&conn, meeting_id).map_err(|e| e.to_string())?;
        let raw = segments.iter().map(|s| {
            if let Some(ref speaker) = s.speaker {
                format!("{}：{}", speaker, s.text)
            } else {
                s.text.clone()
            }
        }).collect::<Vec<_>>().join("\n");
        let auto_t = models::get_meeting(&conn, meeting_id)
            .map(|m| m.auto_titled).unwrap_or(false);
        (c, o, raw, auto_t)
    };

    // 降级：若所需中间数据缺失，从更早阶段开始
    let actual_from_stage = if from_stage >= 3 && organized_opt.is_none() {
        if clean_opt.is_none() { 1 } else { 2 }
    } else if from_stage >= 2 && clean_opt.is_none() {
        1
    } else {
        from_stage
    };

    // 清除 from_stage 及之后的旧数据，防止重复
    {
        let conn = (*db).0.lock().unwrap();
        models::clear_pipeline_from_stage(&conn, meeting_id, actual_from_stage)
            .map_err(|e| e.to_string())?;
    }

    let db_arc = (*db).0.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();

    std::thread::spawn(move || {
        let client = llm_config.build_client();
        let pipeline = Pipeline::new(client.as_ref(), &prompts_dir);

        // 准备各阶段输入
        let mut clean: String = clean_opt.unwrap_or_default();
        let mut organized: String = organized_opt.unwrap_or_default();

        macro_rules! emit_failed {
            ($stage:expr, $err:expr) => {{
                app_handle.emit("pipeline_stage_failed", PipelineStageFailed {
                    stage: $stage,
                    error: $err.to_string(),
                }).ok();
                let _ = tx.send(Err(crate::error::AppError::Llm($err.to_string())));
                return;
            }};
        }

        macro_rules! emit_done {
            ($stage:expr, $name:expr, $summary:expr) => {
                app_handle.emit("pipeline_stage_done", PipelineStageDoneEvent {
                    stage: $stage,
                    name: $name.to_string(),
                    summary: $summary.to_string(),
                }).ok();
            };
        }

        if actual_from_stage <= 1 {
            if raw_transcript.is_empty() {
                emit_failed!(1, "No transcript available");
            }
            match pipeline.stage1_clean(&raw_transcript) {
                Ok(v) => {
                    clean = v;
                    emit_done!(1, "文本清洗", format!("完成（共 {} 字）", clean.len()));
                    let conn = db_arc.lock().unwrap();
                    let _ = models::update_clean_transcript(&conn, meeting_id, &clean);
                }
                Err(e) => emit_failed!(1, e),
            }
        }

        if actual_from_stage <= 2 {
            match pipeline.stage2_speaker(&clean) {
                Ok(v) => {
                    organized = v;
                    emit_done!(2, "说话人整理", organized.chars().take(50).collect::<String>());
                    let conn = db_arc.lock().unwrap();
                    let _ = models::update_organized_transcript(&conn, meeting_id, &organized);
                }
                Err(e) => emit_failed!(2, e),
            }
        }

        if actual_from_stage <= 3 {
            let structure = pipeline.stage3_structure(&organized);
            let s3_summary = format!(
                "主题：{} · 参会 {} 人 · {} 项决策",
                structure.topic.as_deref().unwrap_or("未知"),
                structure.participants.len(),
                structure.decisions.len(),
            );
            emit_done!(3, "结构化提取", s3_summary);
            let conn = db_arc.lock().unwrap();
            let _ = models::upsert_meeting_structure(
                &conn, meeting_id,
                structure.topic.as_deref(),
                &structure.participants,
                &structure.key_points,
                &structure.decisions,
                &structure.risks,
            );
        }

        // Stage 4: summary
        let summary = {
            // Read from DB if we're starting after stage 4
            if actual_from_stage > 4 {
                let conn = db_arc.lock().unwrap();
                models::get_meeting(&conn, meeting_id).ok()
                    .and_then(|m| m.summary)
                    .unwrap_or_default()
            } else {
                match pipeline.stage4_summary(&organized) {
                    Ok(v) => {
                        emit_done!(4, "会议总结", v.chars().take(100).collect::<String>());
                        let conn = db_arc.lock().unwrap();
                        let _ = models::update_meeting_summary(&conn, meeting_id, &v);
                        v
                    }
                    Err(e) => { emit_failed!(4, e); }
                }
            }
        };

        // Stage 5: action items
        let action_items = if actual_from_stage > 5 {
            // Read from DB
            let conn = db_arc.lock().unwrap();
            models::get_action_items(&conn, meeting_id)
                .unwrap_or_default()
                .into_iter()
                .map(|a| crate::llm::pipeline::ActionItemRaw {
                    task: a.task,
                    owner: a.owner,
                    deadline: a.deadline,
                })
                .collect::<Vec<_>>()
        } else {
            let items = pipeline.stage5_actions(&organized);
            emit_done!(5, "行动项提取", format!("共 {} 项行动", items.len()));
            let conn = db_arc.lock().unwrap();
            for item in &items {
                let _ = models::insert_action_item(
                    &conn, meeting_id,
                    &item.task, item.owner.as_deref(), item.deadline.as_deref(),
                );
            }
            items
        };

        // Stage 6
        let actions_json = match serde_json::to_string(&action_items) {
            Ok(j) => j,
            Err(e) => { emit_failed!(6, e); }
        };
        let report = match pipeline.stage6_report(&summary, &actions_json) {
            Ok(v) => {
                emit_done!(6, "报告生成", "报告已生成，点击查看");
                let conn = db_arc.lock().unwrap();
                let _ = models::update_meeting_summary_report(&conn, meeting_id, &summary, &v);
                v
            }
            Err(e) => { emit_failed!(6, e); }
        };

        // Stage 7 (optional)
        let generated_title = if auto_titled {
            match pipeline.stage7_title(&summary) {
                Ok(t) => {
                    let conn = db_arc.lock().unwrap();
                    let _ = models::update_meeting_title(&conn, meeting_id, &t);
                    Some(t)
                }
                Err(e) => { log::warn!("Stage 7 failed: {}", e); None }
            }
        } else {
            None
        };

        let _ = tx.send(Ok(PipelineOutput {
            clean_transcript: clean,
            structure: Default::default(), // already in DB
            summary,
            action_items,
            report,
            generated_title,
        }));
    });

    let output = rx.await
        .map_err(|_| "Pipeline retry thread panicked".to_string())?
        .map_err(|e| e.to_string())?;

    Ok(PipelineResult {
        clean_transcript: output.clean_transcript,
        summary: output.summary,
        report: output.report,
        generated_title: output.generated_title,
    })
}
```

**注意**：`ActionItemRaw` 和 `PipelineOutput` 需要 `pub`（检查 pipeline.rs 中的可见性）。若 `StructuredMeeting` 未实现 `Default`，为其派生 `Default` 或在 `pipeline.rs` 中添加。

**Step 2: 在 lib.rs 中注册新命令**

找到 `tauri::generate_handler![...]` 宏，在其中加入 `retry_pipeline_from_stage`。

**Step 3: 编译验证**

```bash
cd src-tauri && cargo check 2>&1 | tail -10
```
Expected: `Finished` 无 error。

**Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): add retry_pipeline_from_stage command"
```

---

## Task 6: 前端类型 + Store

**Files:**
- Modify: `src/types/index.ts`
- Modify: `src/store/meetingStore.ts`
- Modify: `src/hooks/useTauriCommands.ts`

**Step 1: 在 types/index.ts 中新增类型**

在 `PipelineStageDoneEvent` 后添加：

```typescript
export interface PipelineStageFailed {
  stage: number;
  error: string;
}
```

**Step 2: 在 useTauriCommands.ts 中新增 hook**

在末尾添加：

```typescript
export function useRetryPipelineFromStage() {
  return useCallback(
    (meetingId: number, fromStage: number) =>
      invoke<PipelineResult>("retry_pipeline_from_stage", { meetingId, fromStage }),
    []
  );
}
```

**Step 3: 在 meetingStore.ts 中新增状态**

在 `MeetingStore` interface 中添加：

```typescript
pipelineFailedStage: { stage: number; error: string } | null;
setPipelineFailedStage: (info: { stage: number; error: string } | null) => void;
```

在 `create()` 初始状态中添加：

```typescript
pipelineFailedStage: null,
```

在 actions 中添加：

```typescript
setPipelineFailedStage: (info) => set({ pipelineFailedStage: info }),
```

**Step 4: 编译验证**

```bash
npx tsc --noEmit 2>&1 | tail -10
```
Expected: 无 error。

**Step 5: Commit**

```bash
git add src/types/index.ts src/store/meetingStore.ts src/hooks/useTauriCommands.ts
git commit -m "feat(frontend): add PipelineStageFailed type, store field, and retry hook"
```

---

## Task 7: i18n 新增 key

**Files:**
- Modify: `src/i18n/locales/zh.ts`
- Modify: `src/i18n/locales/en.ts`

**Step 1: 在 zh.ts 的 `meeting.phase` 对象中添加**

```typescript
retryFromStage: "从此阶段重试",
```

**Step 2: 在 en.ts 的 `meeting.phase` 对象中添加**

```typescript
retryFromStage: "Retry from here",
```

**Step 3: Commit**

```bash
git add src/i18n/locales/zh.ts src/i18n/locales/en.ts
git commit -m "feat(i18n): add pipeline retry i18n keys"
```

---

## Task 8: PipelineProgress 组件改造

**Files:**
- Modify: `src/components/PipelineProgress.tsx`

**Step 1: 理解当前组件**

组件当前：监听 `pipeline-stage-done` 事件 → 显示 6 行阶段状态。无错误状态处理，无重试按钮。

**Step 2: 改造组件，接受 props 并显示错误+重试**

将组件改为接受 props：

```typescript
interface PipelineProgressProps {
  onRetryFromStage: (stage: number) => void;
}

export function PipelineProgress({ onRetryFromStage }: PipelineProgressProps) {
```

在 `useEffect` 中额外监听 `pipeline-stage-failed` 事件：

```typescript
const { pipelineStages, appendPipelineStage, recordingPhase, pipelineFailedStage, setPipelineFailedStage } = useMeetingStore();

useEffect(() => {
  const unlistenDone = listen<PipelineStageDoneEvent>("pipeline-stage-done", (event) => {
    appendPipelineStage(event.payload);
  });
  const unlistenFailed = listen<{ stage: number; error: string }>("pipeline-stage-failed", (event) => {
    setPipelineFailedStage(event.payload);
  });
  return () => {
    void unlistenDone.then((fn) => fn());
    void unlistenFailed.then((fn) => fn());
  };
}, [appendPipelineStage, setPipelineFailedStage]);
```

**Step 3: 每行阶段的渲染逻辑**

```typescript
const failedStageNum = pipelineFailedStage?.stage ?? null;

// 在 Array.from 循环中:
const isFailed = stageNum === failedStageNum;
const isActive =
  recordingPhase === "pipeline" &&
  !done &&
  !isFailed &&
  stageNum === (pipelineStages.length + 1);

return (
  <div key={stageNum} className="space-y-0.5">
    <div className="flex items-center gap-2 text-xs">
      {done ? (
        <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-600" />
      ) : isFailed ? (
        <XCircle className="h-3.5 w-3.5 shrink-0 text-destructive" />
      ) : isActive ? (
        <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-muted-foreground" />
      ) : (
        <span className="h-3.5 w-3.5 shrink-0 rounded-full border border-muted-foreground/30" />
      )}
      <span className={isFailed ? "text-destructive" : done ? "text-foreground" : "text-muted-foreground"}>
        {stageName}
      </span>
      {done && stageData && (
        <span className="ml-auto text-muted-foreground">{stageData.elapsed_ms}ms</span>
      )}
      {isFailed && (
        <button
          onClick={() => onRetryFromStage(stageNum)}
          className="ml-auto flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <RotateCcw className="h-3 w-3" />
          {t("meeting.phase.retryFromStage")}
        </button>
      )}
    </div>
    {isFailed && pipelineFailedStage?.error && (
      <p className="ml-5 text-[11px] text-destructive/70 leading-relaxed">
        {pipelineFailedStage.error}
      </p>
    )}
  </div>
);
```

在 import 中添加 `RotateCcw` 和 `XCircle`（来自 lucide-react），以及 `useTranslation`。

**Step 4: 编译验证**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

**Step 5: Commit**

```bash
git add src/components/PipelineProgress.tsx
git commit -m "feat(PipelineProgress): show error detail and retry button on stage failure"
```

---

## Task 9: Meeting 页面接线

**Files:**
- Modify: `src/pages/Meeting.tsx`

**Step 1: 导入新 hook 和 store 字段**

在 `useMeetingStore()` 解构中添加：
```typescript
pipelineFailedStage,
setPipelineFailedStage,
```

导入 `useRetryPipelineFromStage`。

**Step 2: 添加 handleRetryFromStage 函数**

```typescript
const retryPipelineFromStage = useRetryPipelineFromStage();

async function handleRetryFromStage(stage: number) {
  if (!meetingId) return;
  setPipelineFailedStage(null);
  clearPipelineStages();
  setRecordingPhase("pipeline");
  setCurrentMeetingStatus("processing");
  try {
    await retryPipelineFromStage(meetingId, stage);
    setRecordingPhase("done");
    await loadMeeting();
    await loadActionItems();
    const updatedMeetings = await listMeetings();
    setMeetings(updatedMeetings);
    setCurrentMeetingStatus("completed");
  } catch (e) {
    console.error("Pipeline retry failed:", e);
    // pipelineFailedStage will be set via the event listener in PipelineProgress
    setRecordingPhase("error");
    setCurrentMeetingStatus("error");
  }
}
```

**Step 3: 更新 PipelineProgress 显示条件和 props**

找到：
```typescript
{(recordingPhase === "pipeline" || recordingPhase === "done") && (
  <PipelineProgress />
)}
```

替换为：
```typescript
{(recordingPhase === "pipeline" ||
  recordingPhase === "done" ||
  recordingPhase === "error") && (
  <PipelineProgress onRetryFromStage={handleRetryFromStage} />
)}
```

**Step 4: 编译验证**

```bash
npx tsc --noEmit 2>&1 | tail -10
```
Expected: 无 error。

**Step 5: Commit**

```bash
git add src/pages/Meeting.tsx
git commit -m "feat(Meeting): wire retry handler and show PipelineProgress on error"
```

---

## Task 10: 手动验证

**测试步骤：**

1. 运行 `npm run tauri:dev`
2. 创建一个新会议，录音并停止
3. **正常路径**：pipeline 正常完成 → 进度条所有阶段显示绿色 ✅
4. **模拟失败路径**：
   - 在 Settings 中将 LLM model 改为一个不存在的模型名（如 `nonexistent-model`）
   - 录音并停止 → 等待 pipeline 在某个阶段报错
   - 确认进度条显示：失败阶段红色 ❌ + 错误信息文字 + "从此阶段重试" 按钮
   - 将 model 改回正确的名称（**不要**点保存 LLM，只在 Settings 改完就行）
   - 点击「从此阶段重试」→ 确认从该阶段续跑，最终完成

**验收标准：**

- [ ] 正常 pipeline 执行不受影响
- [ ] 失败时进度条保持可见（不消失）
- [ ] 失败阶段显示 ❌ 图标 + 具体错误信息
- [ ] 点击重试从正确阶段续跑（前面已完成阶段不重跑）
- [ ] 续跑成功后状态回到 `done`

---

## 文件变更汇总

| 文件 | Task |
|------|------|
| `src-tauri/src/db/migrations.rs` | Task 1 |
| `src-tauri/src/db/models.rs` | Task 2 |
| `src-tauri/src/commands.rs` | Task 3, 4, 5 |
| `src-tauri/src/lib.rs` | Task 5 |
| `src/types/index.ts` | Task 6 |
| `src/hooks/useTauriCommands.ts` | Task 6 |
| `src/store/meetingStore.ts` | Task 6 |
| `src/i18n/locales/zh.ts` | Task 7 |
| `src/i18n/locales/en.ts` | Task 7 |
| `src/components/PipelineProgress.tsx` | Task 8 |
| `src/pages/Meeting.tsx` | Task 9 |
