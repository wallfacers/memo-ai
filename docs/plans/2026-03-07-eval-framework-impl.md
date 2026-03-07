# memo-ai 自动化评估框架实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 构建自动化 LLM Pipeline 评估框架，彻底替代手动 UI 录制测试方式。

**Architecture:** 两层架构——`cargo test` 使用 Mock LLM 做快速回归，`cargo run --bin eval` 连接真实 LLM 批量跑 fixture 并输出 Markdown 报告。评估模块完全独立于 Tauri GUI，代码打分 + LLM-as-Judge 混合评分。

**Tech Stack:** Rust、toml crate（fixture 解析）、reqwest blocking（已有）、serde_json（已有）

---

## 文件结构总览

```
src-tauri/
  Cargo.toml                        # 修改：新增 toml 依赖 + eval binary
  src/
    lib.rs                          # 修改：新增 pub mod eval
    llm/
      mod.rs                        # 修改：新增 pub mod mock_client (cfg test)
      mock_client.rs                # 新建
      pipeline.rs                   # 修改：新增 #[cfg(test)] 模块
    eval/
      mod.rs                        # 新建
      fixture.rs                    # 新建
      grader_code.rs                # 新建
      grader_llm.rs                 # 新建
      reporter.rs                   # 新建
    bin/
      eval.rs                       # 新建

evals/
  fixtures/
    tech_review.toml                # 新建
    requirements.toml               # 新建
    weekly_standup.toml             # 新建
    edge_empty.toml                 # 新建
  rubrics/
    llm_judge_prompt.txt            # 新建
```

---

## Task 1: 添加依赖并注册 eval binary

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: 在 `[dependencies]` 中添加 toml crate**

在 `Cargo.toml` 的 `serde_json = "1"` 下方添加：

```toml
toml = "0.8"
```

**Step 2: 在 `[[bin]]` 段添加 eval binary**

在现有 `[[bin]]` 段（`name = "memo-ai"`）下方添加：

```toml
[[bin]]
name = "eval"
path = "src/bin/eval.rs"
```

**Step 3: 验证依赖能解析**

```bash
cd src-tauri && cargo fetch
```

Expected: 无错误，`toml` 被下载

**Step 4: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "build: add toml dep and eval binary target"
```

---

## Task 2: MockLlmClient

**Files:**
- Create: `src-tauri/src/llm/mock_client.rs`
- Modify: `src-tauri/src/llm/mod.rs`

**Step 1: 创建 `mock_client.rs`**

```rust
// src-tauri/src/llm/mock_client.rs
//! Mock LLM client for unit tests and eval dry-runs.
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::error::AppResult;
use super::client::LlmClient;

/// Returns preset responses in sequence (round-robin if exhausted).
pub struct MockLlmClient {
    responses: Vec<String>,
    call_count: AtomicUsize,
}

impl MockLlmClient {
    pub fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: responses.into_iter().map(String::from).collect(),
            call_count: AtomicUsize::new(0),
        }
    }

    /// Convenience: single JSON response for stage3/stage5 tests.
    pub fn with_json(json: &str) -> Self {
        Self::new(vec![json])
    }
}

impl LlmClient for MockLlmClient {
    fn complete(&self, _prompt: &str) -> AppResult<String> {
        let count = self.call_count.fetch_add(1, Ordering::SeqCst);
        let idx = count % self.responses.len();
        Ok(self.responses[idx].clone())
    }

    fn provider_name(&self) -> &str {
        "mock"
    }
}
```

**Step 2: 在 `llm/mod.rs` 注册（仅 test 可见）**

将 `src-tauri/src/llm/mod.rs` 改为：

```rust
pub mod client;
pub mod ollama;
pub mod openai;
pub mod pipeline;

