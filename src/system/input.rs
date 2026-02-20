use anyhow::Result;
use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;

use crate::agent::tools::{Tool, ToolResult};

pub struct InputTool;

impl Tool for InputTool {
    fn name(&self) -> &str {
        "input_control"
    }

    fn description(&self) -> &str {
        "Control keyboard and mouse. Actions: 'type', 'key_click', 'mouse_move', 'mouse_click', 'scroll'."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["type", "key_click", "mouse_move", "mouse_click", "scroll"]
                },
                "text": { "type": "string", "description": "Text to type" },
                "key": { "type": "string", "description": "Key to click (e.g., 'Return', 'Tab', 'Space')" },
                "x": { "type": "integer", "description": "Mouse X coordinate" },
                "y": { "type": "integer", "description": "Mouse Y coordinate" },
                "button": { "type": "string", "enum": ["left", "right", "middle"] },
                "scroll_x": { "type": "integer" },
                "scroll_y": { "type": "integer" }
            },
            "required": ["action"]
        })
    }

    fn execute(&self, args: Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> {
        Box::pin(async move {
            let action = args["action"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing action"))?;

            // Enigo 0.6.1 initialization
            let mut enigo = Enigo::new(&Settings::default())?;

            match action {
                "type" => {
                    let text = args["text"].as_str().unwrap_or("");
                    enigo.text(text)?;
                    Ok(format!("Typed: {}", text))
                }
                "key_click" => {
                    let key_str = args["key"].as_str().unwrap_or("");
                    // Map string to enigo Key enum
                    let key = match key_str.to_lowercase().as_str() {
                        "return" | "enter" => Key::Return,
                        "tab" => Key::Tab,
                        "space" => Key::Space,
                        "backspace" => Key::Backspace,
                        "escape" => Key::Escape,
                        _ => {
                            if let Some(c) = key_str.chars().next() {
                                Key::Unicode(c)
                            } else {
                                return Err(anyhow::anyhow!("Unknown key: {}", key_str));
                            }
                        }
                    };
                    enigo.key(key, Direction::Click)?;
                    Ok(format!("Clicked key: {}", key_str))
                }
                "mouse_move" => {
                    let x = args["x"].as_i64().unwrap_or(0) as i32;
                    let y = args["y"].as_i64().unwrap_or(0) as i32;
                    enigo.move_mouse(x, y, Coordinate::Abs)?;
                    Ok(format!("Moved mouse to {}, {}", x, y))
                }
                "mouse_click" => {
                    let button = args["button"].as_str().unwrap_or("left");
                    let btn = match button {
                        "right" => Button::Right,
                        "middle" => Button::Middle,
                        _ => Button::Left,
                    };
                    enigo.button(btn, Direction::Click)?;
                    Ok(format!("Clicked {} mouse button", button))
                }
                "scroll" => {
                    let x = args["scroll_x"].as_i64().unwrap_or(0) as i32;
                    let y = args["scroll_y"].as_i64().unwrap_or(0) as i32;
                    if x != 0 {
                        enigo.scroll(x, Axis::Horizontal)?;
                    }
                    if y != 0 {
                        enigo.scroll(y, Axis::Vertical)?;
                    }
                    Ok(format!("Scrolled {}, {}", x, y))
                }
                _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
            }
        })
    }
}
