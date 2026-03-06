# 开发计划

详细的执行计划见 [exec-plans/](./exec-plans/)。本文件记录高层次路线图。

## 当前阶段：MVP

**目标**：完成核心录音 → 转写 → AI 处理 → 展示的完整闭环。

### MVP 完成标准

- [x] Windows 上可以录制麦克风音频
- [x] Whisper.cpp 成功完成本地转写（代码层面完成，需配置 whisper-cli 可执行文件）
- [x] 6 阶段 LLM Pipeline 顺序执行完成
- [x] 前端展示转写文本、行动项、会议总结
- [x] 支持 Ollama 和 OpenAI 两种后端
- [x] 会议记录持久化到 SQLite

## 下一步：v1.0

- [x] 实时转写字幕（FunASR WebSocket 2pass，temp/final 双态）
- 说话人识别
- 会议历史列表搜索
- 报告导出（Markdown / PDF）
- macOS 支持
- **ACP 智能体集成**：Stage 3~6 完成后，用户可与任意 ACP 兼容智能体多轮对话精炼输出（详见 [docs/design-docs/acp-integration.md](./design-docs/acp-integration.md)）

## 未来规划：v2.0+

- 会议知识库（Vector DB + 语义搜索）
- AI 会议问答（Ask Meeting）
- 日历自动检测（Google Calendar / Outlook）
- Jira / Notion / Slack 集成

## 主动推迟的功能

以下功能已决定不在 MVP 做，避免范围蔓延：

| 功能 | 推迟原因 |
|------|---------|
| 实时字幕 | 增加复杂度，核心路径先跑通 |
| 说话人识别 | 依赖 pyannote 等 Python 生态，集成成本高 |
| 向量数据库 | MVP 不需要搜索能力 |
| 云同步 | 本地优先，云端是增值功能 |
