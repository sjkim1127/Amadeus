use anyhow::Result;
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

use crate::agent::tools::{Tool, ToolResult};

// Singleton browser instance logic would be better, but for simplicity we spin up for now
// Or we can keep a static/shared reference if we want persistence.
// For Phase 2, let's try to launch a headless browser each time? No, that's slow.
// We need a shared browser manager. But `Tool` trait is stateless.
// We'll wrap the browser in a lazy generic or pass it in.
// For now, let's make it launch on demand, but note performance hit.

pub struct BrowserTool;

impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser_automation"
    }

    fn description(&self) -> &str {
        "Automate web browser. Actions: 'navigate'. (Note: Starts a new browser instance per call for now)"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["navigate"]
                },
                "url": { "type": "string", "description": "URL to navigate to" }
            },
            "required": ["action", "url"]
        })
    }

    fn execute(&self, args: Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> {
        Box::pin(async move {
            let action = args["action"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing action"))?;
            let url = args["url"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing URL"))?;

            if action != "navigate" {
                return Err(anyhow::anyhow!("Unknown action: {}", action));
            }

            // Launch browser (Headless)
            let (mut browser, mut handler) = Browser::launch(
                BrowserConfig::builder()
                    .with_head() // Ensure user sees it
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build browser config: {}", e))?,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to launch browser: {}", e))?;

            // Spawn the handler loop
            let handle = tokio::spawn(async move {
                while let Some(h) = handler.next().await {
                    if h.is_err() {
                        break;
                    }
                }
            });

            let page = browser
                .new_page(url)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to create page: {}", e))?;

            // Wait for load?
            // content() waits for network idle usually? No, it just dumps DOM.
            // Let's wait a bit or wait for element?
            // Simple approach: just get content.

            let content = page
                .content()
                .await
                .map_err(|e| anyhow::anyhow!("Content failed: {}", e))?;
            let title = page.get_title().await.ok().flatten().unwrap_or_default();

            browser
                .close()
                .await
                .map_err(|e| anyhow::anyhow!("Close failed: {}", e))?;
            let _ = handle.await;

            let summary = format!("Title: {}\nContent Length: {} chars", title, content.len());
            Ok(summary)
        })
    }
}