#[cfg(test)]
pub mod mock_client;
```

**Step 3: 验证编译**

```bash
cd src-tauri && cargo test --lib 2>&1 | head -20
```

Expected: 编译通过，`running 0 tests`（还没写测试）

**Step 4: Commit**

```bash
git add src-tauri/src/llm/mock_client.rs src-tauri/src/llm/mod.rs
git commit -m "test: add MockLlmClient for pipeline unit tests"
```

---

## Task 3: Pipeline 单元测试

**Files:**
- Modify: `src-tauri/src/llm/pipeline.rs`（在文件末尾添加 test 模块）

**Step 1: 在 `pipeline.rs` 末尾添加测试模块**

在文件末尾（`extract_json` 函数之后）追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::mock_client::MockLlmClient;
    use std::path::Path;

    // prompts 目录相对于 src-tauri/（cargo test 的工作目录）
    fn prompts_dir() -> &'static Path {
        Path::new("../prompts")
    }

    #[test]
    fn test_stage3_returns_empty_struct_on_invalid_json() {
        let mock = MockLlmClient::new(vec!["this is not json"]);
        let pipeline = Pipeline::new(&mock, prompts_dir());
        let result = pipeline.stage3_structure("任意输入");
        assert!(result.participants.is_empty());
        assert!(result.topic.is_none());
    }

    #[test]
    fn test_stage3_parses_valid_json() {
        let json = r#"{
            "topic": "缓存方案评审",
            "participants": ["张三", "李四"],
            "key_points": ["采用 Redis"],
            "decisions": ["使用 Redis 集群"],
            "risks": []
        }"#;
        let mock = MockLlmClient::with_json(json);
        let pipeline = Pipeline::new(&mock, prompts_dir());
        let result = pipeline.stage3_structure("输入文本");
        assert_eq!(result.topic.as_deref(), Some("缓存方案评审"));
        assert_eq!(result.participants.len(), 2);
    }

    #[test]
    fn test_stage5_returns_empty_vec_on_invalid_json() {
        let mock = MockLlmClient::new(vec!["not json"]);
        let pipeline = Pipeline::new(&mock, prompts_dir());
        let result = pipeline.stage5_actions("输入文本");
        assert!(result.is_empty());
    }

    #[test]
    fn test_stage5_parses_action_items() {
        let json = r#"[
            {"task": "方案细化", "owner": "李四", "deadline": "下周五"},
            {"task": "压测", "owner": "王五", "deadline": "周三"}
        ]"#;
        let mock = MockLlmClient::with_json(json);
        let pipeline = Pipeline::new(&mock, prompts_dir());
        let result = pipeline.stage5_actions("输入文本");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].task, "方案细化");
        assert_eq!(result[0].owner.as_deref(), Some("李四"));
    }

    #[test]
    fn test_extract_json_strips_markdown_fence() {
        let input = "```json\n{\"key\":\"value\"}\n```";
        assert_eq!(extract_json(input), "{\"key\":\"value\"}");
    }

    #[test]
    fn test_extract_json_passthrough_plain() {
        let input = r#"{"key":"value"}"#;
        assert_eq!(extract_json(input), r#"{"key":"value"}"#);
    }
}
```

**Step 2: 运行测试确认全部通过**

```bash
cd src-tauri && cargo test --lib llm::pipeline::tests -- --nocapture
```

Expected:
```
test llm::pipeline::tests::test_extract_json_passthrough_plain ... ok
test llm::pipeline::tests::test_extract_json_strips_markdown_fence ... ok
test llm::pipeline::tests::test_stage3_parses_valid_json ... ok
test llm::pipeline::tests::test_stage3_returns_empty_struct_on_invalid_json ... ok
test llm::pipeline::tests::test_stage5_parses_action_items ... ok
test llm::pipeline::tests::test_stage5_returns_empty_vec_on_invalid_json ... ok
test result: ok. 6 passed
```

> **注意：** 如果提示找不到 `../prompts`，说明 cargo test 的工作目录不对。在 `src-tauri/` 下运行即可，prompts 目录在 `src-tauri/../prompts`。

**Step 3: Commit**

```bash
git add src-tauri/src/llm/pipeline.rs
git commit -m "test: add pipeline unit tests with MockLlmClient"
```

---

## Task 4: Fixture 数据模型与加载器

**Files:**
- Create: `src-tauri/src/eval/mod.rs`
- Create: `src-tauri/src/eval/fixture.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: 创建 `eval/mod.rs`**

```rust
// src-tauri/src/eval/mod.rs
pub mod fixture;
pub mod grader_code;
pub mod grader_llm;
pub mod reporter;
```

**Step 2: 创建 `eval/fixture.rs`**

