# CLAUDE.md — memo-ai 智能体上下文工程文件

本文件是 Claude Code 的核心上下文入口。每次开始任务前请先阅读本文件，再根据任务类型查阅相关子文档。

## 项目概述

**memo-ai** 是一款本地优先的 AI 会议助手桌面应用。

核心价值：会议录音 → 自动转写 → AI 理解 → 行动项 → 会议报告，**全程本地处理，无需云端**。

产品定位：**AI Meeting Copilot**

## 技术栈

| 层级 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.x |
| 前端 | React 18 + TypeScript + Vite |
| 状态管理 | Zustand |
| 路由 | React Router v6 |
| 后端 | Rust |
| 数据库 | SQLite（via rusqlite） |
| 音频采集 | WASAPI（Windows）/ CoreAudio（macOS） |
| 语音识别 | Whisper.cpp |
| LLM | Ollama（本地）/ OpenAI API（云端） |

## 目录结构

```
memo-ai/
├── CLAUDE.md              # 本文件（智能体上下文入口）
├── ARCHITECTURE.md        # 系统架构详述
├── src/                   # React 前端
│   ├── components/        # UI 组件
│   ├── pages/             # 页面（Home, Meeting, Settings）
│   ├── store/             # Zustand stores
│   ├── hooks/             # 自定义 hooks
│   ├── utils/             # 工具函数
│   └── types/             # TypeScript 类型定义
├── src-tauri/             # Rust 后端
│   └── src/
│       ├── audio/         # 音频采集与编码
│       ├── asr/           # 语音识别（Whisper/FunASR/Aliyun）
│       ├── process/       # FunASR 进程生命周期管理
│       ├── llm/           # LLM 客户端（Ollama/OpenAI）
│       ├── db/            # 数据库（SQLite）
│       ├── commands.rs    # Tauri 命令（前后端桥接）
│       └── lib.rs         # 应用入口
├── prompts/               # AI Prompt 模板（6 个阶段）
├── schema/                # SQLite schema
├── models/                # 本地模型文件（.gitkeep）
└── docs/                  # 项目文档（见下方）
```

## 文档导航

| 文档 | 用途 |
|------|------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | 系统架构、模块说明、数据流 |
| [docs/DESIGN.md](./docs/DESIGN.md) | 设计原则与 UI/UX 规范 |
| [docs/FRONTEND.md](./docs/FRONTEND.md) | 前端开发规范与组件说明 |
| [docs/PLANS.md](./docs/PLANS.md) | 当前开发计划与路线图 |
| [docs/PRODUCT_SENSE.md](./docs/PRODUCT_SENSE.md) | 产品理念与用户价值 |
| [docs/QUALITY_SCORE.md](./docs/QUALITY_SCORE.md) | 代码质量标准 |
| [docs/RELIABILITY.md](./docs/RELIABILITY.md) | 可靠性目标与错误处理 |
| [docs/SECURITY.md](./docs/SECURITY.md) | 安全策略 |
| [docs/design-docs/](./docs/design-docs/) | 技术设计文档 |
| [docs/product-specs/](./docs/product-specs/) | 产品功能规格 |
| [docs/exec-plans/](./docs/exec-plans/) | 执行计划（进行中/已完成） |
| [docs/generated/db-schema.md](./docs/generated/db-schema.md) | 数据库 schema（自动生成） |
| [docs/references/](./docs/references/) | 技术参考资料 |

## 开发命令

```bash
# 前端开发
npm run dev

# Tauri 开发模式（前后端联调）
npm run tauri:dev

# 构建
npm run tauri:build

# 类型检查
npx tsc --noEmit
```

## 前后端通信约定

前端通过 Tauri `invoke()` 调用 Rust 命令，所有命令定义在 `src-tauri/src/commands.rs`。

```typescript
// 示例
import { invoke } from '@tauri-apps/api/core'
await invoke('start_recording', { meetingId: 1 })
```

## AI Pipeline 简述

6 阶段顺序处理（详见 [docs/design-docs/ai-pipeline.md](./docs/design-docs/ai-pipeline.md)）：

```
ASR → 文本清洗 → 说话人整理 → 结构化提取 → 会议总结 → 行动项提取 → 报告生成
```

Prompt 模板位于 `prompts/` 目录（`01_clean.txt` ~ `06_report.txt`）。

## 关键约定

- 数据库操作全部在 Rust 层完成，前端不直接访问 SQLite
- 所有 LLM 调用走 `src-tauri/src/llm/pipeline.rs` 统一管理
- 音频文件路径存储在 `meetings.audio_path` 字段
- 时间戳使用 ISO 8601 字符串存储
- JSON 字段（participants, key_points 等）以 TEXT 类型存储在 SQLite

## 任务开始前检查清单

1. 阅读 CLAUDE.md（本文件）
2. 根据任务类型查阅对应文档（见文档导航）
3. 了解相关模块的现有代码再修改
4. 遵循 [docs/QUALITY_SCORE.md](./docs/QUALITY_SCORE.md) 中的质量标准
