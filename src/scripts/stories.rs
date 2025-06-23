// src/scripts/stories.rs
use crate::config::{Project, BacklogItem, ItemType, Priority, Status};
use chrono::Utc;

pub fn add_manual_story(title: String, description: String) -> anyhow::Result<()> {
    println!("📝 Adding user story manually...");
    
    let mut project = Project::load()?;
    
    // Generate sequential ID
    let story_count = project.backlog
        .iter()
        .filter(|item| matches!(item.item_type, ItemType::UserStory))
        .count();
    let story_id = format!("US-{:03}", story_count + 1);
    
    // Create the backlog item
    let story = BacklogItem {
        id: story_id.clone(),
        item_type: ItemType::UserStory,
        title: title.clone(),
        story: if description.starts_with("As a") || description.starts_with("As an") {
            description
        } else {
            format!("As a user, I want {} so that I can achieve my goals.", description)
        },
        acceptance_criteria: vec![
            "Define clear acceptance criteria".to_string(),
            "Write unit tests".to_string(),
            "Update documentation".to_string()
        ],
        priority: Priority::Medium,
        effort: 3, // Default estimate
        status: Status::Todo,
        created: Utc::now(),
        sprint: None,
        dependencies: Vec::new(),
    };
    
    project.backlog.push(story);
    project.save()?;
    
    println!("✅ Added story {}: {}", story_id, title);
    println!("💡 Edit project.json to refine acceptance criteria and effort estimate");
    println!();
    println!("🎯 Next steps:");
    println!("   env-coach list-backlog              # View updated backlog");
    println!("   env-coach plan-sprint --goal \"...\"  # Plan development sprint");
    
    Ok(())
}

pub fn list_stories() -> anyhow::Result<()> {
    let project = Project::load()?;
    
    let stories: Vec<_> = project.backlog
        .iter()
        .filter(|item| matches!(item.item_type, ItemType::UserStory))
        .collect();
    
    if stories.is_empty() {
        println!("📖 No user stories found");
        println!();
        println!("🎯 Add stories:");
        println!("   env-coach add-requirement \"I want to build...\"");
        println!("   env-coach add-story --title \"Title\" --description \"Description\"");
        return Ok(());
    }
    
    println!("📖 User Stories ({} total):", stories.len());
    println!();
    
    // Group by status - simple approach to avoid pattern matching complexity
    let in_progress_stories: Vec<_> = stories.iter().filter(|s| matches!(s.status, Status::InProgress)).collect();
    let review_stories: Vec<_> = stories.iter().filter(|s| matches!(s.status, Status::Review)).collect();
    let todo_stories: Vec<_> = stories.iter().filter(|s| matches!(s.status, Status::Todo)).collect();
    let done_stories: Vec<_> = stories.iter().filter(|s| matches!(s.status, Status::Done)).collect();

    if !in_progress_stories.is_empty() {
        println!("🚧 In Progress ({}):", in_progress_stories.len());
        for story in &in_progress_stories {
            print_story_detail(story);
        }
        println!();
    }

    if !review_stories.is_empty() {
        println!("👀 In Review ({}):", review_stories.len());
        for story in &review_stories {
            print_story_detail(story);
        }
        println!();
    }

    if !todo_stories.is_empty() {
        println!("⏳ To Do ({}):", todo_stories.len());
        for story in &todo_stories {
            print_story_detail(story);
        }
        println!();
    }

    if !done_stories.is_empty() {
        println!("✅ Done ({}):", done_stories.len());
        for story in &done_stories {
            print_story_detail(story);
        }
        println!();
    }
    
    Ok(())
}

fn print_story_detail(story: &BacklogItem) {
    let priority_color = match story.priority {
        Priority::Critical => "🔴",
        Priority::High => "🟠",
        Priority::Medium => "🟡",
        Priority::Low => "🟢",
    };
    
    println!("  {} {} - {} [{}pts]", priority_color, story.id, story.title, story.effort);
    println!("     {}", story.story);
    println!("     📋 {} acceptance criteria", story.acceptance_criteria.len());
    if let Some(sprint) = &story.sprint {
        println!("     🏃 Sprint: {}", sprint);
    }
    if !story.dependencies.is_empty() {
        println!("     🔗 Dependencies: {}", story.dependencies.join(", "));
    }
}