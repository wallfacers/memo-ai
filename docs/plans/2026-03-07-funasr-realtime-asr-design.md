# 设计文档：FunASR 实时录音转写集成

**日期：** 2026-03-07
**状态：** 已批准，待执行
**执行计划：** [docs/exec-plans/active/05-funasr-realtime-asr.md](../exec-plans/active/05-funasr-realtime-asr.md)

---

## 目标

将 FunASR（https://github.com/modelscope/FunASR）集成为实时流式 ASR 后端，实现录音时边录边显示字幕（temp/final 双态），录音结束后智能合并实时草稿与高精度批量转写结果，最终送入 LLM 6 阶段 Pipeline。同时为 Pipeline 阶段增加逐阶段动态进度展示。

---

## Section 1：系统架构

### 新增模块

```
src-tauri/src/
├── asr/
│   ├── mod.rs           (扩展：导出 funasr、streaming)
│   ├── provider.rs      (不变：AsrProvider batch trait)
│   ├── whisper.rs       (不变)
│   ├── aliyun.rs        (不变)
│   ├── funasr.rs        (新增：WebSocket 流式 + FunASR batch AsrProvider)
│   └── streaming.rs     (新增：StreamingAsrSession trait)
└── process/
    └── funasr_server.rs (新增：FunASR 进程生命周期管理)

src/
├── components/
│   ├── RealtimeTranscript.tsx  (新增：实时字幕组件)
│   └── PipelineProgress.tsx    (新增：Pipeline 动态进度组件)
├── store/
│   └── recordingStore.ts       (扩展：录音生命周期状态)
```

### StreamingAsrSession Trait

```rust
// src-tauri/src/asr/streaming.rs
use crate::error::AppResult;

pub struct StreamingSegment {
    pub text: String,
    pub is_final: bool,
    pub segment_id: u32,
}

pub trait StreamingAsrSession: Send {
    fn send_audio_chunk(&mut self, pcm: &[i16]) -> AppResult<()>;
    fn finish(&mut self) -> AppResult<Vec<StreamingSegment>>;
}
```

### FunASR WebSocket 协议

FunASR Runtime Server 使用 JSON over WebSocket：

```json
// 发送：音频配置（握手）
{"mode":"2pass","wav_name":"meeting","wav_format":"pcm","is_speaking":true,"chunk_size":[5,10,5],"encoder_chunk_look_back":4,"decoder_chunk_look_back":0}

// 发送：音频数据（binary frame，PCM 16-bit 16kHz mono）

// 发送：结束信号
{"is_speaking":false}

// 接收：识别结果
{"mode":"2pass-online","text":"你好今天","is_final":false}
{"mode":"2pass-offline","text":"你好，今天","is_final":true}
```

### 数据流

```
录音进行中
    │
    ├─► FunASR WebSocket ─► StreamingSegment(is_final=false) ─► Tauri Event "asr_partial"
    │                   └─► StreamingSegment(is_final=true)  ─► Tauri Event "asr_final"
    │
录音结束
    │
    ├─► FunASR flush (is_speaking=false) ─► 最后 final 段
    │
    ├─► batch ASR (现有 build_asr()) ─► Vec<TranscriptSegment>
    │
    └─► smart_merge() ─► 最终 Vec<TranscriptSegment> ─► LLM Pipeline
```

### 智能合并策略（smart_merge）

```
1. 以 batch 结果为主（精度更高）
2. 将 batch 结果按时间戳分段
3. 对 batch 中静音/空段，用 streaming final 段填补
4. 若 batch 完全失败，降级使用全部 streaming final 段
5. 返回合并后的 Vec<TranscriptSegment>
```

### FunASR 进程管理策略

```
AppConfig.funasr_ws_url 非空
    └─► 直接连接（外部服务，不管理进程）

AppConfig.funasr_ws_url 为空
    ├─► 检测 funasr-server 可执行文件（PATH + 配置路径）
    │       ├─► 找到 → 自动启动，监听 localhost:10095，记录 PID
    │       │         应用退出时 kill 子进程
    │       └─► 未找到 → 禁用实时字幕，仍可用 batch ASR
    └─► 自动选择端口（默认 10095，冲突时递增）
```

---

## Section 2：录音页面生命周期状态机

### RecordingPhase 类型

```typescript
type RecordingPhase =
  | "idle"               // 未开始
  | "connecting"         // 连接 FunASR WebSocket 中
  | "recording"          // 录音中，实时字幕追加
  | "stopping"           // 已停止，等待 FunASR flush 最后 final 段
  | "batch_transcribing" // 完整文件批量转写中
  | "merging"            // 智能合并两份结果
  | "pipeline"           // LLM 6 阶段处理中
  | "done"               // 全部完成
  | "error";             // 任意阶段出错（附带 message）
```

### 状态流转

```
idle → connecting → recording → stopping → batch_transcribing → merging → pipeline → done
                                                                                       ↑
任意阶段出错 ──────────────────────────────────────────────────────────────────────► error
```

