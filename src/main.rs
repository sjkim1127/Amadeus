mod agent;
mod avatar;
mod llm;
mod system;
mod ui;
mod voice;

use anyhow::Result;
use bevy::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use tokio::sync::mpsc;

use crate::agent::memory::MemoryManager;
use crate::agent::persona::Persona;
use crate::agent::tools::ToolDispatcher;
use crate::llm::local::{LocalLlmClient, Message};

// System Tools
use crate::system::browser::BrowserTool;
use crate::system::files::FileSystemTool;
use crate::system::input::InputTool;
use crate::system::screenshot::ScreenshotTool;

// Voice
use crate::voice::tts::TtsManager;

// Avatar
use crate::avatar::expression::ExpressionPlugin;
use crate::avatar::renderer::AvatarPlugin;

// UI
use crate::ui::{ChatChannel, UiPlugin};

const MODEL_PATH: &str = "model/localllm/qwen2.5-7b-instruct-q4_k_m.gguf";

fn main() {
    let (ui_tx, agent_rx) = mpsc::unbounded_channel::<String>();
    let (agent_tx, ui_rx) = mpsc::unbounded_channel::<Message>();

    // 1. Spawn Agent Core in background thread
    thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            if let Err(e) = run_agent_loop(agent_rx, agent_tx).await {
                eprintln!("Agent Loop Error: {}", e);
            }
        });
    });

    // 2. Run Bevy App (Main Thread)
    App::new()
        .add_plugins(AvatarPlugin)
        .add_plugins(ExpressionPlugin)
        .add_plugins(UiPlugin)
        .insert_resource(ChatChannel {
            tx: ui_tx,
            rx: Mutex::new(ui_rx),
        })
        .run();
}

