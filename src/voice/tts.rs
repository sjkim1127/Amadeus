use anyhow::Result;
use std::process::Command;

pub struct TtsManager;

impl TtsManager {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn speak(&self, text: &str) -> Result<()> {
        // Use macOS 'say' command
        // This is non-blocking if we use spawn()
        Command::new("say").arg(text).spawn()?;
        Ok(())
    }
}