```rust
// src-tauri/src/eval/fixture.rs
use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
pub struct FixtureMeta {
    pub id: String,
    pub scene: String,
    pub difficulty: String, // "normal" | "hard" | "edge"
}

#[derive(Debug, Deserialize)]
pub struct FixtureInput {
    pub raw_transcript: String,
}

#[derive(Debug, Deserialize)]
pub struct GoldenAction {
    pub task: String,
    pub owner: Option<String>,
    pub deadline: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FixtureExpected {
    #[serde(default)]
    pub summary_must_contain: Vec<String>,
    #[serde(default)]
    pub min_action_items: usize,
    #[serde(default)]
    pub required_participants: Vec<String>,
    pub golden_summary: Option<String>,
    #[serde(default)]
    pub golden_actions: Vec<GoldenAction>,
}

#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub meta: FixtureMeta,
    pub input: FixtureInput,
    pub expected: FixtureExpected,
}

impl Fixture {
    pub fn load(path: &Path) -> AppResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| AppError::Other(format!("Failed to read fixture {:?}: {}", path, e)))?;
        toml::from_str(&content)
            .map_err(|e| AppError::Other(format!("Failed to parse fixture {:?}: {}", path, e)))
    }

    pub fn load_all(dir: &Path) -> AppResult<Vec<Self>> {
        let mut fixtures = Vec::new();
        let entries = std::fs::read_dir(dir)
            .map_err(|e| AppError::Other(format!("Cannot read fixtures dir: {}", e)))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                fixtures.push(Self::load(&path)?);
            }
        }
        fixtures.sort_by(|a, b| a.meta.id.cmp(&b.meta.id));
        Ok(fixtures)
    }
}

/// Full result of running one fixture through the pipeline + graders.
#[derive(Debug, Serialize)]
pub struct EvalResult {
    pub fixture_id: String,
    pub scene: String,
    pub difficulty: String,
    pub code_score: f32,
    pub llm_score: Option<f32>,
    pub llm_reason: Option<String>,
    pub passed_checks: Vec<String>,
    pub failed_checks: Vec<String>,
    pub duration_ms: u64,
}

impl EvalResult {
    pub fn status(&self) -> &'static str {
        if self.code_score < 0.6 {
            "FAIL"
        } else if self.llm_score.map(|s| s < 0.7).unwrap_or(false) {
            "WARN"
        } else if self.code_score >= 0.8 {
            "PASS"
        } else {
            "WARN"
        }
    }
}
```

**Step 3: 在 `lib.rs` 注册 eval 模块**

在 `src-tauri/src/lib.rs` 的 `mod llm;` 下方添加：

```rust
pub mod eval;
```

**Step 4: 验证编译**

```bash
cd src-tauri && cargo build --bin eval 2>&1 | head -30
```

Expected: 报错找不到 `src/bin/eval.rs`（正常，下一步创建）

```bash
cd src-tauri && cargo check --lib 2>&1 | head -20
```

Expected: 编译通过（或仅有 `mod grader_code` 等找不到的错误，因为文件还没建）

**Step 5: Commit**

```bash
git add src-tauri/src/eval/ src-tauri/src/lib.rs
git commit -m "feat: add eval module skeleton with fixture loader"
```

---

## Task 5: 代码打分器

**Files:**
- Create: `src-tauri/src/eval/grader_code.rs`

**Step 1: 创建 `grader_code.rs`**

```rust
// src-tauri/src/eval/grader_code.rs
//! Code-based grader: fast, deterministic checks on pipeline output.
use crate::llm::pipeline::PipelineOutput;
use super::fixture::FixtureExpected;

pub struct CodeGradeResult {
    pub score: f32,
    pub passed: Vec<String>,
    pub failed: Vec<String>,
}

/// Run all code checks against pipeline output.
/// Returns weighted score 0.0~1.0.
pub fn grade(output: &PipelineOutput, expected: &FixtureExpected) -> CodeGradeResult {
    let mut checks: Vec<(&str, f32, bool)> = Vec::new(); // (name, weight, passed)

    // Check 1: stage3 participants non-empty
    let c1 = !output.structure.participants.is_empty();
    checks.push(("stage3_participants_nonempty", 0.5, c1));

    // Check 2: stage3 required participants present
    let c2 = expected.required_participants.iter().all(|p| {
        output.structure.participants.iter().any(|op| op.contains(p.as_str()))
    });
    checks.push(("stage3_required_participants", 1.0, c2));

    // Check 3: stage4 summary contains required keywords
    let c3 = expected.summary_must_contain.iter().all(|kw| {
        output.summary.contains(kw.as_str())
    });
    checks.push(("stage4_summary_keywords", 1.0, c3));

    // Check 4: stage5 action items count >= min
    let c4 = output.action_items.len() >= expected.min_action_items;
    checks.push(("stage5_min_action_items", 1.0, c4));

    // Check 5: stage6 report non-empty
    let c5 = !output.report.trim().is_empty();
    checks.push(("stage6_report_nonempty", 0.5, c5));

    // Check 6: clean transcript non-empty (stage1)
    let c6 = !output.clean_transcript.trim().is_empty();
    checks.push(("stage1_clean_nonempty", 0.5, c6));

    // Compute weighted score
    let total_weight: f32 = checks.iter().map(|(_, w, _)| w).sum();
    let passed_weight: f32 = checks.iter().filter(|(_, _, p)| *p).map(|(_, w, _)| w).sum();
    let score = if total_weight > 0.0 { passed_weight / total_weight } else { 0.0 };

    let passed = checks.iter().filter(|(_, _, p)| *p).map(|(n, _, _)| n.to_string()).collect();
    let failed = checks.iter().filter(|(_, _, p)| !p).map(|(n, _, _)| n.to_string()).collect();

    CodeGradeResult { score, passed, failed }
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check --lib 2>&1 | grep "^error" | head -10
```

