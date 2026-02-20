use anyhow::Result;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use tokio::fs;

use crate::agent::tools::{Tool, ToolResult};

pub struct FileSystemTool;

impl Tool for FileSystemTool {
    fn name(&self) -> &str {
        "file_system"
    }

    fn description(&self) -> &str {
        "Access file system. Actions: 'read_file', 'write_file', 'list_dir'. Paths must be absolute or relative to project root."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read_file", "write_file", "list_dir"]
                },
                "path": { "type": "string", "description": "File or directory path" },
                "content": { "type": "string", "description": "Content to write (for write_file)" }
            },
            "required": ["action", "path"]
        })
    }

    fn execute(&self, args: Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> {
        Box::pin(async move {
            let action = args["action"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing action"))?;
            let path = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing path"))?;

            match action {
                "read_file" => {
                    let content = fs::read_to_string(path).await?;
                    // Truncate if too long? For now, return full content.
                    Ok(content)
                }
                "write_file" => {
                    let content = args["content"].as_str().unwrap_or("");
                    fs::write(path, content).await?;
                    Ok(format!("Successfully wrote to {}", path))
                }
                "list_dir" => {
                    let mut entries = fs::read_dir(path).await?;
                    let mut listing = String::new();
                    while let Some(entry) = entries.next_entry().await? {
                        let path = entry.path();
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        let is_dir = path.is_dir();
                        listing.push_str(&format!("{}{}\n", name, if is_dir { "/" } else { "" }));
                    }
                    Ok(listing)
                }
                _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
            }
        })
    }
}
