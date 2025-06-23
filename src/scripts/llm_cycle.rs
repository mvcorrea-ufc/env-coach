// src/scripts/llm_cycle.rs
use std::fs;
use log::{info, debug, error};

pub async fn run(prompt: String) -> anyhow::Result<()> {  // Make async
    info!("Running LLM cycle script with prompt: {}", prompt);
    
    // Try to load project configuration
    let project = match crate::config::Project::load() {
        Ok(project) => {
            println!("📋 Using project LLM configuration from project.json");
            project
        }
        Err(_) => {
            println!("⚠️  No env-coach project found (project.json missing or invalid).");
            println!("Attempting to use global/default LLM configuration for this cycle.");
            // Load global config to pass to create_in_current_dir for default project setup
            let global_config = crate::config::GlobalConfig::load()
                .map_err(|e| {
                    // Log this error but proceed, as create_in_current_dir can handle None for global_llm_config
                    error!("Failed to load global config for llm_cycle fallback: {}", e);
                    e
                })
                .unwrap_or_default(); // Proceed with default if global load fails catastrophically

            crate::config::Project::create_in_current_dir(global_config.llm.as_ref())?
        }
    };
    
    // Project::load() and Project::new (via create_in_current_dir) now populate resolved_llm_config
    // So we should use project.llm() which returns &FinalLlmConfig
    project.validate()?; // Validate configuration before use
    let cfg = project.llm(); // Use the resolved LLM config

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