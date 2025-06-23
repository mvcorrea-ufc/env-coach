// src/scripts/llm_cycle.rs
use std::fs;
use log::{info, debug, error};

pub async fn run(prompt: String) -> anyhow::Result<()> {  // Make async
    info!("Running LLM cycle script with prompt: {}", prompt);
    
    // Try to load project configuration
    let project = match crate::config::Project::load() {
        Ok(project) => {
            println!("📋 Using project LLM configuration");
            project
        }
        Err(_) => {
            println!("⚠️  No env-coach project found, using default LLM config");
            crate::config::Project::create_in_current_dir()?
        }
    };
    
    project.validate()?; // Validate configuration before use
    let cfg = &project.meta.llm;

    // Check if prompt is a file path
    let prompt_text = if prompt.contains('.') && fs::metadata(&prompt).is_ok() {
        info!("Reading prompt from file: {}", prompt);
        fs::read_to_string(&prompt)
            .map_err(|e| {
                error!("Failed to read prompt file '{}': {}", prompt, e);
                anyhow::anyhow!("Failed to read prompt file '{}': {}", prompt, e)
            })?
    } else {
        // Treat as direct prompt text
        prompt
    };
    
    debug!("Prompt text loaded successfully. Length: {} characters", prompt_text.len());

    println!("🤖 Sending prompt to LLM...");
    crate::ollama::send_prompt(&cfg, &prompt_text).await?;  // Add .await
    info!("LLM cycle completed successfully.");
    Ok(())
}