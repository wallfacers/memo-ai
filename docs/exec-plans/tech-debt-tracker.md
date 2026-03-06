# 技术债务追踪

记录已知的技术债务，按优先级排序。每个条目应说明问题、影响和解决方向。

## 格式

| 字段 | 说明 |
|------|------|
| 优先级 | High / Medium / Low |
| 位置 | 文件路径或模块名 |
| 描述 | 问题描述 |
| 影响 | 不解决会怎样 |
| 解决方向 | 建议的修复方案 |

---

## 当前技术债务

### [Medium] LLM Pipeline 错误处理不完整

- **位置**：`src-tauri/src/llm/pipeline.rs`
- **描述**：各 Stage 的 JSON 解析失败时没有统一的降级策略
- **影响**：某个阶段解析失败可能导致整个 Pipeline 中断
- **解决方向**：为每个 Stage 添加 fallback 逻辑，解析失败时存储原始文本

### [Medium] Whisper 模型路径硬编码

- **位置**：`src-tauri/src/asr/whisper.rs`
- **描述**：模型文件路径可能硬编码或缺乏灵活配置
- **影响**：用户无法自定义模型路径
- **解决方向**：通过 Settings 配置模型路径，存入 settingsStore

### [Low] 前端缺少全局错误边界

- **位置**：`src/App.tsx`
- **描述**：React 组件树没有 ErrorBoundary，未捕获的渲染错误会导致白屏
- **影响**：用户体验差
- **解决方向**：在 App.tsx 根组件添加 ErrorBoundary

### [Low] 数据库迁移版本管理

- **位置**：`src-tauri/src/db/migrations.rs`
- **描述**：当前 schema 变更可能没有版本化迁移机制
- **影响**：升级应用版本时可能导致数据库不兼容
- **解决方向**：引入简单的版本号迁移系统（如 user_version pragma）
