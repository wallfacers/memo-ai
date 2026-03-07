# 评估框架测试指南

本文档说明 memo-ai 评估框架的测试内容、运行方式和扩展方法，面向日常开发使用。

**实现状态**：已完成（截至 2026-03-07）

---

## 概览

评估框架分两层，覆盖不同测试目的：

| 层级 | 命令 | 耗时 | 用途 | 是否需要 LLM |
|------|------|------|------|-------------|
| 单元回归测试 | `cargo test --lib llm::pipeline::tests` | <1s | 验证 pipeline 逻辑不破坏 | 否（Mock） |
| 完整质量评估 | `cargo run --manifest-path src-tauri/Cargo.toml --bin eval` | 1~3min | 验证 LLM 输出质量 | 是 |

---

## 第一层：单元回归测试

### 运行方式

```bash
cd src-tauri
cargo test --lib llm::pipeline::tests
```

### 测试清单（共 6 个）

| 测试名 | 验证内容 |
|--------|---------|
| `test_stage3_parses_valid_json` | stage3 能正确解析合法 JSON，提取 topic 和 participants |
| `test_stage3_returns_empty_struct_on_invalid_json` | stage3 收到非法 JSON 时安全回退（返回空结构体，不 panic） |
| `test_stage5_parses_action_items` | stage5 能正确解析行动项数组，含 task/owner/deadline 字段 |
| `test_stage5_returns_empty_vec_on_invalid_json` | stage5 收到非法 JSON 时安全回退（返回空数组，不 panic） |
| `test_extract_json_strips_markdown_fence` | `extract_json` 能正确剥离 ` ```json ... ``` ` 代码块 |
| `test_extract_json_passthrough_plain` | `extract_json` 对纯 JSON 字符串直接透传，不做多余处理 |

### 使用 Mock LLM

单元测试使用 `MockLlmClient`，不调用真实 LLM：

```rust
// 按顺序返回预设响应（round-robin）
let mock = MockLlmClient::new(vec!["response1", "response2"]);

// 单个 JSON 响应的便捷构造
let mock = MockLlmClient::with_json(r#"{"topic":"..."}"#);
```

**何时运行**：每次修改 `pipeline.rs`、`mock_client.rs` 或 prompt 模板后。

---

## 第二层：完整质量评估（eval binary）

### 运行方式

```bash
# 完整评估（含 LLM-as-Judge）
cargo run --manifest-path src-tauri/Cargo.toml --bin eval

# 纯代码打分（快速，不调用 LLM）
cargo run --manifest-path src-tauri/Cargo.toml --bin eval -- --no-llm-judge

# 只跑某个场景
cargo run --manifest-path src-tauri/Cargo.toml --bin eval -- --fixture tech_review

# 输出报告到文件
cargo run --manifest-path src-tauri/Cargo.toml --bin eval -- --output eval-report.md
```

### LLM 配置

创建 `eval-config.json`（已在 .gitignore，不会提交）：

```json
{
  "provider": "openai",
  "base_url": "https://api.openai.com/v1",
  "model": "gpt-4o-mini",
  "api_key": "sk-..."
}
```

参考 `eval-config.json.example`。

---

## 测试语料（Fixture）

语料文件位于 `evals/fixtures/`，格式为 TOML。

### 当前 4 个 Fixture

#### 1. `tech_review_01` — 技术评审（normal）

**场景**：团队评审缓存方案，决定采用 Redis，分配两个行动项。

**转写摘要**：
> 张三主持，李四和王五参与。决定采用 Redis 集群方案。
> 李四负责方案细化（下周五），王五负责压测（周三）。

**检查点**：
- 摘要必须包含：`Redis`、`李四`、`王五`
- participants 必须包含：张三、李四、王五
- 行动项数量 >= 2

---

#### 2. `requirements_01` — 需求讨论（normal）

**场景**：产品、前端、后端讨论用户反馈模块方案，确定内嵌表单 P1 优先级。

**转写摘要**：
> 确定采用 App 内嵌表单方案，P1 优先级。
> 前端本周五提供排期，后端下周一前完成接口。

**检查点**：
- 摘要必须包含：`反馈`、`内嵌`、`P1`
- participants 必须包含：产品、前端、后端
- 行动项数量 >= 2

---

#### 3. `weekly_standup_01` — 周会（normal）

**场景**：团队周会，小明和小红分别汇报进展，产出 1 个行动项。

**转写摘要**：
> 小明：登录模块测试覆盖率 60%->85%，下周做支付模块。
> 小红：修复 3 个 Bug，含 iOS 崩溃热更新。
> 行动项：小明建立 SDK 升级回归测试流程（两周内）。

