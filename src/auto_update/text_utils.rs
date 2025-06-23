// src/auto_update/text_utils.rs
use crate::config::{Project, BacklogItem, ItemType, Priority, Status};
use chrono::Utc;

// This function is pure and doesn't need Project state.
pub fn extract_title_from_context(lines: &[&str], current_index: usize) -> String {
    for i in (0..current_index).rev() {
        let line = lines[i].trim();
        if !line.is_empty() &&
           !line.to_lowercase().contains("as a user") &&
           !line.starts_with("##") &&
           !line.starts_with("```") &&
           line.len() < 100 {
            return line.to_string();
        }
    }
    format!("Generated Title for Story near line {}", current_index + 1)
}

// This function modifies the project's backlog.
pub fn extract_stories_from_text(project: &mut Project, text: &str) -> anyhow::Result<()> {
    let lines: Vec<&str> = text.lines().collect();
    let mut stories_found = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains("as a user") ||
           line.to_lowercase().contains("user story") ||
           line.contains("US-") {

            // Determine next available US-ID for text extraction
            let story_id_num = project.backlog // Use the passed-in project
                .iter()
                .filter(|item| item.id.starts_with("US-"))
                .count() + 1 + stories_found;
            let story_id = format!("US-{:03}", story_id_num);

            // Call the local (or imported if moved elsewhere) extract_title_from_context
            let title = extract_title_from_context(&lines, i);

            let backlog_item = BacklogItem {
                id: story_id,
                item_type: ItemType::UserStory,
                title,
                story: line.trim().to_string(),
                acceptance_criteria: vec![
                    "Define specific acceptance criteria".to_string(),
                    "Implement the feature".to_string(),
                    "Write tests and documentation".to_string(),
                ],
                priority: Priority::Medium,
                effort: 3,
                status: Status::Todo,
                created: Utc::now(),
                sprint: None,
                dependencies: Vec::new(),
            };

            project.backlog.push(backlog_item);
            stories_found += 1;
        }
    }
    if stories_found > 0 {
        println!("✅ Auto-extracted {} user stories from LLM response via text fallback.", stories_found);
    } else {
        println!("⚠️ No user stories found in LLM response (neither JSON nor text fallback).");
        println!("💡 LLM response may need manual processing or prompt adjustment.");
    }
    Ok(())
}
