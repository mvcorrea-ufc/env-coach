// src/scripts/tasks.rs
use anyhow::{Context, Result};
use reqwest;
use serde_json::Value;
use crate::config::{FinalLlmConfig, Project, Status}; // Changed LlmConfig to FinalLlmConfig
use crate::auto_update::{AutoUpdater, UpdateContext}; // NEW: Import auto-update

pub fn start_task(id: String) -> Result<()> {
    let mut project = Project::load()
        .context("Failed to load project. Run 'env-coach init <n>' first")?;

    // Find the task
    let task_index = project.backlog
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;

    // Update status to In Progress
    project.backlog[task_index].status = Status::InProgress;

    // Store task details for printing (before saving)
    let task_title = project.backlog[task_index].title.clone();
    let task_story = project.backlog[task_index].story.clone();
    let task_priority = project.backlog[task_index].priority.clone();
    let task_effort = project.backlog[task_index].effort;
    let task_criteria = project.backlog[task_index].acceptance_criteria.clone();

    project.save()
        .context("Failed to save project")?;

    println!("🚀 Starting task: {}", id);
    println!("✅ Task {} status updated to 'In Progress'", id);
    println!("📋 Task Details:");
    println!("   Title: {}", task_title);
    println!("   Story: {}", task_story);
    println!("   Priority: {:?}", task_priority);
    println!("   Effort: {} points", task_effort);
    println!("   Acceptance Criteria:");
    for (i, criteria) in task_criteria.iter().enumerate() {
        println!("     {}. {}", i + 1, criteria);
    }
    
    println!("🤖 Need LLM assistance?");
    println!("   env-coach assist-task {}", id);
    println!("⏯️  When done:");
    println!("   env-coach complete-task {}", id);

    Ok(())
}

