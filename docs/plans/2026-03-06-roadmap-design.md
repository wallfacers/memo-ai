# MVP + 技术债 + v1.0 先行特性 设计文档

**日期：** 2026-03-06
**范围：** MVP 补完 / 技术债清理 / v1.0 先行特性，输出 3 份执行计划至 docs/exec-plans/active/

---

## 背景

当前项目已完成：前端 Tailwind 迁移、数据库 CRUD、音频录制骨架、LLM Pipeline 6 阶段结构。
尚未完成的 MVP 完成标准：Whisper 真实集成、设置持久化、前端 invoke 合规。

---

## 计划结构（方案 3：按优先层级）

```
docs/exec-plans/active/
  01-mvp-completion.md
  02-tech-debt.md
  03-v1-early.md
```

---

## 计划 01 — MVP 补完

### 功能 1：Whisper CLI 集成

- 不使用 whisper-rs（构建依赖重），改为调用 whisper.cpp 编译的 `whisper-cli` 可执行文件
- `WhisperAsr` 接收完整可执行路径（来自 Settings）
- 输出格式：`--output_format json`，解析后映射到 `TranscriptSegment`
- 找不到可执行文件时返回明确错误，不再 silent stub

### 功能 2：设置持久化

- `save_settings` 将 `AppConfig` 序列化为 JSON，写入 `app_data_dir/settings.json`
- `lib.rs` 启动时读取该文件，不存在则用 `AppConfig::default()`
- Settings 页增加 `whisper_cli_path` 输入项（shadcn Input）

### 功能 3：useTauriCommands 合规

- 将 `Sidebar.tsx`、`Meeting.tsx`、`Settings.tsx` 中的直接 `invoke()` 调用
  迁移到 `src/hooks/useTauriCommands.ts` 封装函数
- 组件通过 hook 调用，不直接依赖 `@tauri-apps/api/core`

### 功能 4：AI 自动生成会议标题

- `meetings` 表新增 `auto_titled INTEGER DEFAULT 0` 字段
- Sidebar `createMeeting`：无用户输入时传 `auto_titled: true`
- `run_pipeline` 完成 Stage 4（summary）后，若 `auto_titled = 1`，追加 Stage 7 LLM 调用
- 新增 `prompts/07_title.txt`：基于摘要生成 10 字内标题
- `PipelineResult` 增加 `generated_title: Option<String>`，前端收到后更新标题

---

## 计划 02 — 技术债

### 债务 1：Pipeline JSON 解析降级（Medium）

- Stage 3 / Stage 5 JSON 解析失败时，返回空结构体（不中断 Pipeline）
- 失败信息写入 `log::warn!`
- `PipelineOutput` 字段语义不变，只是可能为空值

### 债务 2：Whisper 路径可配置（Medium）

- `AppConfig` 增加 `whisper_model_dir: String` 字段（默认 `"models"`）
- Settings UI 增加对应 Input 表单项
- `transcribe_audio` 命令使用 `{model_dir}/ggml-{model}.bin` 拼接路径

### 债务 3：前端 ErrorBoundary（Low）

- 新建 `src/components/ErrorBoundary.tsx`（Class 组件）
- 捕获子组件渲染错误，显示友好提示 + 重试按钮
- 包裹 `App.tsx` 中的 `<main>` 区域

### 债务 4：数据库迁移版本管理（Low）

- `migrations.rs` 使用 `PRAGMA user_version` 管理 schema 版本
- 当前 schema 标记为 v1
- 提供 `run_migrations(conn)` 函数，按版本号顺序执行迁移

---

## 计划 03 — v1.0 先行特性

### 特性 1：报告导出 Markdown

- 后端：新增 `export_report(meeting_id, path)` Tauri 命令
  - 从 DB 读取 title / summary / action_items / report
  - 拼装 Markdown 并写入指定文件路径
- 前端：Meeting 页"报告"Tab 增加「导出 .md」按钮
  - 使用 `@tauri-apps/plugin-dialog` `save()` 弹出文件保存对话框
  - 选择路径后 invoke `export_report`
- Markdown 格式：
  ```
  # {title}
  **日期：** {start_time}
  ## 会议总结
  {summary}
  ## 行动项
  - [ ] {task}（{owner} / {deadline}）
  ## 完整报告
  {report}
  ```

### 特性 2：会议历史搜索

- 后端：新增 `search_meetings(query)` Tauri 命令
  - `WHERE title LIKE '%{q}%' OR summary LIKE '%{q}%' ORDER BY start_time DESC`
- 前端：Sidebar 新建会议区域上方增加搜索 Input（Search icon）
  - 输入时 debounce 300ms 调用搜索命令
  - 结果替换会议列表；清空时恢复全量列表
  - 空结果显示"无匹配会议"

---

## 约束

- 不在 MVP 中引入 whisper-rs 原生绑定
- macOS 支持不在本批计划内
- ACP 集成不在本批计划内
- 每个执行计划独立可执行，02 和 03 依赖 01 完成