Expected: 无 error（可能有 warning）

**Step 3: Commit**

```bash
git add src-tauri/src/eval/grader_code.rs
git commit -m "feat: add code-based grader for pipeline eval"
```

---

## Task 6: LLM-as-Judge 打分器

**Files:**
- Create: `src-tauri/src/eval/grader_llm.rs`

**Step 1: 创建 `grader_llm.rs`**

```rust
// src-tauri/src/eval/grader_llm.rs
//! LLM-as-Judge grader: uses a second LLM call to score summary quality.
use serde::Deserialize;
use crate::error::{AppError, AppResult};
use crate::llm::client::LlmClient;

#[derive(Debug, Deserialize)]
struct JudgeResponse {
    score: f32,
    reason: String,
}

pub struct LlmGradeResult {
    pub score: f32,
    pub reason: String,
}

/// Load judge prompt template from rubrics dir and evaluate actual vs golden summary.
pub fn grade(
    client: &dyn LlmClient,
    rubrics_dir: &std::path::Path,
    golden_summary: &str,
    actual_summary: &str,
) -> AppResult<LlmGradeResult> {
    let prompt_template = std::fs::read_to_string(rubrics_dir.join("llm_judge_prompt.txt"))
        .map_err(|e| AppError::Other(format!("Cannot read judge prompt: {}", e)))?;

    let prompt = prompt_template
        .replace("{{golden_summary}}", golden_summary)
        .replace("{{actual_summary}}", actual_summary);

    let response = client.complete(&prompt)?;

    // Strip markdown fences if present
    let json_str = strip_fences(response.trim());

    let parsed: JudgeResponse = serde_json::from_str(json_str)
        .map_err(|e| AppError::Other(format!("Judge response parse error: {}. Raw: {}", e, response)))?;

    // Clamp score to [0.0, 1.0]
    let score = parsed.score.clamp(0.0, 1.0);
    Ok(LlmGradeResult { score, reason: parsed.reason })
}

fn strip_fences(text: &str) -> &str {
    if let Some(start) = text.find("```") {
        let after = &text[start + 3..];
        let content_start = after.find('\n').map(|i| i + 1).unwrap_or(0);
        let content = &after[content_start..];
        if let Some(end) = content.rfind("```") {
            return content[..end].trim();
        }
    }
    text
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check --lib 2>&1 | grep "^error" | head -10
```

Expected: 无 error

**Step 3: Commit**

```bash
git add src-tauri/src/eval/grader_llm.rs
git commit -m "feat: add LLM-as-Judge grader"
```

---

## Task 7: Markdown 报告生成器

**Files:**
- Create: `src-tauri/src/eval/reporter.rs`

**Step 1: 创建 `reporter.rs`**

```rust
// src-tauri/src/eval/reporter.rs
use super::fixture::EvalResult;
use chrono::Local;

