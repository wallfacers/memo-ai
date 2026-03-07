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

    let json_str = strip_fences(response.trim());

    let parsed: JudgeResponse = serde_json::from_str(json_str)
        .map_err(|e| AppError::Other(format!("Judge response parse error: {}. Raw: {}", e, response)))?;

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
