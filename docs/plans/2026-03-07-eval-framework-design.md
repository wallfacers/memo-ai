# memo-ai 自动化评估框架设计

**日期**：2026-03-07
**状态**：已批准，待实现
**背景**：当前测试方式需要手动打开页面录制会议，耗时耗力。本设计基于 Anthropic《揭秘 AI 智能体评估》的经验，为 memo-ai 的 LLM Pipeline 构建自动化评估体系。

---

## 目标

1. **LLM Pipeline 输出质量**：给定固定转写文本，验证 6 个阶段输出符合预期
2. **回归防护**：改 Prompt 或换模型后，自动检测退化
3. **端到端链路**：绕过 UI 和录音，直接测试 Rust 后端 pipeline

---

## 整体架构

```
memo-ai/
├── evals/
│   ├── fixtures/
│   │   ├── tech_review.toml       # 技术评审场景
│   │   ├── requirements.toml      # 需求讨论场景
│   │   ├── weekly_standup.toml    # 周会场景
│   │   └── edge_empty.toml        # 边界：极短/空转写
│   └── rubrics/
│       ├── code_checks.toml       # 代码打分规则
│       └── llm_judge_prompt.txt   # LLM-as-Judge 提示词
│
└── src-tauri/src/
    ├── bin/
    │   └── eval.rs                # eval 独立二进制入口
    ├── llm/
    │   ├── pipeline.rs            # 现有（不改动）
    │   └── mock_client.rs         # 新增：Mock LLM 客户端
    └── eval/                      # 新增：评估模块
        ├── mod.rs
        ├── fixture.rs             # Fixture 加载与解析
        ├── grader_code.rs         # 代码打分器
        ├── grader_llm.rs          # LLM-as-Judge 打分器
        └── reporter.rs            # Markdown 报告生成
```

**两条执行路径**：

| 路径 | 命令 | LLM | 速度 | 用途 |
|------|------|-----|------|------|
| 单元回归 | `cargo test` | Mock（固定响应） | 秒级 | 逻辑回归，CI 用 |
| 完整评估 | `cargo run --bin eval` | 真实（Ollama/OpenAI） | 分钟级 | 质量评估，Prompt 迭代用 |

---

## Fixture 格式

```toml
# evals/fixtures/tech_review.toml

[meta]
id = "tech_review_01"
scene = "技术评审"
difficulty = "normal"   # normal | hard | edge

[input]
raw_transcript = """
张三：今天评审一下新的缓存方案。
李四：我觉得 Redis 集群方案可行，但要考虑数据一致性。
张三：那我们决定采用 Redis，李四负责方案细化，下周五提交文档。
王五：我来做压测，周三出结果。
"""

[expected]
# 代码打分：stage4 摘要必须命中的关键词
summary_must_contain = ["Redis", "缓存", "李四"]

# 代码打分：stage5 行动项最少数量
min_action_items = 2

# 代码打分：stage3 participants 必须包含
required_participants = ["张三", "李四", "王五"]

# LLM-as-Judge：黄金摘要（用于对比评分）
golden_summary = "会议决定采用 Redis 集群方案，李四负责方案细化（周五截止），王五负责压测（周三截止）。"

# LLM-as-Judge：预期行动项参考
golden_actions = [
  { task = "Redis 方案细化", owner = "李四", deadline = "下周五" },
  { task = "压测", owner = "王五", deadline = "周三" },
]
```

---

## 代码打分规则

```toml
# evals/rubrics/code_checks.toml

[[checks]]
name = "stage3_json_valid"
description = "stage3 输出能解析为合法 JSON"
weight = 1.0

[[checks]]
name = "stage3_participants_nonempty"
description = "participants 字段非空"
weight = 0.5

[[checks]]
name = "stage3_required_participants"
description = "participants 包含 required_participants 中的所有人"
weight = 1.0

[[checks]]
name = "stage4_summary_keywords"
description = "摘要命中 summary_must_contain 中的所有关键词"
weight = 1.0

[[checks]]
name = "stage5_json_valid"
description = "stage5 输出能解析为合法 JSON array"
weight = 1.0

[[checks]]
name = "stage5_min_actions"
description = "行动项数量 >= min_action_items"
weight = 1.0
```

**code_score = 加权命中数 / 加权总数**（0.0 ~ 1.0）

---

## LLM-as-Judge 提示词