**检查点**：
- 摘要必须包含：`登录`、`iOS`、`崩溃`、`SDK`
- participants 必须包含：小明、小红
- 行动项数量 >= 1

---

#### 4. `edge_empty_01` — 边界：极短转写（edge）

**场景**：极短无效转写（"嗯。好的。散会。"），验证 pipeline 不崩溃。

**检查点**：
- 无关键词要求
- 无 participants 要求
- 行动项数量 >= 0（即不检查）
- 不触发 LLM-as-Judge（golden_summary 为空）

---

## 评分机制

### 代码打分（6 项加权检查）

| 检查项 | 权重 | 说明 |
|--------|------|------|
| `stage1_clean_nonempty` | 0.5 | 清洗后转写非空 |
| `stage3_participants_nonempty` | 0.5 | 参与者列表非空 |
| `stage3_required_participants` | 1.0 | 包含 fixture 要求的所有参与者 |
| `stage4_summary_keywords` | 1.0 | 摘要包含所有必填关键词 |
| `stage5_min_action_items` | 1.0 | 行动项数量达到最低要求 |
| `stage6_report_nonempty` | 0.5 | 最终报告非空 |

**计算方式**：`code_score = 通过项权重之和 / 总权重`（范围 0.0~1.0）

### LLM-as-Judge 打分

**触发条件**：`code_score >= 0.6` 且 fixture 有 `golden_summary`

使用 `evals/rubrics/llm_judge_prompt.txt` 提示词，评分维度：
- 事实准确性（40%）：关键决策、负责人、截止日期
- 完整性（35%）：重要信息是否遗漏
- 简洁性（25%）：是否有冗余信息

返回 `score`（0.0~1.0）和 `reason`（简短说明）。

### 最终状态判定

| 状态 | 条件 |
|------|------|
| `PASS` | `code_score >= 0.8` 且 `llm_score >= 0.7`（或无 LLM 评分） |
| `WARN` | `code_score >= 0.6` 但 `llm_score < 0.7` |
| `FAIL` | `code_score < 0.6` |

---

## 报告格式示例

```markdown
# memo-ai Eval Report — 2026-03-07

| Fixture            | Scene    | Code分 | LLM分 | 耗时   | 状态 |
|--------------------|----------|--------|-------|--------|------|
| edge_empty_01      | 边界     | 1.00   | —     | 120ms  | PASS |
| requirements_01    | 需求讨论 | 0.91   | 0.82  | 4800ms | PASS |
| tech_review_01     | 技术评审 | 1.00   | 0.88  | 5200ms | PASS |
| weekly_standup_01  | 周会     | 0.91   | 0.75  | 4600ms | PASS |

**总体通过率：4/4（100%）**
**平均 LLM 评分：0.82**
```

---

## 添加新 Fixture

1. 在 `evals/fixtures/` 创建新 `.toml` 文件：

```toml
[meta]
id = "your_scene_01"         # 唯一 ID，用于 --fixture 过滤
scene = "场景名称"
difficulty = "normal"        # normal | hard | edge

[input]
raw_transcript = """
（粘贴真实或手写的会议转写文本）
"""

[expected]
summary_must_contain = ["关键词1", "关键词2"]
min_action_items = 1
required_participants = ["张三", "李四"]
golden_summary = "（理想的摘要内容，用于 LLM-as-Judge 对比）"

[[expected.golden_actions]]
task = "行动项描述"
owner = "负责人"
deadline = "截止时间"
```

2. 运行验证：

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin eval -- --fixture your_scene
```

---

## 文件索引

| 路径 | 说明 |
|------|------|
| `evals/fixtures/*.toml` | 测试语料 |
| `evals/rubrics/llm_judge_prompt.txt` | LLM 评分提示词 |
| `eval-config.json` | LLM 配置（本地，不提交） |
| `eval-config.json.example` | 配置模板 |
| `src-tauri/src/eval/fixture.rs` | Fixture 数据结构与加载器 |
| `src-tauri/src/eval/grader_code.rs` | 代码打分器 |
| `src-tauri/src/eval/grader_llm.rs` | LLM-as-Judge 打分器 |
| `src-tauri/src/eval/reporter.rs` | Markdown 报告生成器 |
| `src-tauri/src/bin/eval.rs` | eval CLI 二进制入口 |
| `src-tauri/src/llm/mock_client.rs` | 单元测试用 Mock LLM |
| `src-tauri/src/llm/pipeline.rs` | LLM Pipeline（含单元测试） |