### 阶段 → UI 映射

| 阶段 | 顶部状态栏 | 字幕区 | 进度条 |
|------|-----------|--------|--------|
| `idle` | 点击开始录音 | 空 | 隐藏 |
| `connecting` | 连接中… | 等待提示 | 隐藏 |
| `recording` | 录音中 `00:03:27` | 实时追加，temp 灰色+光标 | 隐藏 |
| `stopping` | 正在停止… | 冻结，等 flush | step 1 亮 |
| `batch_transcribing` | 精确转写中… | 冻结显示草稿 | step 2 亮 |
| `merging` | 智能合并中… | 合并后更新一次 | step 3 亮 |
| `pipeline` | AI 分析中（4/6 阶段） | 显示最终文本 | step 4 亮 |
| `done` | 完成 | 最终文本 | 全部完成 |
| `error` | 出错：[消息] | 保留已有内容 | 出错步骤红色 |

---

## Section 3：Pipeline 动态进度展示

### 页面布局（录音结束后显示）

```
┌─────────────────────────────────────────────┐
│  AI 分析中                                  │
│  [shadcn Progress 组件] ████████░░  4/6     │
├─────────────────────────────────────────────┤
│  [shadcn Badge green]  文本清洗      完成    │
│  [shadcn Badge green]  说话人整理    张三、李四 · 3人 │
│  [shadcn Badge green]  结构化提取    主题：Q2 · 5人 · 3项决策 │
│  [shadcn Badge blue spinner] 会议总结  正在生成... │
│    本次会议主要讨论了下季度销售目标▌          │  ← 流式打字
│  [shadcn Badge muted]  行动项提取    等待中  │
│  [shadcn Badge muted]  报告生成      等待中  │
└─────────────────────────────────────────────┘
```

### 使用的 shadcn/ui 组件

| 用途 | 组件 |
|------|------|
| 总进度条 | `Progress` |
| 阶段状态标签 | `Badge`（variant: default/secondary/destructive） |
| 阶段结果卡片 | `Card` + `CardContent` |
| 分隔线 | `Separator` |
| 加载动画 | `Loader2`（lucide-react） |

### 各阶段完成后展示内容

| Stage | Badge 颜色 | 完成后展示摘要 |
|-------|-----------|--------------|
| 1. 文本清洗 | green | `完成（共 XXX 字）` |
| 2. 说话人整理 | green | `识别 N 人：张三、李四…` |
| 3. 结构化提取 | green | `主题：XXX · N 人 · N 项决策` |
| 4. 会议总结 | green | 摘要全文（默认展开，可折叠） |
| 5. 行动项提取 | green | `N 项：[负责人] 任务…`（列表） |
| 6. 报告生成 | green | `报告已生成，点击查看` |

### Tauri Event 协议

```typescript
// 后端 emit
"asr_partial"   → { text: string, segment_id: number }
"asr_final"     → { text: string, segment_id: number, start: number, end: number }
"pipeline_stage_start"   → { stage: 1..6, name: string }
"pipeline_stage_stream"  → { stage: 1..6, delta: string }  // LLM 流式输出
"pipeline_stage_done"    → { stage: 1..6, summary: string }
"recording_phase_change" → { phase: RecordingPhase, message?: string }
```

---

## Section 4：配置变更

### AppConfig 新增字段（向后兼容）

```rust
#[serde(default)]
pub funasr_ws_url: String,         // 外部服务 URL，空则尝试本地自管理
#[serde(default)]
pub funasr_server_path: String,    // funasr-server 可执行文件路径
#[serde(default = "default_funasr_port")]
pub funasr_port: u16,              // 自管理模式监听端口，默认 10095
#[serde(default = "default_true")]
pub funasr_enabled: bool,          // 是否启用实时字幕
```

### Settings 页新增 FunASR 面板

在 ASR Provider 选择器中新增 `funasr` 选项，对应配置面板包含：
- WebSocket 地址输入框（留空则自动管理）
- funasr-server 路径输入框 + 检测按钮
- 端口配置
- 启用/禁用实时字幕开关
- 连接测试按钮

---

## Section 5：需要改动的现有文档

| 文档 | 改动 |
|------|------|
| `ARCHITECTURE.md` | 新增 `process/` 模块；ASR 模块表增加 funasr/streaming；数据流图增加实时字幕分支 |
| `docs/PLANS.md` | v1.0 实时转写字幕状态改为进行中，标注 FunASR |
| `docs/design-docs/index.md` | 新增本设计文档索引 |
| `CLAUDE.md` | ASR 模块说明增加新文件 |

---

## 不在本次范围内

- FunASR 云端 ModelScope API（后续支持）
- 说话人识别（diarization）——FunASR 支持但留后续
- FunASR Docker 自动拉取——用户需自行安装
- macOS/Linux 进程管理差异——本次仅 Windows
