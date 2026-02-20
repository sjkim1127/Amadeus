use anyhow::Result;
use futures_util::stream::Stream;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

const OLLAMA_API_BASE: &str = "http://localhost:11434/api";

#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    model: String,
}

#[derive(Serialize, Debug)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub model: Option<String>,
    pub created_at: Option<String>,
    pub message: Option<MessageRes>,
    pub done: Option<bool>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct MessageRes {
    pub role: String,
    pub content: String,
}

impl OllamaClient {
    pub fn new(model_name: &str) -> Self {
        Self {
            client: Client::new(),
            model: model_name.to_string(),
        }
    }

    /// Check if Ollama is running and the model is available
    pub async fn health_check(&self) -> Result<bool> {
        let res = self
            .client
            .get(format!("{}/tags", OLLAMA_API_BASE))
            .send()
            .await;

        match res {
            Ok(r) => Ok(r.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// Non-streaming chat: send messages, get full response
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        let res = self
            .client
            .post(format!("{}/chat", OLLAMA_API_BASE))
            .json(&request)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            return Err(anyhow::anyhow!("Ollama API error: {}", error_text));
        }

        let response: ChatResponse = res.json().await?;
        match response.message {
            Some(msg) => Ok(msg.content),
            None => Err(anyhow::anyhow!("No message in Ollama response")),
        }
    }

    /// Streaming chat: returns a stream of content chunks
    #[allow(dead_code)]
    pub async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
        };

        let res = self
            .client
            .post(format!("{}/chat", OLLAMA_API_BASE))
            .json(&request)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            return Err(anyhow::anyhow!("Ollama API error: {}", error_text));
        }

        let stream = res.bytes_stream();

        let parsed_stream = stream.map(|chunk_result| match chunk_result {
            Ok(chunk) => {
                let text = String::from_utf8_lossy(&chunk).to_string();
                let mut output = String::new();
                for line in text.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    if let Ok(response) = serde_json::from_str::<ChatResponse>(line) {
                        if let Some(msg) = response.message {
                            output.push_str(&msg.content);
                        }
                    }
                }
                Ok(output)
            }
            Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
        });

        Ok(Box::pin(parsed_stream))
    }
}
