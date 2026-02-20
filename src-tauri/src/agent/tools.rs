use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub type ToolResult = Result<String>;

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value; // JSON Schema
    fn execute(&self, args: Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>>;
}

pub struct ToolDispatcher {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolDispatcher {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get_tools_schema(&self) -> Value {
        let mut schemas = Vec::new();
        for tool in self.tools.values() {
            schemas.push(serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name(),
                    "description": tool.description(),
                    "parameters": tool.parameters()
                }
            }));
        }
        serde_json::json!(schemas)
    }

    pub async fn execute(&self, name: &str, args: Value) -> Result<String> {
        if let Some(tool) = self.tools.get(name) {
            tool.execute(args).await
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }
}
