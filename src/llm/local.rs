use anyhow::{Context, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;
use std::pin::pin;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

pub struct LocalLlmClient {
    backend: LlamaBackend,
    model_path: String,
}

impl LocalLlmClient {
    pub fn new(model_path: &str) -> Result<Self> {
        let backend = LlamaBackend::init()
            .map_err(|e| anyhow::anyhow!("Failed to init llama backend: {:?}", e))?;

        // Verify model file exists
        if !std::path::Path::new(model_path).exists() {
            return Err(anyhow::anyhow!("Model file not found: {}", model_path));
        }

        println!("[LLM] Backend initialized (Metal GPU)");
        println!("[LLM] Model path: {}", model_path);

        Ok(Self {
            backend,
            model_path: model_path.to_string(),
        })
    }

    /// Format messages into a prompt string for the model.
    /// Uses a simple ChatML-like format.
    fn format_prompt(messages: &[Message]) -> String {
        let mut prompt = String::new();
        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    prompt.push_str(&format!("<|im_start|>system\n{}<|im_end|>\n", msg.content));
                }
                "user" => {
                    prompt.push_str(&format!("<|im_start|>user\n{}<|im_end|>\n", msg.content));
                }
                "assistant" => {
                    prompt.push_str(&format!(
                        "<|im_start|>assistant\n{}<|im_end|>\n",
                        msg.content
                    ));
                }
                _ => {}
            }
        }
        // Start assistant turn
        prompt.push_str("<|im_start|>assistant\n");
        prompt
    }

    /// Generate a response from the local model.
    /// This is a blocking operation — call from a thread, not from async directly.
    pub fn chat(&self, messages: Vec<Message>) -> Result<String> {
        let prompt = Self::format_prompt(&messages);

        // Load model with GPU offload
        let model_params = LlamaModelParams::default().with_n_gpu_layers(1000);
        let model_params = pin!(model_params);

        let model = LlamaModel::load_from_file(&self.backend, &self.model_path, &model_params)
            .map_err(|e| anyhow::anyhow!("Failed to load model: {:?}", e))?;

        // Create context
        let ctx_params =
            LlamaContextParams::default().with_n_ctx(Some(NonZeroU32::new(4096).unwrap()));

        let mut ctx = model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| anyhow::anyhow!("Failed to create context: {:?}", e))?;

        // Tokenize
        let tokens = model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| anyhow::anyhow!("Failed to tokenize: {:?}", e))?;

        // Create batch and add prompt tokens
        let mut batch = LlamaBatch::new(4096, 1);

        let last_index = (tokens.len() - 1) as i32;
        for (i, token) in (0_i32..).zip(tokens.iter()) {
            let is_last = i == last_index;
            batch
                .add(*token, i, &[0], is_last)
                .context("Failed to add token to batch")?;
        }

        // Decode prompt
        ctx.decode(&mut batch)
            .map_err(|e| anyhow::anyhow!("Failed to decode prompt: {:?}", e))?;

        // Generate tokens
        let mut output = String::new();
        let mut n_cur = batch.n_tokens();
        let n_len = n_cur + 2048; // Max generation length

        let mut decoder = encoding_rs::UTF_8.new_decoder();

        let mut sampler =
            LlamaSampler::chain_simple([LlamaSampler::dist(1234), LlamaSampler::greedy()]);

        while n_cur < n_len {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);

            // Check for end of generation
            if model.is_eog_token(token) {
                break;
            }

            // Convert token to text
            match model.token_to_piece(token, &mut decoder, true, None) {
                Ok(piece) => {
                    // Check for end-of-turn marker
                    if piece.contains("<|im_end|>") {
                        break;
                    }
                    print!("{}", piece);
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                    output.push_str(&piece);
                }
                Err(_) => break,
            }

            batch.clear();
            batch
                .add(token, n_cur, &[0], true)
                .context("Failed to add generated token")?;

            n_cur += 1;

            ctx.decode(&mut batch)
                .map_err(|e| anyhow::anyhow!("Failed to decode: {:?}", e))?;
        }

        println!(); // Newline after generation
        Ok(output)
    }
}
