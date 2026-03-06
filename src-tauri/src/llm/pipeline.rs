use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};
use super::client::LlmClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct StructuredMeeting {
    pub topic: Option<String>,
    pub participants: Vec<String>,
    pub key_points: Vec<String>,
    pub decisions: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionItemRaw {
    pub task: String,
    pub owner: Option<String>,
    pub deadline: Option<String>,
}

#[derive(Debug)]
pub struct PipelineOutput {
    pub clean_transcript: String,
    pub structure: StructuredMeeting,
    pub summary: String,
    pub action_items: Vec<ActionItemRaw>,
    pub report: String,
}

pub struct Pipeline<'a> {
    client: &'a dyn LlmClient,
    prompts_dir: &'a Path,
}

impl<'a> Pipeline<'a> {
    pub fn new(client: &'a dyn LlmClient, prompts_dir: &'a Path) -> Self {
        Pipeline { client, prompts_dir }
    }

    fn load_prompt(&self, filename: &str) -> AppResult<String> {
        let path = self.prompts_dir.join(filename);
        std::fs::read_to_string(&path)
            .map_err(|e| AppError::Other(format!("Failed to load prompt {}: {}", filename, e)))
    }

    fn fill_template(template: &str, var: &str, value: &str) -> String {
        template.replace(&format!("{{{{{}}}}}", var), value)
    }

    /// Stage 1: Clean the raw ASR transcript.
    pub fn stage1_clean(&self, raw_transcript: &str) -> AppResult<String> {
        let template = self.load_prompt("01_clean.txt")?;
        let prompt = Self::fill_template(&template, "transcript", raw_transcript);
        log::info!("Running pipeline stage 1: clean");
        self.client.complete(&prompt)
    }

    /// Stage 2: Organize speakers.
    pub fn stage2_speaker(&self, clean_transcript: &str) -> AppResult<String> {
        let template = self.load_prompt("02_speaker.txt")?;
        let prompt = Self::fill_template(&template, "transcript", clean_transcript);
        log::info!("Running pipeline stage 2: speaker organize");
        self.client.complete(&prompt)
    }

    /// Stage 3: Extract structured info (JSON).
    pub fn stage3_structure(&self, meeting_text: &str) -> AppResult<StructuredMeeting> {
        let template = self.load_prompt("03_structure.txt")?;
        let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
        log::info!("Running pipeline stage 3: structure extraction");
        let response = self.client.complete(&prompt)?;
        // Extract JSON from response (LLM may wrap in markdown code blocks)
        let json_str = extract_json(&response);
        serde_json::from_str(json_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse structure JSON: {}. Response: {}", e, response)))
    }

    /// Stage 4: Generate meeting summary.
    pub fn stage4_summary(&self, meeting_text: &str) -> AppResult<String> {
        let template = self.load_prompt("04_summary.txt")?;
        let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
        log::info!("Running pipeline stage 4: summary");
        self.client.complete(&prompt)
    }

    /// Stage 5: Extract action items (JSON array).
    pub fn stage5_actions(&self, meeting_text: &str) -> AppResult<Vec<ActionItemRaw>> {
        let template = self.load_prompt("05_actions.txt")?;
        let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
        log::info!("Running pipeline stage 5: action items");
        let response = self.client.complete(&prompt)?;
        let json_str = extract_json(&response);
        serde_json::from_str(json_str)
            .map_err(|e| AppError::Llm(format!("Failed to parse action items JSON: {}. Response: {}", e, response)))
    }

    /// Stage 6: Generate final report.
    pub fn stage6_report(&self, summary: &str, actions_json: &str) -> AppResult<String> {
        let template = self.load_prompt("06_report.txt")?;
        let prompt = Self::fill_template(
            &Self::fill_template(&template, "summary", summary),
            "actions",
            actions_json,
        );
        log::info!("Running pipeline stage 6: report generation");
        self.client.complete(&prompt)
    }

    /// Run the full 6-stage pipeline.
    pub fn run(&self, raw_transcript: &str) -> AppResult<PipelineOutput> {
        let clean = self.stage1_clean(raw_transcript)?;
        let organized = self.stage2_speaker(&clean)?;
        let structure = self.stage3_structure(&organized)?;
        let summary = self.stage4_summary(&organized)?;
        let action_items = self.stage5_actions(&organized)?;
        let actions_json = serde_json::to_string(&action_items)?;
        let report = self.stage6_report(&summary, &actions_json)?;

        Ok(PipelineOutput {
            clean_transcript: clean,
            structure,
            summary,
            action_items,
            report,
        })
    }
}

/// Extract JSON content from LLM response, stripping markdown code fences if present.
fn extract_json(text: &str) -> &str {
    let text = text.trim();
    // Strip ```json ... ``` or ``` ... ```
    if let Some(start) = text.find("```") {
        let after_fence = &text[start + 3..];
        let content_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        let content = &after_fence[content_start..];
        if let Some(end) = content.rfind("```") {
            return content[..end].trim();
        }
    }
    text
}
