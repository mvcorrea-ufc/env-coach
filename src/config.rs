// src/config.rs
//! Project configuration and data structures

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub meta: ProjectMeta, // meta.llm is Option<PartialLlmConfig> for serialization
    pub backlog: Vec<BacklogItem>,
    pub sprints: Vec<Sprint>,
    pub current_sprint: Option<String>,
    #[serde(skip)] // This field is for runtime use, not persisted in project.json directly
    pub resolved_llm_config: FinalLlmConfig,
}

// ProjectMeta still has llm: Option<PartialLlmConfig>
// This is what gets serialized/deserialized from project.json's "meta" field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub description: String,
    pub created: DateTime<Utc>,
    pub tech_stack: Vec<String>,
    pub tags: Vec<String>,
    // This field in project.json will store project-specific overrides.
    // It's optional itself; if None, only global/defaults are used.
    // If Some, its fields override global/defaults.
    pub llm: Option<PartialLlmConfig>,
}

// Represents LLM config as stored in JSON files (global or project-specific)
// All fields are optional to allow for overriding and defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)] // Added PartialEq
pub struct PartialLlmConfig {
    pub model: Option<String>,
    pub timeout_ms: Option<u64>,
    pub host: Option<String>,
    pub port: Option<u16>,
}

// Represents the fully resolved LLM configuration after merging global and project settings.
// Fields here are non-optional, with defaults applied if not specified anywhere.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalLlmConfig {
    pub model: String,
    pub timeout_ms: u64,
    pub host: String,
    pub port: u16,
}

impl Default for FinalLlmConfig {
    fn default() -> Self {
        Self {
            model: DEFAULT_LLM_MODEL.to_string(),
            timeout_ms: DEFAULT_LLM_TIMEOUT_MS,
            host: DEFAULT_LLM_HOST.to_string(),
            port: DEFAULT_LLM_PORT,
        }
    }
}

impl FinalLlmConfig {
    /// Get the base URL from host and port
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

// Global configuration structure, primarily for LLM settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    pub llm: Option<PartialLlmConfig>,
    // Potentially other global settings can be added here
}

impl GlobalConfig {
    pub fn load() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("env-coach");

        let config_path = config_dir.join("config.json");

