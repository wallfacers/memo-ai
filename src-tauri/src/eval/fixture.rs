use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::error::{AppError, AppResult};

#[derive(Debug, Deserialize)]
pub struct FixtureMeta {
    pub id: String,
    pub scene: String,
    pub difficulty: String,
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