pub fn generate(results: &[EvalResult]) -> String {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let total = results.len();
    let passed = results.iter().filter(|r| r.status() == "PASS").count();
    let avg_llm: Option<f32> = {
        let scores: Vec<f32> = results.iter().filter_map(|r| r.llm_score).collect();
        if scores.is_empty() { None } else { Some(scores.iter().sum::<f32>() / scores.len() as f32) }
    };

    let mut out = format!("# memo-ai Eval Report — {}\n\n", date);

    // Summary table
    out.push_str("| Fixture | Scene | Code分 | LLM分 | 耗时 | 状态 |\n");
    out.push_str("|---------|-------|--------|-------|------|------|\n");
    for r in results {
        let llm_str = r.llm_score.map(|s| format!("{:.2}", s)).unwrap_or_else(|| "—".into());
        out.push_str(&format!(
            "| {} | {} | {:.2} | {} | {}ms | {} |\n",
            r.fixture_id, r.scene, r.code_score, llm_str, r.duration_ms, r.status()
        ));
    }

    // Overall stats
    out.push_str(&format!("\n**总体通过率：{}/{}（{:.0}%）**\n", passed, total, passed as f32 / total as f32 * 100.0));
    if let Some(avg) = avg_llm {
        out.push_str(&format!("**平均 LLM 评分：{:.2}**\n", avg));
    }

    // Failure details
    let failures: Vec<&EvalResult> = results.iter().filter(|r| r.status() == "FAIL" || r.status() == "WARN").collect();
    if !failures.is_empty() {
        out.push_str("\n## 未通过详情\n");
        for r in failures {
            out.push_str(&format!("\n### {} ({})\n", r.fixture_id, r.status()));
            for check in &r.failed_checks {
                out.push_str(&format!("- FAIL {}\n", check));
            }
            if let Some(ref reason) = r.llm_reason {
                out.push_str(&format!("- LLM Judge: {}\n", reason));
            }
        }
    }

    out
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo check --lib 2>&1 | grep "^error" | head -10
```

Expected: 无 error

**Step 3: Commit**

```bash
git add src-tauri/src/eval/reporter.rs
git commit -m "feat: add Markdown report generator for eval"
```

---

## Task 8: eval 二进制 CLI

**Files:**
- Create: `src-tauri/src/bin/eval.rs`

**Step 1: 创建 `src-tauri/src/bin/eval.rs`**

```rust
// src-tauri/src/bin/eval.rs
//! eval binary: run LLM pipeline against fixtures and produce quality report.
//!
//! Usage:
//!   cargo run --bin eval
//!   cargo run --bin eval -- --fixture tech_review
//!   cargo run --bin eval -- --no-llm-judge
//!   cargo run --bin eval -- --output report.md

use memo_ai_lib::{
    eval::{
        fixture::{EvalResult, Fixture},
        grader_code,
        grader_llm,
        reporter,
    },
    llm::client::{LlmClient, LlmConfig},
};
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();
    let cli = parse_args(&args);

    println!("memo-ai eval — loading fixtures...");

    // Resolve paths relative to workspace root (where Cargo.toml lives)
    let workspace_root = find_workspace_root().expect("Cannot find workspace root");
    let fixtures_dir = workspace_root.join("evals/fixtures");
    let rubrics_dir = workspace_root.join("evals/rubrics");
    let prompts_dir = workspace_root.join("prompts");

    // Load fixtures
    let mut fixtures = Fixture::load_all(&fixtures_dir)
        .expect("Failed to load fixtures");

    if let Some(ref filter) = cli.fixture_filter {
        fixtures.retain(|f| f.meta.id.contains(filter.as_str()));
        println!("Filtered to {} fixture(s) matching '{}'", fixtures.len(), filter);
    }

    if fixtures.is_empty() {
        eprintln!("No fixtures found in {:?}", fixtures_dir);
        std::process::exit(1);
    }

    // Build LLM client
    let config = load_llm_config(&cli.config_path, &workspace_root);
    let client = config.build_client();
    println!("Using LLM: {} @ {}", config.model, config.base_url);

    // Run each fixture
    let mut results: Vec<EvalResult> = Vec::new();
    for fixture in &fixtures {
        println!("\nRunning: {} ({})", fixture.meta.id, fixture.meta.scene);
        let result = run_fixture(
            fixture,
            client.as_ref(),
            &prompts_dir,
            &rubrics_dir,
            cli.no_llm_judge,
        );
        println!("  code={:.2} llm={} status={}",
            result.code_score,
            result.llm_score.map(|s| format!("{:.2}", s)).unwrap_or("—".into()),
            result.status()
        );
        results.push(result);
    }

    // Generate report
    let report = reporter::generate(&results);

    match &cli.output_path {
        Some(path) => {
            std::fs::write(path, &report).expect("Failed to write report");
            println!("\nReport saved to {:?}", path);
        }
        None => {
            println!("\n{}", report);
        }
    }

    // Exit with non-zero if any FAIL
    let has_fail = results.iter().any(|r| r.status() == "FAIL");
    if has_fail {
        std::process::exit(1);
    }
}