        if !config_path.exists() {
            // It's okay for the global config not to exist, means use defaults.
            return Ok(GlobalConfig::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Error reading global env-coach config file at: {:?}", config_path))?;

        let config: GlobalConfig = serde_json::from_str(&content)
            .with_context(|| format!("Error parsing global env-coach config file. Please check its JSON structure at: {:?}", config_path))?;

        Ok(config)
    }
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

// Default values for LLM configuration
pub const DEFAULT_LLM_MODEL: &str = "deepseek-coder:6.7b";
pub const DEFAULT_LLM_TIMEOUT_MS: u64 = 180000;
pub const DEFAULT_LLM_HOST: &str = "localhost";
pub const DEFAULT_LLM_PORT: u16 = 11434;

fn resolve_llm_config(global_cfg: Option<&PartialLlmConfig>, project_cfg: Option<&PartialLlmConfig>) -> FinalLlmConfig {
    let g_model = global_cfg.and_then(|g| g.model.as_ref());
    let g_timeout = global_cfg.and_then(|g| g.timeout_ms);
    let g_host = global_cfg.and_then(|g| g.host.as_ref());
    let g_port = global_cfg.and_then(|g| g.port);

    let p_model = project_cfg.and_then(|p| p.model.as_ref());
    let p_timeout = project_cfg.and_then(|p| p.timeout_ms);
    let p_host = project_cfg.and_then(|p| p.host.as_ref());
    let p_port = project_cfg.and_then(|p| p.port);

    FinalLlmConfig {
        model: p_model.or(g_model).map(String::from).unwrap_or_else(|| DEFAULT_LLM_MODEL.to_string()),
        timeout_ms: p_timeout.or(g_timeout).unwrap_or(DEFAULT_LLM_TIMEOUT_MS),
        host: p_host.or(g_host).map(String::from).unwrap_or_else(|| DEFAULT_LLM_HOST.to_string()),
        port: p_port.or(g_port).unwrap_or(DEFAULT_LLM_PORT),
    }
}

impl Project {
    // ProjectMeta now stores Option<PartialLlmConfig>
    // The actual FinalLlmConfig is stored in Project.resolved_llm_config.
    // ProjectMeta.llm (Option<PartialLlmConfig>) is for serialization to project.json.
    pub fn new(name: String, description: String, global_llm_config: Option<&PartialLlmConfig>) -> Self {
        let resolved_llm_config = resolve_llm_config(global_llm_config, None);
        let tech_stack = Self::detect_tech_stack();
        let tags = Self::generate_initial_tags(&name, &tech_stack);

        Self {
            meta: ProjectMeta {
                name: name.clone(),
                description,
                created: Utc::now(),
                tech_stack,
                tags,
                llm: None, // Project-specific overrides are initially None
            },
            backlog: Vec::new(),
            sprints: Vec::new(),
            current_sprint: None,
            resolved_llm_config, // Store the fully resolved config
        }
    }

    // Helper struct for deserializing Project from project.json
    // This matches the structure of project.json where meta.llm is Option<PartialLlmConfig>
} // End of impl Project block

// Helper struct for deserializing Project from project.json, moved to module scope
#[derive(Deserialize)]
struct ProjectFileContent {
    meta: ProjectMeta,
    backlog: Vec<BacklogItem>,
    sprints: Vec<Sprint>,
    current_sprint: Option<String>,
}

impl Project { // Re-open impl Project block for remaining methods

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

    /// Get the resolved LLM configuration for runtime use
    pub fn llm(&self) -> &FinalLlmConfig {
        &self.resolved_llm_config
    }

    pub fn load() -> Result<Self> {
        // 1. Load global config
        let global_config = GlobalConfig::load()?;
        let global_llm_cfg = global_config.llm.as_ref();

        // 2. Read project.json
        let content = fs::read_to_string("project.json")
            .context("Failed to read project.json. Run 'env-coach init' first.")?;
        
        // 3. Deserialize into ProjectFileContent, which expects meta.llm to be Option<PartialLlmConfig>
        let project_file_content: ProjectFileContent = serde_json::from_str(&content)
            .context("Failed to parse project.json. Check its structure.")?;
        
        // 4. Resolve the LLM configuration
        let resolved_llm_config = resolve_llm_config(
            global_llm_cfg,
            project_file_content.meta.llm.as_ref(),
        );

        // 5. Construct the final Project struct
        Ok(Project {
            meta: project_file_content.meta,
            backlog: project_file_content.backlog,
            sprints: project_file_content.sprints,
            current_sprint: project_file_content.current_sprint,
            resolved_llm_config,
        })
    }

    pub fn save(&self) -> Result<()> {
        // Create a temporary serializable structure if Project itself cannot be directly serialized
        // due to resolved_llm_config vs meta.llm.
        // However, since resolved_llm_config is #[serde(skip)], serializing Project directly
        // should work as intended, and meta.llm (Option<PartialLlmConfig>) will be used.
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
            anyhow::bail!("Project name cannot be empty in project.json.");
        }
        
        // Validate the resolved LLM configuration that will be used at runtime
        if self.resolved_llm_config.model.is_empty() {
            anyhow::bail!("LLM model cannot be empty. Check global config (~/.config/env-coach/config.json) or project.json.");
        }
        
        if self.resolved_llm_config.host.is_empty() {
            anyhow::bail!("LLM host cannot be empty. Check global config or project.json.");
        }

        if self.resolved_llm_config.port == 0 {
            anyhow::bail!("LLM port must be greater than 0. Check global config or project.json.");
        }
        
        Ok(())
    }

    pub fn is_initialized() -> bool {
        Path::new("project.json").exists()
    }

    // Updated to pass global_llm_config to new
    pub fn create_in_current_dir(global_llm_config: Option<&PartialLlmConfig>) -> Result<Self> {
        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;
        let project_name = cwd
            .file_name()
            .context("Failed to get directory name")?
            .to_string_lossy()
            .to_string();

        Ok(Self::new(project_name, "Generated project".to_string(), global_llm_config))
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


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    // use std::io::Write; // No longer needed after removing create_temp_project_json
    // use tempfile::NamedTempFile; // No longer needed

    // Helper to create a temporary global config file
    fn create_temp_global_config(content: &str, base_dir: &Path) -> Result<()> {
        let config_dir = base_dir.join("env-coach");
        fs::create_dir_all(&config_dir)?;
        fs::write(config_dir.join("config.json"), content)?;
        Ok(())
    }

    // This test is covered by test_global_config_load_from_path_no_file and the general
    // behavior of GlobalConfig::load() returning default when the actual global config is missing.
    // #[test]
    // fn test_load_global_config_no_file_real_dirs() {
    //     // This test would rely on the real dirs::config_dir() and ensure no file exists there.
    //     // It's hard to guarantee that state across all test environments.
    //     // GlobalConfig::load_from_path covers the logic of handling a non-existent file.
    //     let default_global_config = GlobalConfig::load().unwrap_or_default();
    //     assert!(default_global_config.llm.is_none());
    // }

    #[test]
    fn test_resolve_llm_config_all_defaults() {
        let resolved = resolve_llm_config(None, None);
        assert_eq!(resolved.model, DEFAULT_LLM_MODEL);
        assert_eq!(resolved.host, DEFAULT_LLM_HOST);
        assert_eq!(resolved.port, DEFAULT_LLM_PORT);
        assert_eq!(resolved.timeout_ms, DEFAULT_LLM_TIMEOUT_MS);
    }

    #[test]
    fn test_resolve_llm_config_global_overrides_defaults() {
        let global_partial = PartialLlmConfig {
            model: Some("global-model".to_string()),
            host: Some("global-host".to_string()),
            port: Some(1234),
            timeout_ms: Some(50000),
        };
        let resolved = resolve_llm_config(Some(&global_partial), None);
        assert_eq!(resolved.model, "global-model");
        assert_eq!(resolved.host, "global-host");
        assert_eq!(resolved.port, 1234);
        assert_eq!(resolved.timeout_ms, 50000);
    }

    #[test]
    fn test_resolve_llm_config_project_overrides_global_and_defaults() {
        let global_partial = PartialLlmConfig {
            model: Some("global-model".to_string()),
            host: Some("global-host".to_string()),
            port: Some(1234),
            timeout_ms: Some(50000),
        };
        let project_partial = PartialLlmConfig {
            model: Some("project-model".to_string()),
            host: None, // Project uses global host
            port: Some(5678),
            timeout_ms: None, // Project uses global timeout
        };
        let resolved = resolve_llm_config(Some(&global_partial), Some(&project_partial));
        assert_eq!(resolved.model, "project-model");
        assert_eq!(resolved.host, "global-host"); // From global
        assert_eq!(resolved.port, 5678);         // From project
        assert_eq!(resolved.timeout_ms, 50000);  // From global
    }

    #[test]
    fn test_resolve_llm_config_project_overrides_only_some_defaults() {
        let project_partial = PartialLlmConfig {
            model: Some("project-model".to_string()),
            host: None,
            port: None,
            timeout_ms: Some(10000),
        };
        let resolved = resolve_llm_config(None, Some(&project_partial));
        assert_eq!(resolved.model, "project-model");
        assert_eq!(resolved.host, DEFAULT_LLM_HOST); // Default
        assert_eq!(resolved.port, DEFAULT_LLM_PORT); // Default
        assert_eq!(resolved.timeout_ms, 10000);    // Project
    }

    #[test]
    fn test_project_new_uses_global_or_defaults() {
        // Case 1: No global config
        let project1 = Project::new("test1".to_string(), "desc1".to_string(), None);
        assert_eq!(project1.resolved_llm_config.model, DEFAULT_LLM_MODEL);
        assert_eq!(project1.meta.llm, None); // Project specific overrides are None initially

        // Case 2: With global config
        let global_partial = PartialLlmConfig {
            model: Some("global-model-for-new".to_string()),
            ..Default::default()
        };
        let project2 = Project::new("test2".to_string(), "desc2".to_string(), Some(&global_partial));
        assert_eq!(project2.resolved_llm_config.model, "global-model-for-new");
        assert_eq!(project2.meta.llm, None);
    }

    // To test Project::load and GlobalConfig::load properly, we need to manage
    // the filesystem environment for each test. This involves creating temp files and dirs.
    // The `dirs::config_dir()` call in `GlobalConfig::load` makes it hard to test in isolation
    // without actually creating files in the real config location or using more advanced mocking.

    // Let's focus on tests that don't require filesystem manipulation beyond current dir for project.json
    // or can be adapted. For GlobalConfig::load, we might need to refactor it slightly to allow
    // injecting the config path for testing.

    // Temporary refactor for testability of GlobalConfig::load:
    // Add a new method for testing that takes a base path.
    impl GlobalConfig {
        #[cfg(test)]
        fn load_from_path(base_path: &Path) -> Result<Self> {
            let config_dir = base_path.join("env-coach");
            let config_path = config_dir.join("config.json");
            if !config_path.exists() { Ok(GlobalConfig::default()) } else {
                let content = fs::read_to_string(&config_path)?;
                serde_json::from_str(&content).map_err(Into::into)
            }
        }
    }

    #[test]
    fn test_global_config_load_from_path_no_file() {
        let temp_dir = tempdir().unwrap();
        let gc = GlobalConfig::load_from_path(temp_dir.path()).unwrap();
        assert!(gc.llm.is_none());
    }

    #[test]
    fn test_global_config_load_from_path_valid_file() {
        let temp_dir = tempdir().unwrap();
        let global_content = r#"{ "llm": { "model": "global-test-model", "port": 7777 } }"#;
        create_temp_global_config(global_content, temp_dir.path()).unwrap();

        let gc = GlobalConfig::load_from_path(temp_dir.path()).unwrap();
        assert!(gc.llm.is_some());
        let llm_cfg = gc.llm.unwrap();
        assert_eq!(llm_cfg.model.unwrap(), "global-test-model");
        assert_eq!(llm_cfg.port.unwrap(), 7777);
        assert!(llm_cfg.host.is_none()); // Not specified in this global config
    }

    #[test]
    fn test_global_config_load_from_path_malformed_file() {
        let temp_dir = tempdir().unwrap();
        let global_content = r#"{ "llm": { model: "missing_quotes" } }"#; // Malformed JSON
        create_temp_global_config(global_content, temp_dir.path()).unwrap();

        let result = GlobalConfig::load_from_path(temp_dir.path());
        assert!(result.is_err());
    }

    // For Project::load, it reads "project.json" from the current directory.
    // We can create "project.json" in a temporary directory and set current_dir for the test.
    #[test]
    fn test_project_load_project_only_no_global() {
        let temp_project_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_project_dir.path()).unwrap();

        let project_content = r#"
        {
            "meta": {
                "name": "TestProject", "description": "Test Desc", "created": "2024-01-01T00:00:00Z",
                "tech_stack": [], "tags": [],
                "llm": { "model": "project-load-model", "port": 8888 }
            },
            "backlog": [], "sprints": [], "current_sprint": null
        }"#;
        fs::write("project.json", project_content).unwrap();

        // Simulate no global config by passing None to resolve_llm_config through a mock GlobalConfig::load
        // This is where the direct dependency on dirs::config_dir() in Project::load hurts.
        // To properly test Project::load's interaction with GlobalConfig::load, we'd need
        // to either mock dirs::config_dir or have Project::load take a GlobalConfig instance.

        // For now, let's assume GlobalConfig::load() returns default if no global file.
        // (This requires no actual global file exists or it's empty during test run, which is flaky)
        // A better way: Project::load could take an optional path for global_config_override for testing.

        let project = Project::load().unwrap(); // Assumes no conflicting global config or it's correctly handled
        assert_eq!(project.meta.name, "TestProject");
        assert_eq!(project.resolved_llm_config.model, "project-load-model");
        assert_eq!(project.resolved_llm_config.port, 8888);
        assert_eq!(project.resolved_llm_config.host, DEFAULT_LLM_HOST); // Default

        std::env::set_current_dir(original_dir).unwrap(); // Cleanup
        fs::remove_file(temp_project_dir.path().join("project.json")).unwrap();
    }

    #[test]
    fn test_project_load_project_and_simulated_global() {
        // This test demonstrates the difficulty of testing Project::load directly
        // without refactoring it to allow injection of GlobalConfig or its path.
        // We are testing the *resolution* part, assuming global and project parts are loaded.

        let global_partial = PartialLlmConfig { // Simulating a loaded global config
            model: Some("global-model".to_string()),
            host: Some("global-host".to_string()),
            port: Some(1111),
            timeout_ms: Some(10000),
        };

        let project_meta_llm = Some(PartialLlmConfig { // Simulating project.json's llm part
            model: Some("project-model".to_string()),
            port: Some(2222),
            ..Default::default()
        });

        let resolved = resolve_llm_config(Some(&global_partial), project_meta_llm.as_ref());
        assert_eq!(resolved.model, "project-model"); // Project overrides global
        assert_eq!(resolved.host, "global-host");   // Global (project didn't specify)
        assert_eq!(resolved.port, 2222);            // Project overrides global
        assert_eq!(resolved.timeout_ms, 10000);     // Global (project didn't specify)
    }


    #[test]
    fn test_project_load_missing_project_json() {
        let temp_project_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_project_dir.path()).unwrap();

        let result = Project::load();
        assert!(result.is_err());
        // Check for specific error context if possible/needed

        std::env::set_current_dir(original_dir).unwrap(); // Cleanup
    }

