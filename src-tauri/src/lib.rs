mod agent;
mod llm;
mod system;
mod voice;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{mpsc, Mutex};

use crate::agent::memory::MemoryManager;
use crate::agent::persona::Persona;
use crate::agent::tools::ToolDispatcher;
use crate::llm::local::{LocalLlmClient, Message};

use crate::system::browser::BrowserTool;
use crate::system::files::FileSystemTool;
use crate::system::input::InputTool;
use crate::system::screenshot::ScreenshotTool;

use crate::voice::tts::TtsManager;

const MODEL_PATH: &str = "model/localllm/qwen2.5-7b-instruct-q4_k_m.gguf";

// ===== Tauri State =====

pub struct AppState {
    pub tx: mpsc::UnboundedSender<String>,
}

// ===== Events sent to frontend =====

#[derive(Clone, Serialize)]
struct ChatEvent {
    role: String,
    content: String,
}

#[derive(Clone, Serialize)]
struct StatusEvent {
    status: String,
    is_thinking: bool,
}

// ===== Tauri Commands =====

#[tauri::command]
async fn send_message(
    state: tauri::State<'_, Arc<Mutex<AppState>>>,
    message: String,
) -> Result<(), String> {
    let state = state.lock().await;
    state
        .tx
        .send(message)
        .map_err(|e| format!("Failed to send message: {}", e))
}

#[tauri::command]
async fn clear_chat(state: tauri::State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let state = state.lock().await;
    state
        .tx
        .send("__CLEAR__".to_string())
        .map_err(|e| format!("Failed to send clear: {}", e))
}

// ===== Agent Loop =====

async fn run_agent_loop(
    app: AppHandle,
    mut agent_rx: mpsc::UnboundedReceiver<String>,
) -> Result<()> {
    println!("AMADEUS SYSTEM ONLINE.");

    // Helper to emit chat messages to frontend
    let emit_chat = |app: &AppHandle, role: &str, content: &str| {
        let _ = app.emit(
            "chat-message",
            ChatEvent {
                role: role.to_string(),
                content: content.to_string(),
            },
        );
    };

    let emit_status = |app: &AppHandle, status: &str, is_thinking: bool| {
        let _ = app.emit(
            "chat-status",
            StatusEvent {
                status: status.to_string(),
                is_thinking,
            },
        );
    };

    // Initialize Memory
    let memory = MemoryManager::new("amadeus.db").await?;

    // Initialize Local LLM
    println!("[System] Loading LLM model... (this may take a moment)");
    emit_status(&app, "Loading LLM model...", true);

    let client = match LocalLlmClient::new(MODEL_PATH) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            let err_msg = format!("[Error] LLM init failed: {}. Chat disabled.", e);
            eprintln!("{}", err_msg);
            emit_chat(&app, "assistant", &err_msg);
            emit_status(&app, "LLM Error", false);

            while let Some(_) = agent_rx.recv().await {
                emit_chat(
                    &app,
                    "assistant",
                    "LLM is not loaded. Please check model path.",
                );
            }
            return Ok(());
        }
    };
    println!("[System] LLM ready.");
    emit_status(&app, "Online", false);

    // Initialize Persona
    let persona = Persona::amadeus();

    // Initialize Tools
    let mut dispatcher = ToolDispatcher::new();
    dispatcher.register(Box::new(ScreenshotTool));
    dispatcher.register(Box::new(InputTool));
    dispatcher.register(Box::new(FileSystemTool));
    dispatcher.register(Box::new(BrowserTool));

    // Voice
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

    // Initial greeting
    emit_chat(&app, "assistant", "System online. Waiting for input...");

    while let Some(mut input) = agent_rx.recv().await {
        input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }

        // Handle Clear Chat
        if input == "__CLEAR__" {
            chat_history.clear();
            let sys_msg = Message {
                role: "system".to_string(),
                content: full_system_prompt.clone(),
                images: None,
            };
            chat_history.push(sys_msg);
            emit_chat(&app, "assistant", "대화 기록이 초기화되었습니다.");
            continue;
        }

        // User message
        let user_msg = Message {
            role: "user".to_string(),
            content: input.to_string(),
            images: None,
        };
        memory.save_message(&user_msg).await?;
        chat_history.push(user_msg);

        emit_status(&app, "Thinking", true);

        // Chat Loop
        loop {
            let messages_clone = chat_history.clone();
            let client_clone = Arc::clone(&client);

            let full_response = tokio::task::spawn_blocking(move || {
                client_clone.chat_streaming(messages_clone, |_piece| {})
            })
            .await??;

            let assistant_msg = Message {
                role: "assistant".to_string(),
                content: full_response.clone(),
                images: None,
            };
            memory.save_message(&assistant_msg).await?;
            chat_history.push(assistant_msg);
            emit_chat(&app, "assistant", &full_response);
            emit_status(&app, "Online", false);

            // TTS
            if let Some(tts_manager) = &tts {
                if !full_response.trim().starts_with('{') {
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
                    emit_chat(&app, "system", &format!("Tool '{}' を実行中...", tool_name));
                    emit_status(&app, &format!("Running tool: {}", tool_name), true);

                    match dispatcher.execute(tool_name, args.clone()).await {
                        Ok(result) => {
                            emit_chat(&app, "system", &format!("✅ Tool '{}' 완료", tool_name));
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
                            emit_chat(
                                &app,
                                "system",
                                &format!("❌ Tool '{}' 오류: {}", tool_name, e),
                            );
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

// ===== Tauri Entry Point =====

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let (tx, rx) = mpsc::unbounded_channel::<String>();

            let state = Arc::new(Mutex::new(AppState { tx }));
            app.manage(state);

            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                if let Err(e) = run_agent_loop(app_handle, rx).await {
                    eprintln!("Agent Loop Error: {}", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![send_message, clear_chat])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