fn run_fixture(
    fixture: &Fixture,
    client: &dyn LlmClient,
    prompts_dir: &Path,
    rubrics_dir: &Path,
    no_llm_judge: bool,
) -> EvalResult {
    use memo_ai_lib::llm::pipeline::Pipeline;

    let start = Instant::now();
    let pipeline = Pipeline::new(client, prompts_dir);

    let output = match pipeline.run(&fixture.input.raw_transcript, false) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("  Pipeline error: {}", e);
            return EvalResult {
                fixture_id: fixture.meta.id.clone(),
                scene: fixture.meta.scene.clone(),
                difficulty: fixture.meta.difficulty.clone(),
                code_score: 0.0,
                llm_score: None,
                llm_reason: Some(format!("Pipeline error: {}", e)),
                passed_checks: vec![],
                failed_checks: vec!["pipeline_run".to_string()],
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }
    };

    // Code grading
    let code_result = grader_code::grade(&output, &fixture.expected);

    // LLM grading (only if code_score >= 0.6 and not disabled)
    let (llm_score, llm_reason) = if !no_llm_judge
        && code_result.score >= 0.6
        && fixture.expected.golden_summary.is_some()
    {
        match grader_llm::grade(
            client,
            rubrics_dir,
            fixture.expected.golden_summary.as_deref().unwrap(),
            &output.summary,
        ) {
            Ok(r) => (Some(r.score), Some(r.reason)),
            Err(e) => {
                eprintln!("  LLM judge error: {}", e);
                (None, Some(format!("Judge error: {}", e)))
            }
        }
    } else {
        (None, None)
    };

    EvalResult {
        fixture_id: fixture.meta.id.clone(),
        scene: fixture.meta.scene.clone(),
        difficulty: fixture.meta.difficulty.clone(),
        code_score: code_result.score,
        llm_score,
        llm_reason,
        passed_checks: code_result.passed,
        failed_checks: code_result.failed,
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

// --- CLI parsing ---

struct CliArgs {
    fixture_filter: Option<String>,
    no_llm_judge: bool,
    config_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> CliArgs {
    let mut cli = CliArgs {
        fixture_filter: None,
        no_llm_judge: false,
        config_path: None,
        output_path: None,
    };
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--fixture" => { i += 1; cli.fixture_filter = args.get(i).cloned(); }
            "--no-llm-judge" => { cli.no_llm_judge = true; }
            "--config" => { i += 1; cli.config_path = args.get(i).map(PathBuf::from); }
            "--output" => { i += 1; cli.output_path = args.get(i).map(PathBuf::from); }
            _ => {}
        }
        i += 1;
    }
    cli
}

fn load_llm_config(config_path: &Option<PathBuf>, workspace_root: &Path) -> LlmConfig {
    // Try explicit config path first
    if let Some(path) = config_path {
        if let Ok(s) = std::fs::read_to_string(path) {
            if let Ok(c) = serde_json::from_str::<LlmConfig>(&s) {
                return c;
            }
        }
    }
    // Try workspace root settings.json
    let settings_path = workspace_root.join("eval-config.json");
    if let Ok(s) = std::fs::read_to_string(&settings_path) {
        if let Ok(c) = serde_json::from_str::<LlmConfig>(&s) {
            return c;
        }
    }
    // Default: Ollama with llama3
    LlmConfig {
        provider: "ollama".into(),
        base_url: "http://localhost:11434".into(),
        model: "llama3".into(),
        api_key: None,
    }
}

