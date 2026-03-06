use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start: f64,
    pub end: f64,
    pub text: String,
    pub speaker: Option<String>,
    pub confidence: Option<f64>,
}