pub async fn assist_task(id: String) -> Result<()> {
    let project = Project::load()
        .context("Failed to load project. Run 'env-coach init <n>' first")?;

    println!("🤖 Providing LLM assistance for task: {}", id);
    println!("🔍 Analyzing task and generating assistance...");

    // Find the task
    let task = project.backlog
        .iter()
        .find(|item| item.id == id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;

    println!("📋 Task Details:");
    println!("   Title: {}", task.title);
    println!("   Story: {}", task.story);
    println!("   Priority: {:?}", task.priority);
    println!("   Effort: {} points", task.effort);
    println!("   Acceptance Criteria:");
    for (i, criteria) in task.acceptance_criteria.iter().enumerate() {
        println!("     {}. {}", i + 1, criteria);
    }

    // Send to LLM for assistance with full project context
    let llm_response = send_llm_assistance_request(task, project.llm(), &project)
        .await
        .context("Failed to get LLM assistance")?;

    println!("🤖 LLM Response:");
    println!("{}", llm_response);

    // NEW: Auto-generate code files if LLM provides implementation
    let mut updater = AutoUpdater::new(project);
    updater.process_llm_response(&llm_response, UpdateContext::CodeGeneration(id.clone()))
        .context("Failed to auto-generate code files")?;

    println!("💡 Use the LLM suggestions to implement your solution");
    println!("💡 When ready: env-coach complete-task {}", id);

    Ok(())
}

pub fn complete_task(id: String) -> Result<()> {
    let mut project = Project::load()
        .context("Failed to load project. Run 'env-coach init <n>' first")?;

    // Find the task
    let task = project.backlog
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;

    // Update status to Done
    task.status = Status::Done;
    
    // Update sprint progress if task is in a sprint
    if let Some(sprint_id) = &task.sprint {
        if let Some(sprint) = project.sprints.iter_mut().find(|s| s.id == *sprint_id) {
            sprint.completed_points += task.effort;
        }
    }

    println!("✅ Completing task: {}", id);
    println!("📋 Task '{}' marked as Done", task.title);

    // NEW: Auto-update documentation
    let mut updater = AutoUpdater::new(project);
    updater.process_llm_response("", UpdateContext::TaskCompletion(id.clone()))
        .context("Failed to auto-update documentation")?;

    // Save the updated project
    let updated_project = updater.get_project();
    updated_project.save()
        .context("Failed to save project")?;

    println!("📝 Documentation auto-updated (README.md, CHANGELOG.md)");
    
    // Show updated sprint progress if applicable
    if let Some(task) = updated_project.backlog.iter().find(|item| item.id == id) {
        if let Some(sprint_id) = &task.sprint {
            if let Some(sprint) = updated_project.sprints.iter().find(|s| s.id == *sprint_id) {
                let progress_percent = if sprint.total_points > 0 {
                    (sprint.completed_points * 100) / sprint.total_points
                } else {
                    0
                };
                
                println!("📊 Sprint Progress: {} / {} points ({}%)", 
                    sprint.completed_points, sprint.total_points, progress_percent);
            }
        }
    }

    println!("🎯 Next steps:");
    println!("   env-coach show-sprint               # View current sprint status");
    println!("   env-coach start-task <next-id>      # Start next task");

    Ok(())
}

async fn send_llm_assistance_request(
    task: &crate::config::BacklogItem, 
    llm_config: &FinalLlmConfig, // Changed LlmConfig to FinalLlmConfig
    project: &Project
) -> Result<String> {
    let client = reqwest::Client::new();
    
    let acceptance_criteria_text = task.acceptance_criteria
        .iter()
        .enumerate()
        .map(|(i, criteria)| format!("{}. {}", i + 1, criteria))
        .collect::<Vec<_>>()
        .join("\n");

    // Get primary programming language
    let primary_language = get_primary_language(&project.meta.tech_stack);
    let language_specific_guidance = get_language_guidance(&primary_language);

    let prompt = format!(
        r#"You are a software engineering expert providing implementation guidance for a {primary_language} project.

PROJECT CONTEXT:
- Project: {project_name}
- Description: {project_description}
- Tech Stack: {tech_stack}
- Primary Language: {primary_language}
- Tags: {tags}

TASK DETAILS:
- Task: {task_title}
- Story: {task_story}
- Priority: {task_priority:?}
- Effort: {task_effort} points

ACCEPTANCE CRITERIA:
{acceptance_criteria}

Please provide implementation guidance specifically for {primary_language}:

1. **Approach and Architecture** - How to structure this in {primary_language}
2. **Dependencies/Crates** - Specific {primary_language} libraries/crates needed
3. **Code Implementation** - Complete working {primary_language} code examples
4. **Testing** - {primary_language} unit test examples
5. **Best Practices** - {primary_language}-specific implementation tips

{language_guidance}

IMPORTANT: 
- Provide ALL code examples in {primary_language}
- Use proper {primary_language} syntax and conventions
- Format code blocks as ```{code_block_lang}
- Give complete, runnable examples that match the project structure"#,
        primary_language = primary_language,
        project_name = project.meta.name,
        project_description = project.meta.description,
        tech_stack = project.meta.tech_stack.join(", "),
        tags = project.get_tags_display(),
        task_title = task.title,
        task_story = task.story,
        task_priority = task.priority,
        task_effort = task.effort,
        acceptance_criteria = acceptance_criteria_text,
        language_guidance = language_specific_guidance,
        code_block_lang = get_code_block_language(&primary_language)
    );

    let request_body = serde_json::json!({
        "model": llm_config.model,
        "prompt": prompt,
        "stream": false,
        "options": {
            "temperature": 0.3,
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

/// Get language-specific guidance
fn get_language_guidance(language: &str) -> String {
    match language {
        "Rust" => r#"
Focus on:
- Ownership and borrowing principles
- Error handling with Result<T, E> and proper error propagation
- Using appropriate data structures (Vec, HashMap, etc.)
- Implementing traits where appropriate
- Writing idiomatic Rust code with proper lifetime management
- Using Cargo.toml for dependencies"#.to_string(),
        
        "JavaScript/Node.js" => r#"
Focus on:
- Modern JavaScript/ES6+ features
- Proper async/await usage
- NPM package management
- Error handling with try/catch
- Modular code structure"#.to_string(),
        
        "Python" => r#"
Focus on:
- Pythonic code style and PEP 8 compliance
- Proper exception handling
- Using virtual environments and requirements.txt
- Type hints where appropriate
- Object-oriented or functional programming as suitable"#.to_string(),
        
        _ => "Follow language best practices and conventions.".to_string(),
    }
}

/// Get the code block language identifier
fn get_code_block_language(language: &str) -> String {
    match language {
        "Rust" => "rust".to_string(),
        "JavaScript/Node.js" => "javascript".to_string(),
        "Python" => "python".to_string(),
        "Go" => "go".to_string(),
        "Java" => "java".to_string(),
        _ => "rust".to_string(), // Default to rust
    }
}