fn find_workspace_root() -> Option<PathBuf> {
    // Walk up from current dir to find Cargo.toml with workspace, or evals/ dir
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..10 {
        if dir.join("evals").exists() {
            return Some(dir);
        }
        if !dir.pop() { break; }
    }
    // Fallback: assume we're in src-tauri/, workspace root is parent
    std::env::current_dir().ok().map(|d| d.parent().map(PathBuf::from)).flatten()
}
```

**Step 2: 验证编译**

```bash
cd src-tauri && cargo build --bin eval 2>&1 | grep "^error" | head -20
```

Expected: 无 error（可能有 warning）

**Step 3: Commit**

```bash
git add src-tauri/src/bin/eval.rs
git commit -m "feat: add eval binary CLI"
```

---

## Task 9: 创建 Fixture 语料

**Files:**
- Create: `evals/fixtures/tech_review.toml`
- Create: `evals/fixtures/requirements.toml`
- Create: `evals/fixtures/weekly_standup.toml`
- Create: `evals/fixtures/edge_empty.toml`
- Create: `evals/rubrics/llm_judge_prompt.txt`

**Step 1: 创建 `evals/fixtures/tech_review.toml`**

```toml
[meta]
id = "tech_review_01"
scene = "技术评审"
difficulty = "normal"

[input]
raw_transcript = """
张三：好，今天我们评审一下新的缓存方案，大家都看过方案文档了吗？
李四：看了，我觉得 Redis 集群方案总体可行，但需要重点考虑数据一致性问题，尤其是在节点故障时。
王五：我这边关注的是性能，我们现在的 QPS 大概是两万，Redis 应该没问题。
张三：好，那我们就决定采用 Redis 集群方案。李四，你负责把方案细化，下周五之前提交详细设计文档。
李四：没问题。
张三：王五，你这边做一下压测，周三出结果，我们好做最终决策。
王五：好的，我来安排。
张三：那今天就到这里，下周继续跟进。
"""

[expected]
summary_must_contain = ["Redis", "李四", "王五"]
min_action_items = 2
required_participants = ["张三", "李四", "王五"]
golden_summary = "会议决定采用 Redis 集群方案。李四负责方案细化，下周五前提交详细设计文档；王五负责压测，周三出结果，用于最终决策。"

[[expected.golden_actions]]
task = "Redis 方案细化，提交详细设计文档"
owner = "李四"
deadline = "下周五"

[[expected.golden_actions]]
task = "压测"
owner = "王五"
deadline = "周三"
```

**Step 2: 创建 `evals/fixtures/requirements.toml`**

```toml
[meta]
id = "requirements_01"
scene = "需求讨论"
difficulty = "normal"

[input]
raw_transcript = """
产品：这次我们要讨论用户反馈模块的需求，主要有两个方向，一个是在 App 内嵌入反馈表单，另一个是跳转到外部链接。
前端：内嵌表单对用户体验更好，但开发量大一些，大概需要两周。
后端：如果内嵌表单的话，我这边还要开一个接口存储反馈数据，大概三天的工作量。
产品：那我们先做内嵌表单，优先级 P1。前端这周开始排期，后端接口下周一前提供。
前端：收到，这周五给出详细排期。
产品：好的，这个需求就确定了。
"""

[expected]
summary_must_contain = ["反馈", "内嵌", "P1"]
min_action_items = 2
required_participants = ["产品", "前端", "后端"]
golden_summary = "确定用户反馈模块采用 App 内嵌表单方案，优先级 P1。前端本周开始排期（周五提供详细排期），后端接口下周一前提供。"

[[expected.golden_actions]]
task = "提供内嵌反馈表单详细排期"
owner = "前端"
deadline = "本周五"

[[expected.golden_actions]]
task = "开发反馈数据存储接口"
owner = "后端"
deadline = "下周一"
```

**Step 3: 创建 `evals/fixtures/weekly_standup.toml`**

```toml
[meta]
id = "weekly_standup_01"
scene = "周会"
difficulty = "normal"

[input]
raw_transcript = """
主持人：好，开始今天的周会，每个人说一下本周进展和下周计划。小明先来。
小明：本周完成了登录模块的单元测试，覆盖率从 60% 提升到 85%。下周继续做支付模块的测试。
主持人：好，小红。
小红：本周修了三个线上 Bug，其中一个是 iOS 崩溃问题，影响范围比较大，已经发了热更新。下周做性能优化专项。
主持人：好，有什么需要同步的吗？
小红：iOS 那个崩溃问题根因是第三方 SDK 版本兼容问题，建议后续升级 SDK 时都先跑一遍回归测试。
主持人：好，记一个行动项，小明负责建立 SDK 升级回归测试流程，两周内完成。
小明：没问题。
"""

