use crate::llm::Message;
use anyhow::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Pool, Row, Sqlite,
};
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct MemoryManager {
    pool: Pool<Sqlite>,
}

impl MemoryManager {
    pub async fn new(db_path: &str) -> Result<Self> {
        let options =
            SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?.create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let manager = Self { pool };
        manager.init_tables().await?;

        Ok(manager)
    }

    async fn init_tables(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn save_message(&self, message: &Message) -> Result<()> {
        sqlx::query("INSERT INTO messages (role, content) VALUES (?, ?)")
            .bind(&message.role)
            .bind(&message.content)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_recent_history(&self, limit: i64) -> Result<Vec<Message>> {
        let rows = sqlx::query("SELECT role, content FROM messages ORDER BY id DESC LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;

        let mut messages = Vec::new();
        for row in rows {
            messages.push(Message {
                role: row.get("role"),
                content: row.get("content"),
                images: None,
            });
        }

        // Reverse to get chronological order
        messages.reverse();
        Ok(messages)
    }

    pub async fn clear_history(&self) -> Result<()> {
        sqlx::query("DELETE FROM messages")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
