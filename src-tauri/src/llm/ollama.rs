use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};
use super::client::LlmClient;

pub struct OllamaClient {
    base_url: String,
    model: String,
    http: reqwest::blocking::Client,
}

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Deserialize)]
struct OllamaStreamChunk {
    response: String,
    done: bool,
}

impl OllamaClient {
    pub fn new(base_url: String, model: String) -> Self {
        OllamaClient {
            base_url,
            model,
            http: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

impl LlmClient for OllamaClient {
    fn complete(&self, prompt: &str) -> AppResult<String> {
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let req = OllamaRequest {
            model: &self.model,
            prompt,
            stream: false,
        };
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .map_err(|e| AppError::Llm(format!("Ollama request failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AppError::Llm(format!(
                "Ollama returned status {}",
                resp.status()
            )));
        }

        let body: OllamaResponse = resp
            .json()
            .map_err(|e| AppError::Llm(format!("Failed to parse Ollama response: {}", e)))?;
        Ok(body.response)
    }

    fn complete_streaming(
        &self,
        prompt: &str,
        on_token: Box<dyn Fn(&str) + Send>,
    ) -> AppResult<String> {
        use std::io::BufRead;

        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let req = OllamaRequest {
            model: &self.model,
            prompt,
            stream: true,
        };
        let resp = self
            .http
            .post(&url)
            .json(&req)
            .send()
            .map_err(|e| AppError::Llm(format!("Ollama streaming request failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(AppError::Llm(format!(
                "Ollama returned status {}",
                resp.status()
            )));
        }

        let mut full_text = String::new();
        let reader = std::io::BufReader::new(resp);
        for line in reader.lines() {
            let line = line.map_err(|e| AppError::Llm(format!("Stream read error: {}", e)))?;
            if line.is_empty() {
                continue;
            }
            let chunk: OllamaStreamChunk = serde_json::from_str(&line)
                .map_err(|e| AppError::Llm(format!("Failed to parse stream chunk: {}", e)))?;
            if !chunk.response.is_empty() {
                on_token(&chunk.response);
                full_text.push_str(&chunk.response);
            }
            if chunk.done {
                break;
            }
        }
        Ok(full_text)
    }

    fn provider_name(&self) -> &str {
        "ollama"
    }
}