[expected]
summary_must_contain = ["登录", "iOS", "崩溃", "SDK"]
min_action_items = 1
required_participants = ["小明", "小红"]
golden_summary = "小明完成登录模块测试覆盖率提升（60%→85%），下周做支付模块测试。小红修复3个Bug，含iOS崩溃热更新。行动项：小明建立SDK升级回归测试流程（两周内）。"

[[expected.golden_actions]]
task = "建立 SDK 升级回归测试流程"
owner = "小明"
deadline = "两周内"
```

**Step 4: 创建 `evals/fixtures/edge_empty.toml`**

```toml
[meta]
id = "edge_empty_01"
scene = "边界：极短转写"
difficulty = "edge"

[input]
raw_transcript = "嗯。好的。散会。"

[expected]
summary_must_contain = []
min_action_items = 0
required_participants = []
golden_summary = ""
```

**Step 5: 创建 `evals/rubrics/llm_judge_prompt.txt`**

```
你是一名专业的会议记录质量评审员。请对以下会议摘要进行客观评分。

【黄金答案】
{{golden_summary}}

【待评摘要】
{{actual_summary}}

【评分标准】
- 事实准确性（关键决策、负责人、截止日期是否正确）：40%
- 完整性（重要信息是否有遗漏）：35%
- 简洁性（是否有冗余或无关信息）：25%

请只返回 JSON，不要有其他内容：
{"score": 0.85, "reason": "简短说明评分理由，不超过50字"}
```

**Step 6: 验证 fixtures 格式正确**

创建临时测试脚本验证 TOML 能被解析（可以用 Rust test）：

```bash
cd src-tauri && cargo test --bin eval 2>&1 | head -20
```

如果 eval binary 没有 test，也可以先跑：

```bash
cd src-tauri && cargo run --bin eval -- --no-llm-judge 2>&1
```

Expected: 显示 "loading fixtures..." 并尝试运行（LLM 可能报连接错误，但 fixture 加载应成功）

**Step 7: Commit**

```bash
git add evals/
git commit -m "feat: add eval fixtures and LLM judge rubric"
```

---

## Task 10: 创建 eval-config.json 示例

**Files:**
- Create: `eval-config.json.example`（不提交实际 config，避免泄露 API key）

**Step 1: 创建示例配置**

```json
{
  "provider": "ollama",
  "base_url": "http://localhost:11434",
  "model": "llama3",
  "api_key": null
}
```

若使用 OpenAI：

```json
{
  "provider": "openai",
  "base_url": "https://api.openai.com/v1",
  "model": "gpt-4o-mini",
  "api_key": "sk-..."
}
```

**Step 2: 将 `eval-config.json` 加入 .gitignore**

在项目根目录 `.gitignore` 中添加：

```
eval-config.json
```

**Step 3: Commit**

```bash
git add eval-config.json.example .gitignore
git commit -m "docs: add eval-config.json example and gitignore"
```

---

## Task 11: 端到端冒烟测试

**Step 1: 确认 Ollama 在运行（或有 OpenAI key）**

```bash
curl http://localhost:11434/api/tags 2>&1 | head -5
```

**Step 2: 运行纯代码打分（不需要 LLM 连接）**

```bash
cd src-tauri && cargo run --bin eval -- --no-llm-judge 2>&1
```

Expected 输出示例：
```
memo-ai eval — loading fixtures...
Using LLM: llama3 @ http://localhost:11434

Running: edge_empty_01 (边界：极短转写)
  code=1.00 llm=— status=PASS

Running: requirements_01 (需求讨论)
  ...
```

**Step 3: 运行完整评估（含 LLM-as-Judge）**

```bash
cd src-tauri && cargo run --bin eval 2>&1
```

**Step 4: 输出报告到文件**

```bash
cd src-tauri && cargo run --bin eval -- --output ../eval-report.md && cat ../eval-report.md
```

**Step 5: Commit（如有修复）**

```bash
git add -A && git commit -m "fix: eval end-to-end smoke test fixes"
```

---

## 验收标准

- [ ] `cargo test --lib llm::pipeline::tests` — 6 个测试全部通过
- [ ] `cargo run --bin eval -- --no-llm-judge` — 4 个 fixture 全部加载并运行，无 panic
- [ ] `cargo run --bin eval` — 含 LLM-as-Judge 完整运行（需要 Ollama/OpenAI）
- [ ] 报告输出格式正确（表格 + 统计 + 失败详情）
- [ ] `edge_empty_01` fixture 正确通过（min_action_items=0，无关键词要求）
