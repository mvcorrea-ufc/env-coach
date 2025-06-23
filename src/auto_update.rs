// src/auto_update.rs
use std::fs;
use std::path::Path;
use serde_json::Value;
use chrono::Utc;
use crate::config::{Project, BacklogItem, ItemType, Priority, Status};

#[derive(Debug)]
pub enum UpdateContext {
    RequirementAnalysis,
    TaskCompletion(String),
    CodeGeneration(String), 
    #[allow(dead_code)]
    Documentation,
}

pub struct AutoUpdater {
    project: Project,
}

impl AutoUpdater {
    pub fn new(project: Project) -> Self {
        Self { project }
    }

    /// Main orchestrator for processing LLM responses and updating project files
    pub fn process_llm_response(&mut self, llm_response: &str, context: UpdateContext) -> anyhow::Result<()> {
        match context {
            UpdateContext::RequirementAnalysis => {
                self.update_from_requirement_analysis(llm_response)?;
            },
            UpdateContext::TaskCompletion(task_id) => {
                self.update_from_task_completion(&task_id, llm_response)?;
            },
            UpdateContext::CodeGeneration(task_id) => {
                self.generate_code_files(&task_id, llm_response)?;
            },
            UpdateContext::Documentation => {
                self.update_documentation(llm_response)?;
            }
        }
        
        // Always save project.json after updates
        self.project.save()?;
        Ok(())
    }

    /// Parse LLM requirement analysis and automatically add user stories to project.json
    fn update_from_requirement_analysis(&mut self, llm_response: &str) -> anyhow::Result<()> {
        println!("🔄 Auto-updating project.json from LLM analysis...");
        
        // First, try to parse JSON from LLM response
        if let Ok(json_data) = serde_json::from_str::<Value>(llm_response) {
            if let Some(stories) = json_data.get("user_stories").and_then(|s| s.as_array()) {
                let mut added_count = 0;
                
                for story in stories {
                    if let Ok(backlog_item) = self.parse_user_story_from_json(story) {
                        self.project.backlog.push(backlog_item);
                        added_count += 1;
                    }
                }
                
                if added_count > 0 {
                    println!("✅ Auto-added {} user stories to project.json", added_count);
                    return Ok(());
                }
            }
        }
        
        // If JSON parsing fails, try to extract stories from text format
        self.extract_stories_from_text(llm_response)?;
        Ok(())
    }

    /// Parse a JSON user story object into a BacklogItem
    fn parse_user_story_from_json(&self, story_json: &Value) -> anyhow::Result<BacklogItem> {
        let title = story_json.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Generated Story")
            .to_string();
            
        let story_text = story_json.get("story")
            .and_then(|v| v.as_str())
            .unwrap_or("As a user, I want this feature")
            .to_string();
            
        let effort = story_json.get("effort")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as u32;
            
        let priority = match story_json.get("priority").and_then(|v| v.as_str()) {
            Some("critical") => Priority::Critical,
            Some("high") => Priority::High,
            Some("medium") => Priority::Medium,
            Some("low") => Priority::Low,
            _ => Priority::Medium,
        };
        
        let acceptance_criteria = story_json.get("acceptance_criteria")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect())
            .unwrap_or_else(|| vec![
                "Implement the feature as described".to_string(),
                "Write comprehensive tests".to_string(),
                "Update documentation".to_string(),
            ]);

        // Generate unique ID
        let story_count = self.project.backlog
            .iter()
            .filter(|item| matches!(item.item_type, ItemType::UserStory))
            .count();
        let story_id = format!("US-{:03}", story_count + 1);

