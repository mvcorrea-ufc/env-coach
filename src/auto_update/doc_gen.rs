// src/auto_update/doc_gen.rs

use std::fs;
use std::path::Path;
use crate::config::{Project, BacklogItem};
use chrono::Utc; // Ensure chrono is imported for Utc::now() if used here, or rely on Project's created times.

fn update_readme(project: &Project, task: &BacklogItem) -> anyhow::Result<()> { // project is used
    let readme_path = "README.md";
    let mut content = if Path::new(readme_path).exists() {
        fs::read_to_string(readme_path)?
    } else {
        format!("# {}\n\n## Features\n\n", project.meta.name) // Use project.meta.name
    };

    let feature_line = format!("- ✅ {} ({})\n", task.title, task.id);

    if content.contains(&task.id) { // Avoid duplicates
        return Ok(());
    }

    if let Some(features_pos) = content.find("## Features") {
        let insert_pos = features_pos + content[features_pos..].find('\n').unwrap_or(0) + 1;
        content.insert_str(insert_pos, &feature_line);
    } else {
        content.push_str(&format!("\n## Features\n{}", feature_line));
    }

    fs::write(readme_path, content)?;
    Ok(())
}

fn update_changelog(_project: &Project, task: &BacklogItem) -> anyhow::Result<()> { // _project
    let changelog_path = "CHANGELOG.md";
    let mut content = if Path::new(changelog_path).exists() {
        fs::read_to_string(changelog_path)?
    } else {
        "# Changelog\n\n".to_string()
    };

    let today = Utc::now().format("%Y-%m-%d"); // task.completed_at would be better if available
    let entry = format!("## {} - {}\n- Completed: {} ({})\n\n", today, task.title, task.story, task.id);

    if content.contains(&task.id) { // Avoid duplicates
        return Ok(());
    }

    if let Some(first_newline) = content.find('\n') {
        content.insert_str(first_newline + 1, &entry);
    } else {
        content.push_str(&entry);
    }

    fs::write(changelog_path, content)?;
    Ok(())
}

/// Placeholder for future, more comprehensive documentation updates.
#[allow(dead_code)]
pub fn update_documentation(_project: &Project, _llm_response: &str) -> anyhow::Result<()> {
    println!("📚 Documentation update (placeholder) completed");
    Ok(())
}

/// Main function for this module, called by AutoUpdater.
pub fn update_docs_for_task_completion(project: &Project, task_id: &str, _llm_response: &str) -> anyhow::Result<()> {
    println!("📝 Auto-updating documentation for completed task {}...", task_id);
    if let Some(task) = project.backlog.iter().find(|item| item.id == task_id) {
        update_readme(project, task)?;
        update_changelog(project, task)?;
        println!("✅ Documentation auto-updated for {}", task_id);
    } else {
        println!("⚠️ Task {} not found for documentation update.", task_id);
    }
    Ok(())
}
