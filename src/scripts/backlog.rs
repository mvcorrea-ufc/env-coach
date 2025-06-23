// src/scripts/backlog.rs
use crate::config::{Project, Priority, Status, ItemType};

pub fn run() -> anyhow::Result<()> {
    let project = Project::load()?;
    
    if project.backlog.is_empty() {
        println!("📋 Backlog is empty");
        println!();
        println!("🎯 Get started:");
        println!("   env-coach add-requirement \"I want to build a web API\"");
        println!("   env-coach add-story --title \"Story\" --description \"Description\"");
        return Ok(());
    }
    
    println!("📋 Project Backlog ({} items)", project.backlog.len());
    println!();
    
    // Group items by status
    let todo_items: Vec<_> = project.backlog.iter().filter(|item| matches!(item.status, Status::Todo)).collect();
    let in_progress_items: Vec<_> = project.backlog.iter().filter(|item| matches!(item.status, Status::InProgress)).collect();
    let review_items: Vec<_> = project.backlog.iter().filter(|item| matches!(item.status, Status::Review)).collect();
    let done_items: Vec<_> = project.backlog.iter().filter(|item| matches!(item.status, Status::Done)).collect();

    if !in_progress_items.is_empty() {
        println!("🚧 In Progress ({}):", in_progress_items.len());
        for item in &in_progress_items {
            print_backlog_item(item);
        }
        println!();
    }

    if !review_items.is_empty() {
        println!("👀 In Review ({}):", review_items.len());
        for item in &review_items {
            print_backlog_item(item);
        }
        println!();
    }

    if !todo_items.is_empty() {
        println!("⏳ To Do ({}):", todo_items.len());
        for item in &todo_items {
            print_backlog_item(item);
        }
        println!();
    }

    if !done_items.is_empty() {
        println!("✅ Done ({}):", done_items.len());
        for item in &done_items {
            print_backlog_item(item);
        }
        println!();
    }

    // Show summary statistics
    let total_effort: u32 = project.backlog.iter().map(|item| item.effort).sum();
    let completed_effort: u32 = done_items.iter().map(|item| item.effort).sum();
    
    println!("📊 Summary:");
    println!("   Total effort: {} points", total_effort);
    println!("   Completed: {} points", completed_effort);
    if total_effort > 0 {
        let completion_percent = (completed_effort * 100) / total_effort;
        println!("   Progress: {}%", completion_percent);
    }
    
    // Show next action
    if !todo_items.is_empty() {
        println!();
        println!("🎯 Next action:");
        if let Some(next_item) = todo_items.first() {
            println!("   env-coach start-task {}             # Start working on next task", next_item.id);
        }
    }

    Ok(())
}

fn print_backlog_item(item: &crate::config::BacklogItem) {
    let priority_emoji = match item.priority {
        Priority::Critical => "🔴",
        Priority::High => "🟠",
        Priority::Medium => "🟡",
        Priority::Low => "🟢",
    };
    
    let type_emoji = match item.item_type {
        ItemType::UserStory => "📖",
        ItemType::Bug => "🐛",
        ItemType::Epic => "🎯",   // Changed from Feature to Epic
        ItemType::Task => "📋",
    };
    
    println!("  {} {} {} - {} [{}pts]", priority_emoji, type_emoji, item.id, item.title, item.effort);
    println!("     {}", item.story);
    if let Some(sprint) = &item.sprint {
        println!("     🏃 Sprint: {}", sprint);
    }
    if !item.dependencies.is_empty() {
        println!("     🔗 Dependencies: {}", item.dependencies.join(", "));
    }
}