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
    println!("🤖 LLM Configuration:");
    println!("   Model: {}", project.llm().model);
    println!("   URL: {}", project.llm().base_url());
    println!("   Tags: {}", project.get_tags_display());
    
    match ollama::check_status(project.llm()).await {  // Add .await
        Ok(()) => {
            // Status message is printed by check_status function
        }
        Err(e) => {
            println!("   Status: ❌ Not reachable");
            println!("   Error: {}", e);
            println!("💡 Make sure Ollama is running:");
            println!("   ollama serve");
            println!("   ollama pull {}", project.llm().model);
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