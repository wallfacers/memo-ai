# AI 处理管道设计

## 概述

memo-ai 将会议处理拆分为 6 个顺序执行的 AI 阶段，每个阶段对应 `prompts/` 目录中的一个 Prompt 模板文件。

阶段分为两类：
- **自动阶段**（Stage 1~2）：全自动执行，无需用户干预
- **可交互阶段**（Stage 3~6，标注 `[ACP]`）：自动执行后，用户可选择通过 ACP 协议调用外部智能体进行多轮对话精炼输出

ACP 集成详见 [acp-integration.md](./acp-integration.md)。

## 管道全貌

```
Audio File
    │
    ▼
[ASR] Whisper.cpp
    │  原始转写文本（含噪声、断句问题）
    ▼
[Stage 1] 文本清洗       →  prompts/01_clean.txt
    │  清洁、正确断句的文本
    ▼
[Stage 2] 说话人整理      →  prompts/02_speaker.txt
    │  [说话人]: 发言内容 格式
    ▼
[Stage 3] 结构化提取 [ACP]  →  prompts/03_structure.txt
    │  JSON: {topic, participants, key_points, decisions, risks}
    │  ↳ 用户可触发 ACP 多轮对话：补充参会人、修正决议
    ▼
[Stage 4] 会议总结   [ACP]  →  prompts/04_summary.txt
    │  会议纪要文本
    │  ↳ 用户可触发 ACP 多轮对话：调整风格、详略程度
    ▼
[Stage 5] 行动项提取 [ACP]  →  prompts/05_actions.txt
    │  JSON: [{task, owner, deadline}]
    │  ↳ 用户可触发 ACP 多轮对话：修改负责人、推迟截止日期
    ▼
[Stage 6] 报告生成   [ACP]  →  prompts/06_report.txt
    │  Markdown 格式会议报告
    │  ↳ 用户可触发 ACP 多轮对话：精炼报告内容（主要交互入口）
    ▼
SQLite 存储（acp_sessions / acp_messages 同步记录对话历史）
```

## 各阶段说明

### Stage 1：文本清洗
- **输入**：ASR 原始文本（`{{transcript}}`）
- **输出**：清洁文本，含正确标点和断句
- **模型**：任意 LLM（轻量级任务，可用小模型）

### Stage 2：说话人整理
- **输入**：清洁文本
- **输出**：`[说话人]: 内容` 格式的结构化对话
- **前提**：需要 ASR 提供 Speaker Diarization 信息

### Stage 3：结构化提取
- **输入**：整理后的会议文本（`{{meeting_text}}`）
- **输出**：JSON 结构（存入 `meeting_structures` 表）
```json
{
  "topic": "string",
  "participants": ["string"],
  "key_points": ["string"],
  "decisions": ["string"],
  "risks": ["string"]
}
```

### Stage 4：会议总结
- **输入**：会议文本
- **输出**：5 部分会议纪要（主题、时间、人员、内容、结论）

### Stage 5：行动项提取
- **输入**：会议文本
- **输出**：JSON 数组（存入 `action_items` 表）
```json
[{"task": "string", "owner": "string", "deadline": "string"}]
```

### Stage 6：报告生成
- **输入**：会议纪要 + 行动项（`{{summary}}` + `{{actions}}`）
- **输出**：Markdown 格式报告（存入 `meetings.report` 字段）
- **ACP 交互**：报告生成后，用户可与 ACP 智能体多轮对话精炼报告，最终满意后点击「保存」覆盖 `meetings.report`

## Prompt 模板管理

Prompt 文件使用 `{{变量名}}` 占位符，由 `src-tauri/src/llm/pipeline.rs` 在运行时替换。

修改 Prompt 时只需编辑 `prompts/` 目录中的 `.txt` 文件，无需重新编译。

## LLM 后端切换

`src-tauri/src/llm/client.rs` 定义统一接口，`ollama.rs` 和 `openai.rs` 分别实现。

用户在 Settings 页面选择后端，配置存入 `settingsStore`。

## ACP 智能体扩展

Stage 3~6 完成后，用户可选择调用 ACP 兼容智能体进行多轮交互精炼。

ACP 客户端实现在 `src-tauri/src/acp/`，与 LLM Pipeline 相互独立。详见 [acp-integration.md](./acp-integration.md)。
