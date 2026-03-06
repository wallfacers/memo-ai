# 系统架构

## 总体架构

memo-ai 采用 Tauri 架构，前端（React/TypeScript）与后端（Rust）通过 IPC 通信。

```
┌──────────────────────────────────────────────────────┐
│                   Desktop App (Tauri)                 │
│                                                      │
│  ┌─────────────────┐   ┌────────────────────────┐   │
│  │  Frontend        │   │     Rust Backend       │   │
│  │  React + TS      │◄──►   Tauri Commands       │   │
│  │  Zustand         │   │                        │   │
│  │  AgentChatPanel  │   │  ┌──────────────────┐  │   │
│  └─────────────────┘   │  │ Audio            │  │   │
│                         │  │ WASAPI/CoreAudio │  │   │
│                         │  └────────┬─────────┘  │   │
│                         │           │            │   │
│                         │  ┌────────▼─────────┐  │   │
│                         │  │ ASR (Whisper.cpp)│  │   │
│                         │  └────────┬─────────┘  │   │
│                         │           │            │   │
│                         │  ┌────────▼─────────┐  │   │
│                         │  │ LLM Pipeline     │  │   │
│                         │  │ (Stage 1~6 自动) │  │   │
│                         │  └────────┬─────────┘  │   │
│                         │           │            │   │
│                         │  ┌────────▼─────────┐  │   │
│                         │  │ ACP Client       │  │   │
│                         │  │ (多轮交互，可选) │  │   │
│                         │  └────────┬─────────┘  │   │
│                         │           │            │   │
│                         │  ┌────────▼─────────┐  │   │
│                         │  │ SQLite DB        │  │   │
│                         │  └──────────────────┘  │   │
│                         └────────────────────────┘   │
└──────────────────────────────────────────────────────┘
                               │ ACP REST
                               ▼
                ┌──────────────────────────┐
                │  ACP 兼容智能体（外部）   │
                │  BeeAI / LangChain /     │
                │  自定义 Agent / 任意实现  │
                └──────────────────────────┘
```

## 模块说明

### 前端模块（`src/`）

| 模块 | 路径 | 职责 |
|------|------|------|
| Pages | `src/pages/` | Home（会议列表）、Meeting（会议详情）、Settings（设置） |
| Components | `src/components/` | RecordButton、TranscriptView、ActionItemList、MeetingCard |
| Store | `src/store/` | meetingStore（会议状态）、settingsStore（用户设置） |
| Hooks | `src/hooks/` | useRecording（录音控制）、useTauriCommands（后端调用封装） |
| Types | `src/types/` | 共享 TypeScript 类型定义 |

### 后端模块（`src-tauri/src/`）

| 模块 | 路径 | 职责 |
|------|------|------|
| audio | `src/audio/` | 音频采集（capture.rs）、WASAPI 实现（wasapi.rs）、编码（encoder.rs） |
| asr | `src/asr/` | Whisper 推理（whisper.rs）、转写结果（transcript.rs）、funasr.rs（WebSocket 流式 + batch）、streaming.rs（StreamingAsrSession trait） |
| process | `src-tauri/src/process/` | FunASR 进程生命周期管理（funasr_server.rs） |
| llm | `src/llm/` | LLM 客户端接口（client.rs）、Ollama 实现（ollama.rs）、OpenAI 实现（openai.rs）、处理管道（pipeline.rs） |
| db | `src/db/` | 数据库连接（connection.rs）、数据库迁移（migrations.rs）、数据模型（models.rs） |
| acp | `src/acp/` | ACP 客户端（client.rs）、多轮会话管理（session.rs） |
| commands | `commands.rs` | 所有 Tauri IPC 命令 |
| error | `error.rs` | 统一错误类型 |

## 数据流

### 录音 → 转写 → AI 处理

```
用户点击录音
    │
    ▼
Audio Capture (WASAPI)
    │  PCM 音频流
    ▼
Audio Encoder (WAV/MP3)
    │  音频文件
    ├─► Whisper ASR（批量转写）
    │       │  原始转写文本
    │       ▼
    │   LLM Pipeline (6 stages)
    │
    └─► FunASR WebSocket（实时流式）
            │  temp/final 双态字幕
            ▼
        前端实时字幕展示

（批量路径）
LLM Pipeline (6 stages)
    │  结构化会议数据
    ▼
SQLite DB
    │
    ▼
前端展示（TranscriptView + ActionItemList）
```

### LLM Pipeline 数据流

Stage 1~6 自动顺序执行，带 [ACP] 标记的阶段完成后用户可选择触发多轮交互。

```
原始转写
    │
    ├─► [Stage 1] 文本清洗         → 清洁文本
    │
    ├─► [Stage 2] 说话人整理        → 结构化对话
    │
    ├─► [Stage 3] 结构化提取 [ACP] → JSON（topic, participants, decisions...）
    │
    ├─► [Stage 4] 会议总结   [ACP] → 会议纪要文本
    │
    ├─► [Stage 5] 行动项提取 [ACP] → JSON（task, owner, deadline[]）
    │
    └─► [Stage 6] 报告生成   [ACP] → Markdown 报告
                                        │
                        用户触发 ACP 多轮对话
                                        │
                              ┌─────────▼─────────┐
                              │  ACP 兼容智能体    │
                              │  (多轮精炼报告)    │
                              └─────────┬─────────┘
                                        │ 用户满意后保存
                                        ▼
                                    SQLite DB
```

## 数据库 Schema

详见 [docs/generated/db-schema.md](./docs/generated/db-schema.md)。

核心表：`meetings`、`transcripts`、`action_items`、`meeting_structures`、`acp_sessions`、`acp_messages`

## 跨平台差异

| 功能 | Windows | macOS | Linux |
|------|---------|-------|-------|
| 音频 API | WASAPI | CoreAudio | ALSA/PulseAudio |
| 系统音采集 | 原生支持 | 需虚拟音频设备（BlackHole） | 需配置 |
| 打包格式 | .msi / .exe | .dmg / .app | .AppImage / .deb |
