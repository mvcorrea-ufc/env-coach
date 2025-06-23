// src/scripts/status.rs
use anyhow::{Context, Result};
use crate::config::{Project, Status, SprintStatus};
use crate::ollama;

pub async fn run() -> Result<()> {  // Make async
    // Check if project is initialized
    if !Project::is_initialized() {
        println!("❌ No env-coach project found");
        println!("💡 Initialize a project first:");
        println!("   env-coach init                      # Use current directory name");
        println!("   env-coach init <n>               # Use custom name");
        return Ok(());
    }

    let project = Project::load()
        .context("Failed to load project configuration")?;

    println!("📋 Project Status: {}", project.meta.name);
    println!("📝 Description: {}", project.meta.description);
    println!("🛠️  Tech Stack: {}", project.get_tech_stack_description());
    println!("📅 Created: {}", project.meta.created.format("%Y-%m-%d %H:%M UTC"));
    println!();

    // Test LLM connectivity
    // Load global config to show sources
    let global_config = crate::config::GlobalConfig::load().unwrap_or_default(); // Handle error better if needed

    println!("🤖 LLM Configuration (resolved):");
    let resolved_llm = project.llm();
    let project_llm_override = project.meta.llm.as_ref();
    let global_llm_settings = global_config.llm.as_ref();

    // Helper to determine source
    fn get_source_info(
        p_val: Option<&str>, g_val: Option<&str>, def_val: &str, actual_val: &str,
        p_source_name: &str, g_source_name: &str, def_source_name: &str
    ) -> String {
        if p_val.map_or(false, |v| v == actual_val) {
            p_source_name.to_string()
        } else if g_val.map_or(false, |v| v == actual_val) {
            g_source_name.to_string()
        } else if actual_val == def_val {
            def_source_name.to_string()
        } else {
            "Unknown".to_string() // Should not happen if logic is correct
        }
    }
    
    // Using a more specific default check for numeric types like port/timeout
    fn get_source_info_numeric<T: PartialEq + std::fmt::Display>(
        p_opt_val: Option<T>, g_opt_val: Option<T>, def_val: T, actual_val: &T,
        p_source_name: &str, g_source_name: &str, def_source_name: &str
    ) -> String {
        if p_opt_val.as_ref().map_or(false, |v| v == actual_val) {
            p_source_name.to_string()
        } else if g_opt_val.as_ref().map_or(false, |v| v == actual_val) {
            g_source_name.to_string()
        } else if *actual_val == def_val {
            def_source_name.to_string()
        } else {
            "Unknown".to_string() // Should ideally not be reached
        }
    }


    let model_source = get_source_info(
        project_llm_override.and_then(|p| p.model.as_deref()),
        global_llm_settings.and_then(|g| g.model.as_deref()),
        crate::config::DEFAULT_LLM_MODEL,
        &resolved_llm.model,
        "Project (project.json)", "Global (~/.config/env-coach/config.json)", "Default"
    );
    println!("   Model:      {} (Source: {})", resolved_llm.model, model_source);

    let host_source = get_source_info(
        project_llm_override.and_then(|p| p.host.as_deref()),
        global_llm_settings.and_then(|g| g.host.as_deref()),
        crate::config::DEFAULT_LLM_HOST,
        &resolved_llm.host,
        "Project (project.json)", "Global (~/.config/env-coach/config.json)", "Default"
    );
    println!("   Host:       {} (Source: {})", resolved_llm.host, host_source);

    let port_source = get_source_info_numeric(
        project_llm_override.and_then(|p| p.port),
        global_llm_settings.and_then(|g| g.port),
        crate::config::DEFAULT_LLM_PORT,
        &resolved_llm.port,
        "Project (project.json)", "Global (~/.config/env-coach/config.json)", "Default"
    );
    println!("   Port:       {} (Source: {})", resolved_llm.port, port_source);

    let timeout_source = get_source_info_numeric(
        project_llm_override.and_then(|p| p.timeout_ms),
        global_llm_settings.and_then(|g| g.timeout_ms),
        crate::config::DEFAULT_LLM_TIMEOUT_MS,
        &resolved_llm.timeout_ms,
        "Project (project.json)", "Global (~/.config/env-coach/config.json)", "Default"
    );
    println!("   Timeout:    {}ms (Source: {})", resolved_llm.timeout_ms, timeout_source);
    println!("   Base URL:   {}", resolved_llm.base_url());
    println!("   Tags:       {}", project.get_tags_display()); // Tags are not part of LLM config sources

    match ollama::check_status(resolved_llm).await { // Pass resolved_llm explicitly
        Ok(()) => {
            // Success message is printed by ollama::check_status itself
        }
        Err(e) => {
            println!("   Status: ❌ Connection failed");
            println!("   Error details: {}", e);
            println!();
            println!("💡 Troubleshooting tips:");
            println!("   1. Ensure Ollama is running. On your Ollama server, try: ollama ps");
            println!("   2. Verify the Ollama URL used by env-coach: {}", project.llm().base_url());
            println!("      Consider these configuration sources (project overrides global):");
            println!("      - Project specific: ./project.json (in the 'llm' section)");
            println!("      - Global default: ~/.config/env-coach/config.json (in the 'llm' section)");
            println!("   3. If the model '{}' is specified, ensure it's available on the Ollama server:", project.llm().model);
            println!("      On your Ollama server, try: ollama pull {}", project.llm().model);
            println!("   4. Check network connectivity from this machine to the Ollama host: {}", project.llm().host);
        }
    }
    println!();

    // Show backlog summary
    println!("📋 Backlog Summary:");
    let total_items = project.backlog.len();
    let todo_items = project.backlog.iter().filter(|item| matches!(item.status, Status::Todo)).count();
    let in_progress_items = project.backlog.iter().filter(|item| matches!(item.status, Status::InProgress)).count();
    let review_items = project.backlog.iter().filter(|item| matches!(item.status, Status::Review)).count();
    let done_items = project.backlog.iter().filter(|item| matches!(item.status, Status::Done)).count();

    if total_items == 0 {
        println!("   No items in backlog");
        println!("💡 Add requirements to get started:");
        println!("   env-coach add-requirement \"I want to build...\"");
    } else {
        println!("   Total items: {}", total_items);
        println!("   📋 To Do: {}", todo_items);
        println!("   🚧 In Progress: {}", in_progress_items);
        println!("   👀 In Review: {}", review_items);
        println!("   ✅ Done: {}", done_items);
        
        // Calculate completion percentage
        if total_items > 0 {
            let completion_percent = (done_items * 100) / total_items;
            println!("   📊 Completion: {}%", completion_percent);
        }
    }
    println!();

    // Show sprint information
    println!("🏃 Sprint Information:");
    if project.sprints.is_empty() {
        println!("   No sprints created");
        println!("💡 Plan your first sprint:");
        println!("   env-coach plan-sprint --goal \"Sprint objective\"");
    } else {
        let active_sprints = project.sprints.iter().filter(|s| matches!(s.status, SprintStatus::Active)).count();
        let completed_sprints = project.sprints.iter().filter(|s| matches!(s.status, SprintStatus::Completed | SprintStatus::Complete)).count();
        
        println!("   Total sprints: {}", project.sprints.len());
        println!("   🏃 Active: {}", active_sprints);
        println!("   ✅ Completed: {}", completed_sprints);

        // Show current sprint details
        if let Some(active_sprint) = project.sprints.iter().find(|s| matches!(s.status, SprintStatus::Active)) {
            println!();
            println!("📌 Current Sprint: {}", active_sprint.id);
            println!("   Goal: {}", active_sprint.goal);
            println!("   Progress: {} / {} points", active_sprint.completed_points, active_sprint.total_points);
            let sprint_progress = if active_sprint.total_points > 0 {
                (active_sprint.completed_points * 100) / active_sprint.total_points
            } else {
                0
            };
            println!("   📊 Sprint Completion: {}%", sprint_progress);
        }
    }
    println!();

    // Show next suggested actions
    println!("🎯 Suggested Next Actions:");
    if project.backlog.is_empty() {
        println!("   1. env-coach add-requirement \"...\"     # Add your first requirement");
    } else if project.sprints.is_empty() {
        println!("   1. env-coach plan-sprint --goal \"...\"  # Plan your first sprint");
    } else if todo_items > 0 {
        if let Some(next_task) = project.backlog.iter().find(|item| matches!(item.status, Status::Todo)) {
            println!("   1. env-coach start-task {}           # Start next task", next_task.id);
        }
    } else if in_progress_items > 0 {
        if let Some(current_task) = project.backlog.iter().find(|item| matches!(item.status, Status::InProgress)) {
            println!("   1. env-coach assist-task {}          # Get help with current task", current_task.id);
            println!("   2. env-coach complete-task {}        # Mark task as done", current_task.id);
        }
    } else {
        println!("   1. env-coach add-requirement \"...\"     # Add more requirements");
        println!("   2. env-coach plan-sprint --goal \"...\"  # Plan next sprint");
    }

    Ok(())
}