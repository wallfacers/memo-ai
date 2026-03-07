use serde::{Deserialize, Serialize};
use crate::error::{AppError, AppResult};
use super::client::LlmClient;

pub struct OpenAiClient {
    base_url: String,
    model: String,
    api_key: String,
    http: reqwest::blocking::Client,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResp,
}

#[derive(Deserialize)]
struct ChatMessageResp {
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Serialize)]
struct ChatStreamRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
}

impl OpenAiClient {
    pub fn new(base_url: String, model: String, api_key: String) -> Self {
        OpenAiClient {
            base_url,
            model,
            api_key,
            http: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }
}

impl LlmClient for OpenAiClient {
    fn complete(&self, prompt: &str) -> AppResult<String> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let req = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage { role: "user", content: prompt }],
        };
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .map_err(|e| AppError::Llm(format!("OpenAI request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(AppError::Llm(format!(
                "OpenAI returned status {}: {}",
                status, body
            )));
        }

        let body: ChatResponse = resp
            .json()
            .map_err(|e| AppError::Llm(format!("Failed to parse OpenAI response: {}", e)))?;

        body.choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| AppError::Llm("Empty choices in OpenAI response".into()))
    }

    fn complete_streaming(
        &self,
        prompt: &str,
        on_token: Box<dyn Fn(&str) + Send>,
    ) -> AppResult<String> {
        use std::io::BufRead;

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let req = ChatStreamRequest {
            model: &self.model,
            messages: vec![ChatMessage { role: "user", content: prompt }],
            stream: true,
        };
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .map_err(|e| AppError::Llm(format!("OpenAI streaming request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(AppError::Llm(format!(
                "OpenAI returned status {}: {}",
                status, body
            )));
        }

        let mut full_text = String::new();
        let reader = std::io::BufReader::new(resp);
        for line in reader.lines() {
            let line = line.map_err(|e| AppError::Llm(format!("Stream read error: {}", e)))?;
            if line.is_empty() || line == "data: [DONE]" {
                continue;
            }
            let data = line.strip_prefix("data: ").unwrap_or(&line);
            let chunk: StreamChunk = match serde_json::from_str(data) {
                Ok(c) => c,
                Err(_) => continue,
            };
            if let Some(choice) = chunk.choices.into_iter().next() {
                if let Some(content) = choice.delta.content {
                    if !content.is_empty() {
                        on_token(&content);
                        full_text.push_str(&content);
                    }
                }
            }
        }
        Ok(full_text)
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}
