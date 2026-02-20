use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use serde_json::{json, Value};
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;

use crate::agent::tools::{Tool, ToolResult};

pub struct ScreenshotTool;

impl Tool for ScreenshotTool {
    fn name(&self) -> &str {
        "take_screenshot"
    }

    fn description(&self) -> &str {
        "Captures the current screen content and returns it as a base64 encoded string. Use this to see what is on the user's screen."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    fn execute(&self, _args: Value) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> {
        Box::pin(async move {
            // Using screenshots crate for cross-platform support
            let screens = screenshots::Screen::all()
                .map_err(|e| anyhow::anyhow!("Failed to get screens: {}", e))?;
            let screen = screens
                .first()
                .ok_or_else(|| anyhow::anyhow!("No screens found"))?;

            let image_buffer = screen
                .capture()
                .map_err(|e| anyhow::anyhow!("Failed to capture screen: {}", e))?;

            // Convert ImageBuffer from screenshots crate to our local image crate type
            // This avoids type mismatch if multiple image crate versions are present
            let width = image_buffer.width();
            let height = image_buffer.height();
            let raw = image_buffer.into_raw();

            let img_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, raw)
                .ok_or_else(|| anyhow::anyhow!("Failed to construct image buffer"))?;

            let img = DynamicImage::ImageRgba8(img_buffer);

            // Resize image to reduce token usage and latency (e.g., max 1024x768)
            let resized = img.resize(1024, 768, image::imageops::FilterType::Lanczos3);

            let mut bytes: Vec<u8> = Vec::new();
            resized.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Jpeg)?;

            let base64_string = general_purpose::STANDARD.encode(&bytes);

            Ok(format!("IMAGE_BASE64:{}", base64_string))
        })
    }
}
