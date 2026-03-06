# ACP 智能体集成设计

## 背景

ACP（Agent Communication Protocol）是一种基于 REST 的标准化智能体通信协议，允许客户端与任意兼容 ACP 的智能体进行多轮对话交互。

引入 ACP 的目标：在 Pipeline 的**可交互阶段**，允许用户通过多轮对话与智能体协作，对 AI 输出进行动态调整，而不仅仅是被动接收一次性生成结果。

## ACP 协议要点

- 传输层：HTTP REST + JSON，流式输出用 SSE（Server-Sent Events）
- 核心资源：`/runs`（创建会话）、`/runs/{run_id}`（继续对话/查询状态）
- 多轮对话：通过 `run_id` 标识同一会话，每次用户追加消息后继续 run
- 智能体无关：任何实现了 ACP 接口的智能体均可接入（BeeAI、LangChain、自定义等）

```
POST /runs
{
  "agent_id": "meeting-report-agent",
  "input": [{"role": "user", "content": "..."}]
}
→ { "run_id": "xxx", "output": [...] }

POST /runs/{run_id}                   ← 继续多轮
{
  "input": [{"role": "user", "content": "帮我把结论部分再精简一下"}]
}
→ { "output": [...] }
```

## 可交互阶段划分

并非所有 Pipeline 阶段都适合多轮交互，按用户需求频率划分：

| 阶段 | 是否支持 ACP 多轮 | 理由 |
|------|-----------------|------|
| Stage 1 文本清洗 | 否 | 纯文本修复，用户通常不需干预 |
| Stage 2 说话人整理 | 否 | 自动化程度高，错误可在 Stage 3 纠正 |
| Stage 3 结构化提取 | **是** | 参会人、决议可能需要用户确认/补充 |
| Stage 4 会议总结 | **是** | 纪要风格、详略程度因人而异 |
| Stage 5 行动项提取 | **是** | 任务分配、截止日期常需用户确认 |
| Stage 6 报告生成 | **是（主要）** | 报告是最终交付物，多轮打磨价值最高 |

## 交互模式

### 模式 A：自动生成 → 用户触发多轮（推荐）

```
Pipeline 自动执行完成
    │
    ▼
AI 输出展示给用户（报告/纪要/行动项）
    │
用户点击「用智能体调整」
    │
    ▼
ACP Client 创建会话（POST /runs）
    │
    ▼
对话界面：用户输入 → 智能体响应 → 更新展示
    │
    ▼（用户满意，点击「保存」）
最终结果写回 SQLite
```

### 模式 B：Stage 内嵌交互（仅 Stage 3/5，结构化阶段）

Stage 3 和 Stage 5 输出 JSON 后，用户可在结构化编辑界面直接修改，也可触发 ACP 对话来批量调整（如"把所有截止日期推迟一周"）。

## 架构新增模块

### Rust 后端

新增 `src-tauri/src/acp/` 模块：

```
src-tauri/src/acp/
├── mod.rs          # 公开接口
├── client.rs       # ACP HTTP 客户端（reqwest）
└── session.rs      # 多轮会话状态管理
```

**`client.rs`** 核心职责：
- `create_run(agent_url, agent_id, messages)` → `RunResponse`
- `continue_run(agent_url, run_id, message)` → `RunResponse`
- 支持 SSE 流式响应（逐 token 推送到前端）

**`session.rs`** 核心职责：
- 维护 `(meeting_id, stage, run_id)` 的会话映射
- 会话持久化到 `acp_sessions` 表，支持跨重启恢复

### 数据库新增表

```sql
-- ACP 会话记录
CREATE TABLE IF NOT EXISTS acp_sessions (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id  INTEGER NOT NULL REFERENCES meetings(id) ON DELETE CASCADE,
    stage       TEXT    NOT NULL,   -- 'summary' | 'actions' | 'report' | 'structure'
    run_id      TEXT    NOT NULL,   -- ACP run_id
    agent_url   TEXT    NOT NULL,   -- 智能体地址
    agent_id    TEXT    NOT NULL,   -- 智能体标识
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

-- ACP 对话消息
CREATE TABLE IF NOT EXISTS acp_messages (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  INTEGER NOT NULL REFERENCES acp_sessions(id) ON DELETE CASCADE,
    role        TEXT    NOT NULL,   -- 'user' | 'agent'
    content     TEXT    NOT NULL,
    created_at  TEXT    NOT NULL
);
```

### 前端新增组件

```
src/components/
└── AgentChat/
    ├── AgentChatPanel.tsx    # 对话面板（侧边栏或底部抽屉）
    ├── MessageBubble.tsx     # 单条消息气泡
    └── AgentChatInput.tsx    # 输入框 + 发送按钮
```

新增 Zustand store：`src/store/acpStore.ts`
- 管理当前活跃会话、消息列表、流式输出状态

## ACP 智能体配置

用户在 Settings 页面配置：

| 字段 | 说明 | 示例 |
|------|------|------|
| 智能体地址 | ACP 服务 base URL | `http://localhost:8333` |
| 智能体 ID | 要调用的 agent_id | `meeting-assistant` |
| 启用阶段 | 哪些 Stage 显示「智能体调整」入口 | Stage 4/5/6 |

支持配置多个智能体（不同 Stage 可调用不同专业智能体）。

## 与现有 LLM 的关系

ACP 智能体和现有 LLM Pipeline **相互独立**：

- LLM Pipeline（Stage 1~6）：**自动执行**，生成初始结果，走 `src-tauri/src/llm/`
- ACP 智能体：**用户主动触发**，用于交互式精炼，走 `src-tauri/src/acp/`

ACP 智能体底层可以是 Ollama、OpenAI 或任何其他模型，与 Pipeline 的 LLM 配置无关。

## 数据流（含 ACP）

```
Pipeline 完成
    │
    ▼
meetings.report / summary / action_items 存入 DB
    │
    ▼
前端展示初始结果
    │
用户点击「智能体调整」（Stage 4/5/6 可用）
    │
    ▼
invoke('acp_create_session', { meeting_id, stage, content })
    │
    ▼
ACP Client → POST /runs → 智能体
    │
    ▼（SSE 流式）
invoke event 'acp_token' 推送到前端（逐 token 显示）
    │
    ▼
用户继续追问
    │
invoke('acp_continue_session', { session_id, message })
    │
    ▼
ACP Client → POST /runs/{run_id}
    │
    ▼
用户满意 → invoke('acp_save_result', { session_id, stage })
    │
    ▼
更新对应 DB 字段（meetings.report / summary / action_items）
```
