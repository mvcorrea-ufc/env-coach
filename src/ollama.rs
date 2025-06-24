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

// --- New structs and function for /api/generate ---

#[derive(Debug, serde::Serialize)]
struct OllamaGenerationRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    // options: Option<std::collections::HashMap<String, f32>>, // For future customization
}

#[derive(Debug, serde::Deserialize)]
struct OllamaGenerationResponse {
    response: String, // This field contains the actual generated text (expected to be a JSON string)
    // model: String,
    // created_at: String,
    // done: bool,
    // context: Option<Vec<i32>>, // For conversational context
    // total_duration: Option<u64>,
    // load_duration: Option<u64>,
    // prompt_eval_count: Option<usize>,
    // prompt_eval_duration: Option<u64>,
    // eval_count: Option<usize>,
    // eval_duration: Option<u64>,
}

/// Sends a prompt to Ollama's /api/generate endpoint.
/// Expects the LLM to produce a response string, which itself should be parsable JSON.
pub async fn send_generation_prompt(cfg: &FinalLlmConfig, prompt_text: &str) -> anyhow::Result<String> {
    use anyhow::Context; // Ensure Context is in scope for .with_context()

    info!("Sending generation prompt to Ollama model: {}", cfg.model);
    debug!("Prompt text (first 200 chars): {:.200}", prompt_text.chars().take(200).collect::<String>());

    let client = Client::builder()
        .timeout(Duration::from_millis(cfg.timeout_ms))
        .build()
        .context("Failed to build HTTP client for Ollama")?;

    let url = format!("{}/api/generate", cfg.base_url());
    let request_body = OllamaGenerationRequest {
        model: &cfg.model,
        prompt: prompt_text,
        stream: false, // We want the full response, not a stream
    };

    debug!("Ollama generation request URL: {}", url);
    // Avoid logging the full prompt if it's very large, already logged a snippet.
    // Consider logging `request_body` only if a specific debug flag is very high.

    let res = client.post(&url)
        .json(&request_body)
        .send()
        .await
        .with_context(|| format!("Failed to send request to Ollama generate API at {}", url))?;

    let response_status = res.status();
    if response_status.is_success() {
        let ollama_response: OllamaGenerationResponse = res.json().await
            .with_context(|| "Failed to parse JSON response from Ollama generate API")?;
        debug!("Successfully received and parsed response from Ollama generate API.");
        Ok(ollama_response.response)
    } else {
        let error_text = res.text().await.unwrap_or_else(|_| "N/A".to_string());
        error!("Ollama generate API responded with HTTP {} and body: {}", response_status, error_text);
        anyhow::bail!("Ollama /api/generate request failed with status: {} - {}", response_status, error_text)
    }
}

// --- End of new structs and function ---

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