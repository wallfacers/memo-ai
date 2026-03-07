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
        if self.responses.is_empty() {
            return Err(crate::error::AppError::Other(
                "MockLlmClient has no responses configured".to_string()
            ));
        }
        let count = self.call_count.fetch_add(1, Ordering::SeqCst);
        let idx = count % self.responses.len();
        Ok(self.responses[idx].clone())
    }

    fn complete_streaming(
        &self,
        _prompt: &str,
        on_token: Box<dyn Fn(&str) + Send>,
    ) -> AppResult<String> {
        let response = self.complete(_prompt)?;
        on_token(&response);
        Ok(response)
    }

    fn provider_name(&self) -> &str {
        "mock"
    }
}
