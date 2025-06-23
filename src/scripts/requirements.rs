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
    
    let prompt = format!(
        r#"You are a software engineering expert analyzing requirements for a {primary_language} project.

PROJECT CONTEXT:
- Project: {project_name}
- Description: {project_description}
- Tech Stack: {tech_stack}
- Primary Language: {primary_language}
- Tags: {tags}

REQUIREMENT TO ANALYZE: "{requirement}"

Please respond with a JSON object containing user stories specifically tailored for this {primary_language} project:

{{
  "user_stories": [
    {{
      "title": "Brief descriptive title",
      "story": "As a user, I want ... so that ...",
      "priority": "high|medium|low",
      "effort": 3,
      "acceptance_criteria": [
        "Specific testable criterion considering {primary_language} implementation",
        "Technical criterion relevant to {tech_stack}",
        "User-facing criterion for the feature"
      ]
    }}
  ]
}}

Focus on:
1. Breaking down the requirement into {primary_language}-appropriate user stories
2. Writing acceptance criteria that consider {primary_language} implementation details
3. Estimating effort appropriately for {primary_language} development (1-8 points)
4. Prioritizing based on user value and technical dependencies in {primary_language}
5. Including technical considerations specific to the tech stack: {tech_stack}

Generate 2-5 user stories that comprehensively cover the requirement for a {primary_language} project.
Return only valid JSON."#,
        requirement = requirement,
        primary_language = primary_language,
        project_name = project.meta.name,
        project_description = project.meta.description,
        tech_stack = project.meta.tech_stack.join(", "),
        tags = project.get_tags_display()
    );

    let request_body = serde_json::json!({
        "model": llm_config.model,
        "prompt": prompt,
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