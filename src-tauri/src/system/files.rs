use serde_json::{json, Value};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::fs;

use crate::agent::tools::{Tool, ToolResult};

pub struct FileSystemTool;

impl FileSystemTool {
    /// Validate that the given path is within the allowed workspace.
    /// Prevents LLM from accessing sensitive system files like ~/.ssh, /etc, etc.
    fn validate_path(path_str: &str) -> Result<PathBuf, anyhow::Error> {
        let workspace_root = std::env::current_dir()?;

        let requested = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            workspace_root.join(path_str)
        };

        // Canonicalize to resolve "..", symlinks, etc.
        // For new files (write_file), parent must exist and be in workspace
        let canonical = if requested.exists() {
            requested.canonicalize()?
        } else {
            // For files that don't exist yet, validate the parent directory
            let parent = requested
                .parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid path: no parent directory"))?;
            if !parent.exists() {
                return Err(anyhow::anyhow!(
                    "Parent directory does not exist: {}",
                    parent.display()
                ));
            }
            let canonical_parent = parent.canonicalize()?;
            canonical_parent.join(requested.file_name().unwrap_or_default())
        };

        let canonical_root = workspace_root.canonicalize()?;

        if !canonical.starts_with(&canonical_root) {
            return Err(anyhow::anyhow!(
                "Access denied: path '{}' is outside the workspace ({})",
                path_str,
                canonical_root.display()
            ));
        }

        Ok(canonical)
    }
}

impl Tool for FileSystemTool {
    fn name(&self) -> &str {
        "file_system"
    }

    fn description(&self) -> &str {
        "Access file system (sandboxed to project directory). Actions: 'read_file', 'write_file', 'list_dir'."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["read_file", "write_file", "list_dir"]
                },
                "path": { "type": "string", "description": "File or directory path (relative to project root)" },
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
            let path_str = args["path"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing path"))?;

            // Sandbox validation
            let safe_path = FileSystemTool::validate_path(path_str)?;

            match action {
                "read_file" => {
                    let content = fs::read_to_string(&safe_path).await?;
                    // Truncate very long files to prevent context overflow
                    if content.len() > 10000 {
                        Ok(format!(
                            "{}...\n\n[Truncated: {} total chars]",
                            &content[..10000],
                            content.len()
                        ))
                    } else {
                        Ok(content)
                    }
                }
                "write_file" => {
                    let content = args["content"].as_str().unwrap_or("");
                    fs::write(&safe_path, content).await?;
                    Ok(format!("Successfully wrote to {}", safe_path.display()))
                }
                "list_dir" => {
                    let mut entries = fs::read_dir(&safe_path).await?;
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
