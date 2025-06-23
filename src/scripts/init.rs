// src/scripts/init.rs
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use crate::config::{Project, GlobalConfig}; // Added GlobalConfig
use crate::templates::Templates;

pub fn run(name: Option<String>, description: Option<String>) -> Result<()> {
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

    // Get project description
    let project_description = description.unwrap_or_else(|| {
        format!("AI-assisted development project for {}", project_name)
    });

    println!("📝 Project description: {}", project_description);

    // Create the project configuration
    // Pass global_llm_cfg_ref to Project::new
    let project = Project::new(project_name.clone(), project_description, global_llm_cfg_ref);

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
        println!("✅ Created .env-coach/ directory structure");
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