use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};
use super::client::LlmClient;

/// 阶段完成回调：(stage编号 1-6, 阶段名称, 结果摘要)
pub type StageCallback = Box<dyn Fn(u8, &str, &str) + Send>;

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
    pub generated_title: Option<String>,
}

pub struct Pipeline<'a> {
    client: &'a dyn LlmClient,
    prompts_dir: &'a Path,
    on_stage_done: Option<StageCallback>,
}

impl<'a> Pipeline<'a> {
    pub fn new(client: &'a dyn LlmClient, prompts_dir: &'a Path) -> Self {
        Pipeline { client, prompts_dir, on_stage_done: None }
    }

    pub fn with_callback(mut self, cb: StageCallback) -> Self {
        self.on_stage_done = Some(cb);
        self
    }

    fn notify_stage(&self, stage: u8, name: &str, summary: &str) {
        if let Some(ref cb) = self.on_stage_done {
            cb(stage, name, summary);
        }
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

    /// Stage 3: Extract structured info (JSON). Falls back to empty struct on parse failure.
    pub fn stage3_structure(&self, meeting_text: &str) -> StructuredMeeting {
        let result = (|| -> AppResult<StructuredMeeting> {
            let template = self.load_prompt("03_structure.txt")?;
            let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
            log::info!("Running pipeline stage 3: structure extraction");
            let response = self.client.complete(&prompt)?;
            let json_str = extract_json(&response);
            serde_json::from_str(json_str)
                .map_err(|e| AppError::Llm(format!("Stage 3 JSON parse failed: {}. Raw: {}", e, response)))
        })();

        match result {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Stage 3 failed, using empty structure: {}", e);
                StructuredMeeting {
                    topic: None,
                    participants: vec![],
                    key_points: vec![],
                    decisions: vec![],
                    risks: vec![],
                }
            }
        }
    }

    /// Stage 4: Generate meeting summary.
    pub fn stage4_summary(&self, meeting_text: &str) -> AppResult<String> {
        let template = self.load_prompt("04_summary.txt")?;
        let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
        log::info!("Running pipeline stage 4: summary");
        self.client.complete(&prompt)
    }

    /// Stage 5: Extract action items (JSON array). Falls back to empty vec on parse failure.
    pub fn stage5_actions(&self, meeting_text: &str) -> Vec<ActionItemRaw> {
        let result = (|| -> AppResult<Vec<ActionItemRaw>> {
            let template = self.load_prompt("05_actions.txt")?;
            let prompt = Self::fill_template(&template, "meeting_text", meeting_text);
            log::info!("Running pipeline stage 5: action items");
            let response = self.client.complete(&prompt)?;
            let json_str = extract_json(&response);
            serde_json::from_str(json_str)
                .map_err(|e| AppError::Llm(format!("Stage 5 JSON parse failed: {}. Raw: {}", e, response)))
        })();

        match result {
            Ok(items) => items,
            Err(e) => {
                log::warn!("Stage 5 failed, returning empty action items: {}", e);
                vec![]
            }
        }
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

    /// Stage 7: Generate meeting title from summary.
    pub fn stage7_title(&self, summary: &str) -> AppResult<String> {
        let template = self.load_prompt("07_title.txt")?;
        let prompt = Self::fill_template(&template, "summary", summary);
        log::info!("Running pipeline stage 7: title generation");
        let title = self.client.complete(&prompt)?;
        Ok(title.trim().to_string())
    }

    /// Run the full pipeline (stages 1-6, plus optional stage 7 if auto_titled).
    pub fn run(&self, raw_transcript: &str, auto_titled: bool) -> AppResult<PipelineOutput> {
        let clean = self.stage1_clean(raw_transcript)?;
        self.notify_stage(1, "文本清洗", &format!("完成（共 {} 字）", clean.len()));

        let organized = self.stage2_speaker(&clean)?;
        let s2_preview = organized.chars().take(50).collect::<String>();
        self.notify_stage(2, "说话人整理", &s2_preview);

        let structure = self.stage3_structure(&organized);
        let s3_summary = format!(
            "主题：{} · 参会 {} 人 · {} 项决策",
            structure.topic.as_deref().unwrap_or("未知"),
            structure.participants.len(),
            structure.decisions.len(),
        );
        self.notify_stage(3, "结构化提取", &s3_summary);

        let summary = self.stage4_summary(&organized)?;
        let s4_preview = summary.chars().take(100).collect::<String>();
        self.notify_stage(4, "会议总结", &s4_preview);

        let action_items = self.stage5_actions(&organized);
        self.notify_stage(5, "行动项提取", &format!("共 {} 项行动", action_items.len()));

        let actions_json = serde_json::to_string(&action_items)?;
        let report = self.stage6_report(&summary, &actions_json)?;
        self.notify_stage(6, "报告生成", "报告已生成，点击查看");

        let generated_title = if auto_titled {
            match self.stage7_title(&summary) {
                Ok(t) => Some(t),
                Err(e) => {
                    log::warn!("Stage 7 title generation failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(PipelineOutput {
            clean_transcript: clean,
            structure,
            summary,
            action_items,
            report,
            generated_title,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::mock_client::MockLlmClient;
    use std::path::Path;

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