        Ok(BacklogItem {
            id: story_id,
            item_type: ItemType::UserStory,
            title,
            story: story_text,
            acceptance_criteria,
            priority,
            effort,
            status: Status::Todo,
            created: Utc::now(),
            sprint: None,
            dependencies: Vec::new(),
        })
    }

    /// Extract user stories from plain text if JSON parsing fails
    fn extract_stories_from_text(&mut self, text: &str) -> anyhow::Result<()> {
        let lines: Vec<&str> = text.lines().collect();
        let mut stories_found = 0;
        
        for (i, line) in lines.iter().enumerate() {
            if line.to_lowercase().contains("as a user") || 
               line.to_lowercase().contains("user story") ||
               line.contains("US-") {
                
                let story_count = self.project.backlog
                    .iter()
                    .filter(|item| matches!(item.item_type, ItemType::UserStory))
                    .count();
                let story_id = format!("US-{:03}", story_count + stories_found + 1);
                
                // Extract title from context (look at previous lines)
                let title = self.extract_title_from_context(&lines, i);
                
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
                
                self.project.backlog.push(backlog_item);
                stories_found += 1;
            }
        }
        
        if stories_found > 0 {
            println!("✅ Auto-extracted {} user stories from LLM response", stories_found);
        } else {
            println!("⚠️ No user stories found in LLM response");
            println!("💡 LLM response may need manual processing");
        }
        
        Ok(())
    }

    /// Extract title from surrounding context lines
    fn extract_title_from_context(&self, lines: &[&str], current_index: usize) -> String {
        // Look backwards for a non-empty line that could be a title
        for i in (0..current_index).rev() {
            let line = lines[i].trim();
            if !line.is_empty() && 
               !line.to_lowercase().contains("as a user") &&
               !line.starts_with("##") &&
               line.len() < 100 {
                return line.to_string();
            }
        }
        
        // If no good title found, generate one
        format!("Generated Story {}", current_index + 1)
    }

    /// Auto-update documentation when task is completed
    fn update_from_task_completion(&mut self, task_id: &str, _llm_response: &str) -> anyhow::Result<()> {
        println!("📝 Auto-updating documentation for completed task {}...", task_id);
        
        if let Some(task) = self.project.backlog.iter().find(|item| item.id == task_id) {
            // Update README.md with completed features
            self.update_readme(task)?;
            
            // Generate changelog entry
            self.update_changelog(task)?;
            
            println!("✅ Documentation auto-updated for {}", task_id);
        }
        
        Ok(())
    }

    /// Generate code files from LLM suggestions
    fn generate_code_files(&self, task_id: &str, llm_response: &str) -> anyhow::Result<()> {
        println!("💻 Auto-generating code files for task {}...", task_id);
        
        // Extract code blocks from LLM response
        let code_blocks = self.extract_code_blocks(llm_response);
        
        if code_blocks.is_empty() {
            println!("ℹ️ No code blocks found in LLM response");
            return Ok(());
        }
        
        for (filename, code) in code_blocks {
            let file_path = Path::new(&filename);
            
            // Create directories if they don't exist
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            // Only create file if it doesn't exist (don't overwrite)
            if !file_path.exists() {
                fs::write(file_path, code)?;
                println!("✅ Generated: {}", filename);
            } else {
                println!("⚠️ File {} already exists - skipping generation", filename);
                println!("💡 To regenerate, delete the file and run assist-task again");
            }
        }
        
        Ok(())
    }

    /// Extract code blocks from LLM response (supports multiple languages)
    fn extract_code_blocks(&self, text: &str) -> Vec<(String, String)> {
        let mut code_blocks = Vec::new();
        let lines: Vec<&str> = text.lines().collect();
        let mut i = 0;
        
        // Determine project's primary language
        let primary_lang = self.get_primary_language();
        
        while i < lines.len() {
            // Look for code blocks with language specifiers
            if let Some(lang) = self.extract_language_from_line(lines[i]) {
                let mut code = String::new();
                i += 1;
                
                // Collect code until closing ```
                while i < lines.len() && !lines[i].starts_with("```") {
                    code.push_str(lines[i]);
                    code.push('\n');
                    i += 1;
                }
                
                if !code.trim().is_empty() {
                    // Try to determine filename from context and language
                    let filename = self.guess_filename_for_language(&code, &lang, code_blocks.len());
                    code_blocks.push((filename, code));
                }
            }
            i += 1;
        }
        
        code_blocks
    }

    /// Extract language from code block opening line
    fn extract_language_from_line(&self, line: &str) -> Option<String> {
        if line.starts_with("```") {
            let lang = line.trim_start_matches("```").trim().to_lowercase();
            if !lang.is_empty() {
                return Some(lang);
            }
        }
        None
    }

    /// Get primary programming language from project
    fn get_primary_language(&self) -> String {
        for tech in &self.project.meta.tech_stack {
            match tech.as_str() {
                "rust" => return "rust".to_string(),
                "nodejs" => return "javascript".to_string(),
                "python" => return "python".to_string(),
                "go" => return "go".to_string(),
                "java" => return "java".to_string(),
                _ => continue,
            }
        }
        "rust".to_string() // Default fallback
    }

    /// Guess appropriate filename for generated code based on language
    fn guess_filename_for_language(&self, code: &str, language: &str, index: usize) -> String {
        match language {
            "rust" => {
                if code.contains("fn main()") {
                    "src/main.rs".to_string()
                } else if code.contains("#[cfg(test)]") || code.contains("mod tests") {
                    "src/tests.rs".to_string()
                } else if code.contains("pub struct") || code.contains("pub enum") {
                    format!("src/lib_{}.rs", index)
                } else if code.contains("impl ") {
                    format!("src/module_{}.rs", index)
                } else {
                    format!("src/generated_{}.rs", index)
                }
            }
            "javascript" | "js" => {
                if code.contains("module.exports") || code.contains("export") {
                    format!("src/module_{}.js", index)
                } else {
                    format!("src/generated_{}.js", index)
                }
            }
            "python" | "py" => {
                if code.contains("if __name__ == \"__main__\"") {
                    "main.py".to_string()
                } else if code.contains("class ") {
                    format!("src/class_{}.py", index)
                } else {
                    format!("src/module_{}.py", index)
                }
            }
            "go" => {
                if code.contains("func main()") {
                    "main.go".to_string()
                } else {
                    format!("src/module_{}.go", index)
                }
            }
            "java" => {
                // Try to extract class name
                if let Some(class_name) = self.extract_java_class_name(code) {
                    format!("src/{}.java", class_name)
                } else {
                    format!("src/Generated_{}.java", index)
                }
            }
            _ => {
                // Default to project's primary language extension
                let primary_lang = self.get_primary_language();
                let extension = match primary_lang.as_str() {
                    "rust" => "rs",
                    "javascript" => "js", 
                    "python" => "py",
                    "go" => "go",
                    "java" => "java",
                    _ => "txt",
                };
                format!("src/generated_{}.{}", index, extension)
            }
        }
    }

    /// Extract Java class name from code
    fn extract_java_class_name(&self, code: &str) -> Option<String> {
        for line in code.lines() {
            if line.trim().starts_with("public class ") || line.trim().starts_with("class ") {
                if let Some(class_part) = line.split_whitespace().nth(2) {
                    return Some(class_part.trim_end_matches('{').to_string());
                }
            }
        }
        None
    }

    /// Update README.md with completed features
    fn update_readme(&self, task: &BacklogItem) -> anyhow::Result<()> {
        let readme_path = "README.md";
        let mut content = if Path::new(readme_path).exists() {
            fs::read_to_string(readme_path)?
        } else {
            format!("# {}\n\n## Features\n\n", self.project.meta.name)
        };
        
        // Add completed feature
        let feature_line = format!("- ✅ {} ({})\n", task.title, task.id);
        
        // Check if feature already exists to avoid duplicates
        if content.contains(&task.id) {
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

    /// Update CHANGELOG.md
    fn update_changelog(&self, task: &BacklogItem) -> anyhow::Result<()> {
        let changelog_path = "CHANGELOG.md";
        let mut content = if Path::new(changelog_path).exists() {
            fs::read_to_string(changelog_path)?
        } else {
            "# Changelog\n\n".to_string()
        };
        
        let today = chrono::Utc::now().format("%Y-%m-%d");
        let entry = format!("## {} - {}\n- Completed: {} ({})\n\n", today, task.title, task.story, task.id);
        
        // Check if entry already exists
        if content.contains(&task.id) {
            return Ok(());
        }
        
        // Insert after the first line (# Changelog)
        if let Some(first_newline) = content.find('\n') {
            content.insert_str(first_newline + 1, &entry);
        } else {
            content.push_str(&entry);
        }
        
        fs::write(changelog_path, content)?;
        Ok(())
    }

    /// Update documentation (placeholder for future features)
    fn update_documentation(&self, _llm_response: &str) -> anyhow::Result<()> {
        println!("📚 Documentation update completed");
        Ok(())
    }

    /// Get the updated project (for saving)
    pub fn get_project(&self) -> &Project {
        &self.project
    }
}