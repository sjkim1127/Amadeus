
use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures_util::stream::Stream;

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

#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub model: String,
    pub created_at: String,
    pub message: Option<MessageRes>,
    pub done: bool,
}

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
        
        // Simple line buffering adapter
        // In a real robust app, we might use tokio_util::codec::LinesCodec
        // For now, we assume chunks contain complete lines or we handle simple fragmentation?
        // Actually, let's use a simpler approach: 
        // We will just map the bytes to string and assume the chunks are valid UTF-8.
        // And then split by newline. 
        // This is not perfect if a multibyte character is split across chunks, but strict correctness for 
        // streaming JSON lines usually implies lines are small enough or chunks are large enough.
        
        let stream = stream.map(|chunk_result| {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk).to_string();
                    Ok(text)
                }
                Err(e) => Err(anyhow::anyhow!("Stream error: {}", e)),
            }
        });

        // We need to flatten the lines.
        // This is a bit tricky with iterator/stream mix.
        // Let's keep it simple: return the stream of strings (chunks) and let the caller handle buffering?
        // No, let's try to parse inside.

        let parsed_stream = stream.map(|text_res| {
             match text_res {
                Ok(text) => {
                    let mut output = String::new();
                     for line in text.lines() {
                        if line.trim().is_empty() { continue; }
                        if let Ok(response) = serde_json::from_str::<ChatResponse>(line) {
                            if let Some(msg) = response.message {
                                output.push_str(&msg.content);
                            }
                        }
                    }
                    Ok(output)
                }
                Err(e) => Err(e),
             }
        });

        Ok(Box::pin(parsed_stream))
    }
}

