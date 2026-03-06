use crate::error::AppResult;

/// Abstraction over different LLM providers.
/// Both Ollama and OpenAI implement this trait.
pub trait LlmClient: Send + Sync {
    /// Send a prompt and return the completion text.
    fn complete(&self, prompt: &str) -> AppResult<String>;

    /// Return the provider name for logging.
    fn provider_name(&self) -> &str;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LlmConfig {
    pub provider: String, // "ollama" | "openai"
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl LlmConfig {
    pub fn build_client(&self) -> Box<dyn LlmClient> {
        match self.provider.as_str() {
            "openai" => Box::new(super::openai::OpenAiClient::new(
                self.base_url.clone(),
                self.model.clone(),
                self.api_key.clone().unwrap_or_default(),
            )),
            _ => Box::new(super::ollama::OllamaClient::new(
                self.base_url.clone(),
                self.model.clone(),
            )),
        }
    }
}
