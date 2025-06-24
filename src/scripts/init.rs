// src/scripts/init.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use crate::config::{Project, GlobalConfig, Prd}; // Added Prd
use crate::templates::Templates;

pub fn run(
    name: Option<String>,
    description: Option<String>,
    problem: Option<String>,
    metrics: Vec<String>,
    description_file: Option<String>, // Added description_file
) -> Result<()> {
    // Load global config first to pass to Project::new or Project::create_in_current_dir
    let global_config = GlobalConfig::load().context("Failed to load global env-coach configuration")?;
    let global_llm_cfg_ref = global_config.llm.as_ref();

    // Check if project is already initialized
    if Project::is_initialized() {
        println!("⚠️  Project already initialized (project.json exists)");
        println!("💡 Use other commands to manage your existing project:");
        println!("   env-coach status                    # View project status");
        println!("   env-coach list-backlog              # View backlog items");
        println!("   env-coach add-requirement \"...\"     # Add new requirements");
        return Ok(());
    }

    // Get project name - use current directory name if not provided
    let project_name = match name {
        Some(provided_name) => {
            println!("🚀 Initializing env-coach project: {}", provided_name);
            provided_name
        }
        None => {
            let current_dir = std::env::current_dir()
                .context("Failed to get current directory")?;
            let dir_name = current_dir
                .file_name()
                .context("Failed to get directory name")?
                .to_string_lossy()
                .to_string();
            println!("🚀 Initializing env-coach project: {} (from current directory)", dir_name);
            dir_name
        }
    };

    // Determine project description
    let mut final_project_description = description.unwrap_or_else(|| {
        format!("AI-assisted development project for {}", project_name)
    });

    if let Some(desc_file_path) = description_file {
        match fs::read_to_string(&desc_file_path) {
            Ok(content) => {
                if !content.trim().is_empty() {
                    final_project_description = content.trim().to_string();
                    println!("ℹ️ Using project description from file: {}", desc_file_path);
                } else {
                    println!("⚠️ Description file '{}' is empty. Using provided or default description.", desc_file_path);
                }
            }
            Err(e) => {
                println!("⚠️ Failed to read description file '{}': {}. Using provided or default description.", desc_file_path, e);
            }
        }
    }

    println!("📝 Project description: {}", final_project_description);

    // Create the project configuration
    // Pass global_llm_cfg_ref to Project::new
    let mut project = Project::new(project_name.clone(), final_project_description, global_llm_cfg_ref); // Use final_project_description

    // Populate PRD if provided
    if problem.is_some() || !metrics.is_empty() {
        let prd_content = Prd {
            problem: problem.unwrap_or_default(),
            success_metrics: metrics,
        };
        project.meta.prd = Some(prd_content);
        println!("📄 PRD information captured.");
    }

    // Validate the project before saving
    project.validate()
        .context("Project validation failed")?;

    // Save project.json
    project.save()
        .context("Failed to save project.json")?;

    println!("✅ Created project.json");

    // Create README.md if it doesn't exist
    let readme_path = "README.md";
    if !Path::new(readme_path).exists() {
        let readme_content = Templates::readme_template(&project_name);
        fs::write(readme_path, readme_content)
            .context("Failed to create README.md")?;
        println!("✅ Created README.md");
    } else {
        println!("📄 README.md already exists - skipping");
    }

    // Create .gitignore additions if .gitignore exists
    let gitignore_path = ".gitignore";
    if Path::new(gitignore_path).exists() {
        let mut gitignore_content = fs::read_to_string(gitignore_path)
            .context("Failed to read .gitignore")?;
        
        let additions = Templates::gitignore_additions();
        if !gitignore_content.contains("# env-coach") {
            gitignore_content.push_str(additions);
            fs::write(gitignore_path, gitignore_content)
                .context("Failed to update .gitignore")?;
            println!("✅ Updated .gitignore with env-coach entries");
        } else {
            println!("📄 .gitignore already contains env-coach entries - skipping");
        }
    } else {
        // Create new .gitignore with env-coach entries
        let gitignore_content = format!(
            "# Rust\ntarget/\nCargo.lock\n\n# env-coach{}", 
            Templates::gitignore_additions()
        );
        fs::write(gitignore_path, gitignore_content)
            .context("Failed to create .gitignore")?;
        println!("✅ Created .gitignore");
    }

    // Create .env-coach directory for future use
    let env_coach_dir = ".env-coach";
    if !Path::new(env_coach_dir).exists() {
        fs::create_dir_all(format!("{}/cache", env_coach_dir))
            .context("Failed to create .env-coach/cache directory")?;
        fs::create_dir_all(format!("{}/logs", env_coach_dir))
            .context("Failed to create .env-coach/logs directory")?;

        // Create prompts directory and default prompts
        let prompts_dir = Path::new(env_coach_dir).join("prompts");
        fs::create_dir_all(&prompts_dir)
            .context("Failed to create .env-coach/prompts directory")?;

        Templates::create_default_prompt_if_missing(
            &prompts_dir,
            "requirements_analyst.md",
            Templates::default_requirements_analyst_prompt_content()
        ).context("Failed to create default requirements_analyst.md prompt")?;

        Templates::create_default_prompt_if_missing(
            &prompts_dir,
            "sprint_planner.md",
            Templates::default_sprint_planner_prompt_content()
        ).context("Failed to create default sprint_planner.md prompt")?;

        Templates::create_default_prompt_if_missing(
            &prompts_dir,
            "task_assistant.md",
            Templates::default_task_assistant_prompt_content()
        ).context("Failed to create default task_assistant.md prompt")?;

        // TODO: Add other default prompts here in the future e.g. code_reviewer.md

        println!("✅ Created .env-coach/ directory structure and default prompts.");
    }

    println!();
    println!("🎉 Project '{}' initialized successfully!", project_name);
    println!();
    println!("🎯 Next steps:");
    println!("   env-coach status                    # Check LLM connectivity");
    println!("   env-coach add-requirement \"...\"     # Add your first requirement");
    println!("   env-coach list-backlog              # View generated backlog");
    println!("   env-coach plan-sprint --goal \"...\"  # Plan your first sprint");
    println!();
    println!("📚 Learn more:");
    println!("   env-coach --help                    # View all commands");
    println!("   cat README.md                       # Read project documentation");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use crate::config::Project; // For loading and checking project.json

    #[test]
    fn test_init_run_with_prd_info() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let project_name = Some("TestPRDProject".to_string());
        let description = Some("A project to test PRD init".to_string());
        let problem = Some("The main problem is testing this feature.".to_string());
        let metrics = vec!["Metric1".to_string(), "Metric2".to_string()];

        run(project_name.clone(), description.clone(), problem.clone(), metrics.clone(), None).unwrap(); // Added None for description_file

        // Load the created project.json and verify its contents
        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path)
            .expect("Test: Failed to read project.json after init run");
        let loaded_project: Project = serde_json::from_str(&project_content_str)
            .expect("Test: Failed to parse project.json content");

        assert_eq!(loaded_project.meta.name, project_name.unwrap());
        assert!(loaded_project.meta.prd.is_some());
        let prd = loaded_project.meta.prd.unwrap();
        assert_eq!(prd.problem, problem.unwrap());
        assert_eq!(prd.success_metrics, metrics);

        // Cleanup: remove project.json and restore original directory
        fs::remove_file("project.json").unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_run_without_prd_info() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let project_name = Some("TestNoPRDProject".to_string());
        let description = Some("A project to test no PRD init".to_string());

        // No PRD info provided
        run(project_name.clone(), description.clone(), None, vec![], None).unwrap(); // Added None for description_file

        // ---- Debug Start ----
        let project_json_content = fs::read_to_string("project.json")
            .expect("Failed to read project.json for debugging");
        println!("DEBUG: Content of project.json:\nSTART_OF_JSON\n{}\nEND_OF_JSON", project_json_content);
        // ---- Debug End ----

        let loaded_project = Project::load().expect("Failed to load project.json in test");

        assert_eq!(loaded_project.meta.name, project_name.unwrap());
        assert!(loaded_project.meta.prd.is_none(), "PRD should be None when not provided");

        fs::remove_file("project.json").unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_run_with_partial_prd_info_problem_only() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let problem = Some("Only a problem statement.".to_string());
        run(Some("ProblemOnly".to_string()), None, problem.clone(), vec![], None).unwrap(); // Added None for description_file

        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path)
            .expect("Test: Failed to read project.json after init run");
        let loaded_project: Project = serde_json::from_str(&project_content_str)
            .expect("Test: Failed to parse project.json content");

        assert!(loaded_project.meta.prd.is_some());
        assert_eq!(loaded_project.meta.prd.as_ref().unwrap().problem, problem.unwrap());
        assert!(loaded_project.meta.prd.as_ref().unwrap().success_metrics.is_empty());

        fs::remove_file("project.json").unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_run_with_partial_prd_info_metrics_only() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let metrics = vec!["Metric A".to_string()];
        run(Some("MetricsOnly".to_string()), None, None, metrics.clone(), None).unwrap(); // Added None for description_file

        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path)
            .expect("Test: Failed to read project.json after init run");
        let loaded_project: Project = serde_json::from_str(&project_content_str)
            .expect("Test: Failed to parse project.json content");

        assert!(loaded_project.meta.prd.is_some());
        assert!(loaded_project.meta.prd.as_ref().unwrap().problem.is_empty());
        assert_eq!(loaded_project.meta.prd.as_ref().unwrap().success_metrics, metrics);

        fs::remove_file("project.json").unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_run_with_description_file() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let desc_content = "Description from a file.".to_string();
        let desc_file_path = temp_dir.path().join("desc.txt");
        fs::write(&desc_file_path, &desc_content).unwrap();

        run(
            Some("DescFileProject".to_string()),
            None, // No direct --description
            None,
            vec![],
            Some(desc_file_path.to_str().unwrap().to_string())
        ).unwrap();

        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path).unwrap();
        let loaded_project: Project = serde_json::from_str(&project_content_str).unwrap();

        assert_eq!(loaded_project.meta.description, desc_content);

        fs::remove_file(project_json_path).unwrap();
        fs::remove_file(desc_file_path).unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_run_with_description_file_takes_precedence() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let direct_desc = "Direct description from flag.".to_string();
        let file_desc = "Description from file (should win).".to_string();
        let desc_file_path = temp_dir.path().join("desc.txt");
        fs::write(&desc_file_path, &file_desc).unwrap();

        run(
            Some("DescFilePrecedence".to_string()),
            Some(direct_desc),
            None,
            vec![],
            Some(desc_file_path.to_str().unwrap().to_string())
        ).unwrap();

        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path).unwrap();
        let loaded_project: Project = serde_json::from_str(&project_content_str).unwrap();

        assert_eq!(loaded_project.meta.description, file_desc);

        fs::remove_file(project_json_path).unwrap();
        fs::remove_file(desc_file_path).unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_run_with_missing_description_file_falls_back() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let direct_desc = "Fallback description.".to_string();
        let missing_file_path = "non_existent_desc.txt"; // Does not exist

        // Expect a warning to be printed, but the run should succeed using direct_desc
        run(
            Some("MissingDescFile".to_string()),
            Some(direct_desc.clone()),
            None,
            vec![],
            Some(missing_file_path.to_string())
        ).unwrap();

        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path).unwrap();
        let loaded_project: Project = serde_json::from_str(&project_content_str).unwrap();

        assert_eq!(loaded_project.meta.description, direct_desc);

        fs::remove_file(project_json_path).unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_run_with_empty_description_file_falls_back() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let direct_desc = "Fallback for empty file.".to_string();
        let desc_file_path = temp_dir.path().join("empty_desc.txt");
        fs::write(&desc_file_path, "").unwrap(); // Empty file

        run(
            Some("EmptyDescFile".to_string()),
            Some(direct_desc.clone()),
            None,
            vec![],
            Some(desc_file_path.to_str().unwrap().to_string())
        ).unwrap();

        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path).unwrap();
        let loaded_project: Project = serde_json::from_str(&project_content_str).unwrap();

        assert_eq!(loaded_project.meta.description, direct_desc);

        fs::remove_file(project_json_path).unwrap();
        fs::remove_file(desc_file_path).unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_init_creates_project_with_default_llm_config() {
        let temp_dir = tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let project_name = Some("DefaultLLMProject".to_string());
        run(project_name.clone(), None, None, vec![], None).unwrap();

        let project_json_path = temp_dir.path().join("project.json");
        let project_content_str = fs::read_to_string(&project_json_path)
            .expect("Test: Failed to read project.json after init run");
        let loaded_project: Project = serde_json::from_str(&project_content_str)
            .expect("Test: Failed to parse project.json content");

        assert_eq!(loaded_project.meta.name, project_name.unwrap());
        assert!(loaded_project.meta.llm.is_some(), "meta.llm should be Some");

        let llm_config = loaded_project.meta.llm.unwrap();
        assert_eq!(llm_config.host.as_deref(), Some(crate::config::DEFAULT_LLM_HOST));
        assert_eq!(llm_config.port, Some(crate::config::DEFAULT_LLM_PORT));
        assert_eq!(llm_config.model.as_deref(), Some(crate::config::DEFAULT_LLM_MODEL));
        assert_eq!(llm_config.timeout_ms, Some(60000)); // User-specified default for new projects

        fs::remove_file(project_json_path).unwrap();
        std::env::set_current_dir(original_dir).unwrap();
    }
}