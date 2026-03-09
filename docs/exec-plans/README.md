# 执行计划索引

本目录存放 memo-ai 的所有开发执行计划，按状态分为两个子目录。

## 目录结构

```
exec-plans/
├── active/       # 进行中的计划
├── completed/    # 已完成的计划
└── tech-debt-tracker.md   # 技术债务追踪
```

## 进行中

| 计划 | 目标 |
|------|------|
| [09-unified-stage-progress-style](./active/09-unified-stage-progress-style.md) | 提取 StageProgressList 共享组件，统一进度条视觉风格 |

## 已完成

| 计划 | 目标 | 完成时间 |
|------|------|---------|
| [08-pipeline-retry](./completed/08-pipeline-retry.md) | Pipeline 阶段性重试 | 2026-03-08 |
| [07-summary-streaming](./completed/07-summary-streaming.md) | Summary 重新生成流式展示 | 2026-03-07 |
| [06-summary-tab-actions](./completed/06-summary-tab-actions.md) | Summary Tab 三操作按钮 | 2026-03-07 |
| [05-funasr-realtime-asr](./completed/05-funasr-realtime-asr.md) | FunASR 实时录音转写集成 | 2026-03-07 |
| [04-asr-pluggable-llm-test](./completed/04-asr-pluggable-llm-test.md) | ASR 可插拔 trait、LLM 连接测试 | 2026-03-06 |
| [03-v1-early](./completed/03-v1-early.md) | 报告导出、会议历史搜索 | 2026-03-06 |
| [02-tech-debt](./completed/02-tech-debt.md) | 技术债务清理 | 2026-03-06 |
| [01-mvp-completion](./completed/01-mvp-completion.md) | MVP 完整流程 | 2026-03-06 |

## 约定

- 每个计划文件包含 Goal、Architecture、Tech Stack 和逐步 Task
- 执行时使用 `superpowers:executing-plans` 技能逐 Task 实现
- 计划完成后从 `active/` 移至 `completed/`，并更新两个子目录的 README
