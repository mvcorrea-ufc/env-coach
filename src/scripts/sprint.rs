// src/scripts/sprint.rs
use anyhow::Result;
use crate::config::{Project, SprintStatus, Status};

pub fn plan(_goal: String, _days: u32) -> Result<()> {
    println!("🏃 Sprint planning functionality coming soon!");
    println!("💡 For now, you can:");
    println!("   env-coach start-task <task-id>      # Start working on tasks");
    println!("   env-coach list-backlog              # View available tasks");
    Ok(())
}

pub fn start_sprint(_sprint_id: String) -> Result<()> {
    println!("🏃 Sprint start functionality coming soon!");
    println!("💡 For now, you can:");
    println!("   env-coach start-task <task-id>      # Start working on tasks");
    Ok(())
}

pub fn show_current_sprint() -> Result<()> {
    let project = Project::load()?;
    
    let active_sprint = project.sprints.iter().find(|s| matches!(s.status, SprintStatus::Active));
    
    match active_sprint {
        Some(sprint) => {
            println!("🏃 Current Sprint: {}", sprint.id);
            println!("🎯 Goal: {}", sprint.goal);
            println!("📅 Duration: {} to {}", 
                sprint.start_date.format("%Y-%m-%d"),
                sprint.end_date.format("%Y-%m-%d")
            );
            println!("📊 Progress: {} / {} points", sprint.completed_points, sprint.total_points);
            
            let progress_percent = if sprint.total_points > 0 {
                (sprint.completed_points * 100) / sprint.total_points
            } else {
                0
            };
            println!("📈 Completion: {}%", progress_percent);
            
            // Show sprint backlog
            let sprint_items: Vec<_> = project.backlog
                .iter()
                .filter(|item| item.sprint.as_ref() == Some(&sprint.id))
                .collect();
                
            if !sprint_items.is_empty() {
                println!();
                println!("📋 Sprint Backlog ({} items):", sprint_items.len());
                
                let todo_count = sprint_items.iter().filter(|item| matches!(item.status, Status::Todo)).count();
                let in_progress_count = sprint_items.iter().filter(|item| matches!(item.status, Status::InProgress)).count();
                let done_count = sprint_items.iter().filter(|item| matches!(item.status, Status::Done)).count();
                
                for item in sprint_items {
                    let status_emoji = match item.status {
                        Status::Todo => { "⏳" },
                        Status::InProgress => { "🚧" },
                        Status::Review => { "👀" },
                        Status::Done => { "✅" },
                    };
                    
                    let priority_emoji = match item.priority {
                        crate::config::Priority::Critical => "🔴",
                        crate::config::Priority::High => "🟠",
                        crate::config::Priority::Medium => "🟡",
                        crate::config::Priority::Low => "🟢",
                    };
                    
                    println!("  {} {} {} - {} [{}pts]", 
                        status_emoji, priority_emoji, item.id, item.title, item.effort);
                }
                
                println!();
                println!("📊 Sprint Status:");
                println!("   ⏳ To Do: {}", todo_count);
                println!("   🚧 In Progress: {}", in_progress_count);
                println!("   ✅ Done: {}", done_count);
            }
        }
        None => {
            println!("📭 No active sprint");
            println!();
            println!("🎯 Start planning:");
            println!("   env-coach plan-sprint --goal \"Sprint objective\"  # Plan new sprint");
            println!("   env-coach list-backlog                           # View available tasks");
        }
    }
    
    Ok(())
}