    #[test]
    fn test_project_load_malformed_project_json() {
        let temp_project_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_project_dir.path()).unwrap();

        let project_content = r#"{ "meta": { "name": "Test" }, "backlog": "not_an_array" }"#; // malformed
        fs::write("project.json", project_content).unwrap();

        let result = Project::load();
        assert!(result.is_err());

        std::env::set_current_dir(original_dir).unwrap();
        fs::remove_file(temp_project_dir.path().join("project.json")).unwrap();
    }

    #[test]
    fn test_project_validate_valid() {
        let project = Project {
            meta: ProjectMeta {
                name: "ValidProject".to_string(), description: "".to_string(), created: Utc::now(),
                tech_stack: vec![], tags: vec![], llm: None,
            },
            backlog: vec![], sprints: vec![], current_sprint: None,
            resolved_llm_config: FinalLlmConfig {
                model: "model".to_string(), host: "host".to_string(), port: 123, timeout_ms: 100,
            },
        };
        assert!(project.validate().is_ok());
    }

    #[test]
    fn test_project_validate_invalid_llm() {
        let mut project = Project {
            meta: ProjectMeta {
                name: "ValidProject".to_string(), description: "".to_string(), created: Utc::now(),
                tech_stack: vec![], tags: vec![], llm: None,
            },
            backlog: vec![], sprints: vec![], current_sprint: None,
            resolved_llm_config: FinalLlmConfig { // Valid initially
                model: "model".to_string(), host: "host".to_string(), port: 123, timeout_ms: 100,
            },
        };

        project.resolved_llm_config.model = "".to_string();
        assert!(project.validate().is_err(), "Empty model should fail validation");
        project.resolved_llm_config.model = "model".to_string(); // Reset

        project.resolved_llm_config.host = "".to_string();
        assert!(project.validate().is_err(), "Empty host should fail validation");
        project.resolved_llm_config.host = "host".to_string(); // Reset

        project.resolved_llm_config.port = 0;
        assert!(project.validate().is_err(), "Port 0 should fail validation");
    }
}