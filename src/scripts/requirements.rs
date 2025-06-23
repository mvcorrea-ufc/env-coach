// src/scripts/requirements.rs
use anyhow::{Context, Result};
use reqwest;
use serde_json::Value;
use crate::config::{FinalLlmConfig, Project}; // Changed LlmConfig to FinalLlmConfig
use crate::auto_update::{AutoUpdater, UpdateContext}; // NEW: Import auto-update

pub async fn process_requirement(requirement: String) -> Result<()> {
    println!("🔍 Processing requirement: {}", requirement);
    
    // Load project configuration
    let project = Project::load()
        .context("Failed to load project. Run 'env-coach init <n>' first")?;
    
    // Send requirement to LLM for analysis
    let llm_response = send_llm_request(&requirement, project.llm(), &project)
        .await
        .context("Failed to get LLM analysis")?;
    
    println!("🤖 LLM Response:");
    println!("{}", llm_response);
    
    // NEW: Auto-update project.json instead of manual edit message
    let mut updater = AutoUpdater::new(project);
    updater.process_llm_response(&llm_response, UpdateContext::RequirementAnalysis)
        .context("Failed to auto-update project files")?;
    
    println!("✅ Requirement processed and project.json auto-updated!");
    println!("🎯 Next steps:");
    println!("   env-coach list-backlog              # View updated backlog");
    println!("   env-coach plan-sprint --goal \"...\"  # Plan development sprint");
    
    Ok(())
}

async fn send_llm_request(requirement: &str, llm_config: &FinalLlmConfig, project: &Project) -> Result<String> { // Changed LlmConfig to FinalLlmConfig
    let client = reqwest::Client::new();
    
    // Get primary programming language for context
    let primary_language = get_primary_language(&project.meta.tech_stack);

    // Load prompt from file
    let prompt_template_path = std::path::Path::new(".env-coach/prompts/requirements_analyst.md");
    let mut prompt_template = std::fs::read_to_string(prompt_template_path)
        .context(format!("Failed to read prompt template from {:?}", prompt_template_path))?;

    // Perform replacements
    prompt_template = prompt_template.replace("{{project_name}}", &project.meta.name);
    prompt_template = prompt_template.replace("{{project_description}}", &project.meta.description);
    prompt_template = prompt_template.replace("{{tech_stack}}", &project.meta.tech_stack.join(", "));
    prompt_template = prompt_template.replace("{{primary_language}}", &primary_language);
    prompt_template = prompt_template.replace("{{tags}}", &project.get_tags_display());
    prompt_template = prompt_template.replace("{{requirement}}", requirement);

    let final_prompt = prompt_template;

    let request_body = serde_json::json!({
        "model": llm_config.model,
        "prompt": final_prompt, // Use the processed prompt string
        "stream": false,
        "options": {
            "temperature": 0.7,
            "top_p": 0.9
        }
    });

    let response = client
        .post(&format!("{}/api/generate", llm_config.base_url()))
        .json(&request_body)
        .timeout(std::time::Duration::from_millis(llm_config.timeout_ms))
        .send()
        .await
        .context("Failed to send request to LLM")?;

    if !response.status().is_success() {
        anyhow::bail!(
            "LLM request failed with status: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }

    let response_json: Value = response
        .json()
        .await
        .context("Failed to parse LLM response as JSON")?;

    let llm_response = response_json
        .get("response")
        .and_then(|r| r.as_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid LLM response format"))?;

    Ok(llm_response.to_string())
}

/// Determine the primary programming language from tech stack
fn get_primary_language(tech_stack: &[String]) -> String {
    for tech in tech_stack {
        match tech.as_str() {
            "rust" => return "Rust".to_string(),
            "nodejs" => return "JavaScript/Node.js".to_string(),
            "python" => return "Python".to_string(),
            "go" => return "Go".to_string(),
            "java" => return "Java".to_string(),
            _ => continue,
        }
    }
    "Rust".to_string() // Default fallback
}