async fn run_agent_loop(
    mut agent_rx: mpsc::UnboundedReceiver<String>,
    agent_tx: mpsc::UnboundedSender<Message>,
) -> Result<()> {
    println!("AMADEUS SYSTEM ONLINE.");

    // Initialize Memory
    let memory = MemoryManager::new("amadeus.db").await?;

    // Initialize Local LLM
    println!("[System] Loading LLM model... (this may take a moment)");
    let client = match LocalLlmClient::new(MODEL_PATH) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            let err_msg = format!("[Error] LLM init failed: {}. Chat disabled.", e);
            eprintln!("{}", err_msg);
            let _ = agent_tx.send(Message {
                role: "assistant".to_string(),
                content: err_msg,
                images: None,
            });
            // Wait for messages but respond with error
            while let Some(_) = agent_rx.recv().await {
                let _ = agent_tx.send(Message {
                    role: "assistant".to_string(),
                    content: "LLM is not loaded. Please check model path.".into(),
                    images: None,
                });
            }
            return Ok(());
        }
    };
    println!("[System] LLM ready.");

    // Initialize Persona
    let persona = Persona::amadeus();

    // Initialize Tools
    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Box::new(ScreenshotTool));
    dispatcher.register(Box::new(InputTool));
    dispatcher.register(Box::new(FileSystemTool));
    dispatcher.register(Box::new(BrowserTool));

    // Voice (We can ignore STT for UI text-only, but keeping it logic-wise if we want to restore stdin later)
    let tts = match TtsManager::new() {
        Ok(t) => Some(t),
        Err(e) => {
            println!("Voice Output Unavailable: {}", e);
            None
        }
    };

    // Load History
    let mut chat_history: Vec<Message> = memory.get_recent_history(50).await?;

    let tools_schema = dispatcher.get_tools_schema();
    let tools_prompt = format!(
        "\nYou have access to the following tools: {}\n\nTo use a tool, respond with a JSON object in this format ONLY:\n{{ \"tool\": \"tool_name\", \"args\": {{ ... }} }}\nIf you use a tool, do not write anything else.",
        tools_schema
    );
    let full_system_prompt = format!("{}{}", persona.system_prompt, tools_prompt);

    if chat_history.is_empty() {
        let sys_msg = Message {
            role: "system".to_string(),
            content: full_system_prompt.clone(),
            images: None,
        };
        memory.save_message(&sys_msg).await?;
        chat_history.push(sys_msg);
    }

    println!(
        "Amadeus ({}) is ready. (Awaiting UI Input...)",
        persona.name
    );

    // Initial greeting via UI
    let greeting = Message {
        role: "assistant".to_string(),
        content: "System online. Waiting for input...".into(),
        images: None,
    };
    let _ = agent_tx.send(greeting);

    while let Some(mut input) = agent_rx.recv().await {
        input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        // ⑤ Handle Clear Chat command from UI
        if input == "__CLEAR__" {
            chat_history.clear();
            // Re-add system prompt
            let sys_msg = Message {
                role: "system".to_string(),
                content: full_system_prompt.clone(),
                images: None,
            };
            chat_history.push(sys_msg);
            let _ = agent_tx.send(Message {
                role: "assistant".to_string(),
                content: "대화 기록이 초기화되었습니다.".into(),
                images: None,
            });
            continue;
        }

        let user_msg = Message {
            role: "user".to_string(),
            content: input.to_string(),
            images: None,
        };
        memory.save_message(&user_msg).await?;
        chat_history.push(user_msg);

        // --- Chat Loop ---
        loop {
            // Run LLM inference with streaming — each token is sent to UI in real-time
            let messages_clone = chat_history.clone();
            let client_clone = Arc::clone(&client);

            let full_response = tokio::task::spawn_blocking(move || {
                client_clone.chat_streaming(messages_clone, |_piece| {
                    // Token arrives — could send incremental updates here
                })
            })
            .await??;

            let assistant_msg = Message {
                role: "assistant".to_string(),
                content: full_response.clone(),
                images: None,
            };
            memory.save_message(&assistant_msg).await?;
            chat_history.push(assistant_msg.clone());
            let _ = agent_tx.send(assistant_msg); // Send completed message to UI

            // TTS
            if let Some(tts_manager) = &tts {
                if !full_response.trim().starts_with('{') {
                    // Start lipsync if we had event trigger.
                    // To do it cleanly we'd need Bevy events handle back to the main thread.
                    let _ = tts_manager.speak(&full_response);
                }
            }

            // Tool Call Check
            let maybe_tool_call: Option<serde_json::Value> =
                serde_json::from_str(&full_response).ok();

            if let Some(tool_json) = maybe_tool_call {
                if let (Some(tool_name), Some(args)) = (
                    tool_json.get("tool").and_then(|v| v.as_str()),
                    tool_json.get("args"),
                ) {
                    println!("[System] Detected tool call: {}", tool_name);

                    // ④ Send tool status to UI as system message
                    let _ = agent_tx.send(Message {
                        role: "system".to_string(),
                        content: format!("Tool '{}' を実行中...", tool_name),
                        images: None,
                    });

                    match dispatcher.execute(tool_name, args.clone()).await {
                        Ok(result) => {
                            // Send tool result to UI
                            let _ = agent_tx.send(Message {
                                role: "system".to_string(),
                                content: format!("✅ Tool '{}' 완료", tool_name),
                                images: None,
                            });
                            let result_msg = Message {
                                role: "user".to_string(),
                                content: format!("Tool Output: {}", result),
                                images: None,
                            };
                            memory.save_message(&result_msg).await?;
                            chat_history.push(result_msg);
                            continue;
                        }
                        Err(e) => {
                            let _ = agent_tx.send(Message {
                                role: "system".to_string(),
                                content: format!("❌ Tool '{}' 오류: {}", tool_name, e),
                                images: None,
                            });
                            let error_msg = Message {
                                role: "user".to_string(),
                                content: format!("Tool Error: {}", e),
                                images: None,
                            };
                            memory.save_message(&error_msg).await?;
                            chat_history.push(error_msg);
                            continue;
                        }
                    }
                }
            }
            break;
        }
    }
    Ok(())
}
