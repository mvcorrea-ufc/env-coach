// src/ollama.rs
//! Networking utilities to interact with the Ollama REST API

use crate::config::FinalLlmConfig; // Changed from LlmConfig
use reqwest::Client;
use std::time::Duration;
use log::{info, debug, error};

/// Ping Ollama and list available models.
/// Returns Ok(()) on success, Err on any network/HTTP error.
pub async fn check_status(cfg: &FinalLlmConfig) -> anyhow::Result<()> {  // Changed cfg type
    info!("Attempting to check Ollama status with config: {:?}", cfg);
    let client = Client::builder()
        .timeout(Duration::from_millis(cfg.timeout_ms))
        .build()?;
    debug!("HTTP client built with timeout: {}ms", cfg.timeout_ms);

    let url = format!("{}/api/tags", cfg.base_url());
    info!("Sending GET request to URL: {}", url);
    let res = client.get(&url).send().await?;  // Add .await
    debug!("Received response status: {}", res.status());

    if res.status().is_success() {
        info!("Ollama responded with success status: {}", res.status());
        println!("   Status: ✅ Connected to {}", cfg.base_url());
        Ok(())
    } else {
        let status = res.status();
        let text = res.text().await.unwrap_or_else(|_| "N/A".to_string());  // Add .await
        error!("Ollama responded with HTTP {} and body: {}", status, text);
        anyhow::bail!("Ollama responded with HTTP {}", status);
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct ChatResponse {
    #[allow(dead_code)]
    pub message: Option<Message>,
    // Add other fields if needed for parsing the response
}

#[allow(dead_code)]
pub async fn send_prompt(cfg: &FinalLlmConfig, prompt: &str) -> anyhow::Result<()> { // Ensure this was FinalLlmConfig
    let client = Client::builder()  // Remove blocking::
        .timeout(Duration::from_millis(cfg.timeout_ms))
        .build()?;

    let url = format!("{}/api/chat", cfg.base_url());
    info!("Sending POST request to URL: {}", url);

    let request_body = ChatRequest {
        model: cfg.model.clone(),
        messages: vec![
            Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ],
        stream: false, // For simplicity, not streaming for now
    };

    debug!("Request body: {:?}", request_body);

    let res = client.post(&url).json(&request_body).send().await?;  // Add .await
    debug!("Received response status: {}", res.status());

    if res.status().is_success() {
        let chat_response: ChatResponse = res.json().await?;  // Add .await
        if let Some(message) = chat_response.message {
            println!("🤖 LLM Response:");
            println!("{}", message.content);
            info!("LLM response received successfully.");
        } else {
            info!("LLM response received, but no message content found.");
            println!("🤖 LLM Response: (No message content)");
        }
        Ok(())
    } else {
        let status = res.status();
        let text = res.text().await.unwrap_or_else(|_| "N/A".to_string());  // Add .await
        error!("Ollama chat API responded with HTTP {} and body: {}", status, text);
        anyhow::bail!("Ollama chat API responded with HTTP {}", status);
    }
}