```
# evals/rubrics/llm_judge_prompt.txt

你是一名会议记录质量评审员。请对以下会议摘要进行评分。

【黄金答案】
{{golden_summary}}

【待评摘要】
{{actual_summary}}

【评分标准】
- 事实准确性（关键决策/负责人/截止日期是否正确）：40%
- 完整性（重要信息是否遗漏）：35%
- 简洁性（是否有冗余信息）：25%

请只返回 JSON，不要有其他内容：
{"score": 0.85, "reason": "简短说明扣分原因"}
```

**触发条件**：`code_score >= 0.6` 才运行 LLM-as-Judge，避免在基础格式错误时浪费调用。

---

## 评分数据结构

```rust
struct EvalResult {
    fixture_id: String,
    scene: String,
    difficulty: String,
    code_score: f32,            // 0.0~1.0，加权通过率
    llm_score: Option<f32>,     // 0.0~1.0，仅 code_score >= 0.6 时运行
    llm_reason: Option<String>,
    passed_checks: Vec<String>,
    failed_checks: Vec<String>,
    duration_ms: u64,
}
```

---

## eval CLI 接口

```bash
# 跑全部 fixture
cargo run --bin eval

# 只跑某个场景
cargo run --bin eval -- --fixture tech_review

# 跳过 LLM-as-Judge（纯代码打分，速度快）
cargo run --bin eval -- --no-llm-judge

# 指定 LLM 配置
cargo run --bin eval -- --config path/to/config.json

# 输出报告到文件（默认输出到 stdout）
cargo run --bin eval -- --output eval-report.md
```

---

## 报告格式

```markdown
# memo-ai Eval Report — 2026-03-07

| Fixture         | Scene  | Code分 | LLM分 | 耗时  | 状态 |
|-----------------|--------|--------|-------|-------|------|
| tech_review_01  | 技术评审| 1.00   | 0.88  | 4.2s  | PASS |
| requirements_01 | 需求讨论| 0.80   | 0.72  | 5.1s  | WARN |
| edge_empty_01   | 边界    | 0.50   | —     | 0.3s  | FAIL |

总体通过率：2/3（67%）
平均 LLM 评分：0.80

## 失败详情

### edge_empty_01
- FAIL stage5_min_actions: 期望 >= 1，实际 0
- FAIL stage3_participants_nonempty: participants 为空
```

**状态规则**：
- `PASS`：code_score >= 0.8 且 llm_score >= 0.7（或无 LLM 评分时 code_score >= 0.8）
- `WARN`：code_score >= 0.6 但 llm_score < 0.7
- `FAIL`：code_score < 0.6

---

## Mock LLM 客户端（单元测试用）

```rust
// src-tauri/src/llm/mock_client.rs
pub struct MockLlmClient {
    // 按调用顺序返回预设响应
    responses: Vec<String>,
    call_count: std::sync::atomic::AtomicUsize,
}

impl MockLlmClient {
    pub fn with_responses(responses: Vec<&str>) -> Self { ... }
}

// 测试示例
#[test]
fn test_stage3_parses_json() {
    let mock = MockLlmClient::with_responses(vec![
        r#"{"topic":"缓存方案","participants":["张三","李四"],"key_points":[],"decisions":[],"risks":[]}"#,
    ]);
    let pipeline = Pipeline::new(&mock, Path::new("prompts"));
    let result = pipeline.stage3_structure("任意输入");
    assert_eq!(result.participants.len(), 2);
}
```

---

## 实现优先级

1. `mock_client.rs` + pipeline 单元测试（最快验证逻辑回归）
2. `fixture.rs` + `grader_code.rs`（代码打分器）
3. `eval.rs` binary + CLI 参数解析
4. `grader_llm.rs`（LLM-as-Judge）
5. `reporter.rs`（Markdown 报告）
6. 补充 fixture 语料（3~5 个场景）

---

## 关键决策记录

| 决策 | 原因 |
|------|------|
| 全部在 Rust 实现，不用 Python | 复用现有 LlmClient trait，零额外依赖 |
| LLM-as-Judge 仅在 code_score >= 0.6 时触发 | 避免在基础格式错误时浪费 LLM 调用 |
| Fixture 用 TOML 格式 | Rust toml crate 原生支持，比 JSON 更易读写 |
| 独立 eval binary 而非集成进主程序 | 避免污染 Tauri 主程序，eval 无需 GUI 上下文 |
