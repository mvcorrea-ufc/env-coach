// src/config.rs
//! Project configuration and data structures

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub meta: ProjectMeta,
    pub backlog: Vec<BacklogItem>,
    pub sprints: Vec<Sprint>,
    pub current_sprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub description: String,
    pub created: DateTime<Utc>,
    pub tech_stack: Vec<String>,
    pub tags: Vec<String>,
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub model: String,
    pub timeout_ms: u64,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklogItem {
    pub id: String,
    pub item_type: ItemType,
    pub title: String,
    pub story: String,
    pub acceptance_criteria: Vec<String>,
    pub priority: Priority,
    pub effort: u32,
    pub status: Status,
    pub created: DateTime<Utc>,
    pub sprint: Option<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemType {
    UserStory,
    Bug,
    Epic,
    Task,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status {
    Todo,
    InProgress,
    Review,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: String,
    pub goal: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub status: SprintStatus,
    pub total_points: u32,
    pub completed_points: u32,
    pub tasks: Vec<String>,
    pub stories: Vec<String>,
    pub planned_velocity: u8,
    pub actual_velocity: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SprintStatus {
    Planning,
    Active,
    Review,
    Completed,
    Complete,  // Compatibility variant
}

impl LlmConfig {
    /// Get the base URL from host and port
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

impl Project {
    pub fn new(name: String, description: String) -> Self {
        let llm_config = LlmConfig {
            model: "deepseek-coder:6.7b".to_string(),
            timeout_ms: 180000,
            host: "localhost".to_string(),
            port: 11434,
        };

        // Detect tech stack from current directory
        let tech_stack = Self::detect_tech_stack();
        
        // Generate initial tags based on project characteristics
        let tags = Self::generate_initial_tags(&name, &tech_stack);

        Self {
            meta: ProjectMeta {
                name: name.clone(),
                description,
                created: Utc::now(),
                tech_stack,
                tags,
                llm: llm_config,
            },
            backlog: Vec::new(),
            sprints: Vec::new(),
            current_sprint: None,
        }
    }

    /// Detect technology stack from current directory files
    fn detect_tech_stack() -> Vec<String> {
        let mut tech_stack = Vec::new();

        // Check for common project files
        if Path::new("Cargo.toml").exists() {
            tech_stack.push("rust".to_string());
        }
        if Path::new("package.json").exists() {
            tech_stack.push("nodejs".to_string());
        }
        if Path::new("requirements.txt").exists() || Path::new("setup.py").exists() || Path::new("pyproject.toml").exists() {
            tech_stack.push("python".to_string());
        }
        if Path::new("go.mod").exists() {
            tech_stack.push("go".to_string());
        }
        if Path::new("pom.xml").exists() || Path::new("build.gradle").exists() {
            tech_stack.push("java".to_string());
        }
        if Path::new("Dockerfile").exists() {
            tech_stack.push("docker".to_string());
        }
        if Path::new(".git").exists() {
            tech_stack.push("git".to_string());
        }

        // Default to "general" if no specific tech detected
        if tech_stack.is_empty() {
            tech_stack.push("general".to_string());
        }

        tech_stack
    }

    /// Generate initial tags based on project name and tech stack
    fn generate_initial_tags(name: &str, tech_stack: &[String]) -> Vec<String> {
        let mut tags = Vec::new();
        
        // Add tags based on project name patterns
        let name_lower = name.to_lowercase();
        if name_lower.contains("api") || name_lower.contains("server") {
            tags.push("backend".to_string());
        }
        if name_lower.contains("web") || name_lower.contains("frontend") || name_lower.contains("ui") {
            tags.push("frontend".to_string());
        }
        if name_lower.contains("cli") || name_lower.contains("tool") {
            tags.push("cli".to_string());
        }
        if name_lower.contains("game") {
            tags.push("game".to_string());
        }
        if name_lower.contains("bot") {
            tags.push("automation".to_string());
        }
        if name_lower.contains("lib") || name_lower.contains("crate") {
            tags.push("library".to_string());
        }

        // Add tags based on tech stack
        if tech_stack.contains(&"rust".to_string()) {
            tags.push("systems".to_string());
        }
        if tech_stack.contains(&"nodejs".to_string()) {
            tags.push("javascript".to_string());
        }
        if tech_stack.contains(&"docker".to_string()) {
            tags.push("containerization".to_string());
        }

        // Default tags
        if tags.is_empty() {
            tags.push("development".to_string());
        }
        
        // Always add env-coach tag
        tags.push("env-coach".to_string());
        
        tags
    }

    /// Get LLM configuration (shortcut to meta.llm)
    pub fn llm(&self) -> &LlmConfig {
        &self.meta.llm
    }

    pub fn load() -> Result<Self> {
        let content = fs::read_to_string("project.json")
            .context("Failed to read project.json. Run 'env-coach init' first")?;
        
        let project: Project = serde_json::from_str(&content)
            .context("Failed to parse project.json")?;
        
        Ok(project)
    }

    pub fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize project")?;
        
        fs::write("project.json", content)
            .context("Failed to write project.json")?;
        
        Ok(())
    }

    #[allow(dead_code)]
    pub fn add_backlog_item(&mut self, item: BacklogItem) {
        self.backlog.push(item);
    }

    #[allow(dead_code)]
    pub fn get_backlog_item(&self, id: &str) -> Option<&BacklogItem> {
        self.backlog.iter().find(|item| item.id == id)
    }

    #[allow(dead_code)]
    pub fn get_backlog_item_mut(&mut self, id: &str) -> Option<&mut BacklogItem> {
        self.backlog.iter_mut().find(|item| item.id == id)
    }

    #[allow(dead_code)]
    pub fn add_sprint(&mut self, sprint: Sprint) {
        self.sprints.push(sprint);
    }

    #[allow(dead_code)]
    pub fn get_active_sprint(&self) -> Option<&Sprint> {
        self.sprints.iter().find(|sprint| matches!(sprint.status, SprintStatus::Active))
    }

    #[allow(dead_code)]
    pub fn get_active_sprint_mut(&mut self) -> Option<&mut Sprint> {
        self.sprints.iter_mut().find(|sprint| matches!(sprint.status, SprintStatus::Active))
    }

    #[allow(dead_code)]
    pub fn get_user_stories(&self) -> Vec<&BacklogItem> {
        self.backlog.iter().filter(|item| matches!(item.item_type, ItemType::UserStory)).collect()
    }

    #[allow(dead_code)]
    pub fn get_todo_items(&self) -> Vec<&BacklogItem> {
        self.backlog.iter().filter(|item| matches!(item.status, Status::Todo)).collect()
    }

    #[allow(dead_code)]
    pub fn get_completed_items(&self) -> Vec<&BacklogItem> {
        self.backlog.iter().filter(|item| matches!(item.status, Status::Done)).collect()
    }

    pub fn validate(&self) -> Result<()> {
        // Basic validation
        if self.meta.name.is_empty() {
            anyhow::bail!("Project name cannot be empty");
        }
        
        if self.meta.llm.model.is_empty() {
            anyhow::bail!("LLM model cannot be empty");
        }
        
        // Validate LLM configuration
        if self.meta.llm.host.is_empty() {
            anyhow::bail!("LLM host cannot be empty");
        }

        if self.meta.llm.port == 0 {
            anyhow::bail!("LLM port must be greater than 0");
        }
        
        Ok(())
    }

    pub fn is_initialized() -> bool {
        Path::new("project.json").exists()
    }

    pub fn create_in_current_dir() -> Result<Self> {
        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;
        let project_name = cwd
            .file_name()
            .context("Failed to get directory name")?
            .to_string_lossy()
            .to_string();

        Ok(Self::new(project_name, "Generated project".to_string()))
    }

    /// Get a user-friendly description of the detected tech stack
    pub fn get_tech_stack_description(&self) -> String {
        match self.meta.tech_stack.as_slice() {
            stack if stack.contains(&"rust".to_string()) => "Rust project with modern tooling".to_string(),
            stack if stack.contains(&"nodejs".to_string()) => "Node.js/JavaScript project".to_string(),
            stack if stack.contains(&"python".to_string()) => "Python project".to_string(),
            stack if stack.contains(&"go".to_string()) => "Go project".to_string(),
            stack if stack.contains(&"java".to_string()) => "Java project".to_string(),
            _ => format!("Multi-technology project ({})", self.meta.tech_stack.join(", "))
        }
    }

    /// Get tags as a formatted string
    pub fn get_tags_display(&self) -> String {
        if self.meta.tags.is_empty() {
            "none".to_string()
        } else {
            self.meta.tags.join(", ")
        }
    }
}