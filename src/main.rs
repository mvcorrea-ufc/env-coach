// src/main.rs
use anyhow::Result;
use clap::{Parser, Subcommand};

mod config;
mod scripts;
mod auto_update;
mod ollama;
mod templates;

#[derive(Parser)]
#[command(name = "env-coach")]
#[command(about = "Environment Coach - AI-powered project management")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project (uses current directory name by default)
    Init {
        /// Project name (optional - uses current directory name if not specified)
        name: Option<String>,
        /// Project description
        #[arg(short, long)]
        description: Option<String>,
        /// Problem statement for the project (PRD)
        #[arg(long)]
        problem: Option<String>,
        /// Success metric for the project (PRD) - can be specified multiple times
        #[arg(long = "metric")]
        metrics: Vec<String>,
    },
    /// Add a new requirement
    AddRequirement {
        /// Requirement description
        requirement: String,
    },
    /// List backlog items
    ListBacklog,
    /// Show project status
    Status,
    /// Plan a new sprint
    PlanSprint {
        /// Sprint goal
        #[arg(short, long)]
        goal: String,
        /// Sprint duration in days
        #[arg(short, long, default_value = "14")]
        days: u32,
    },
    /// Start a sprint
    StartSprint {
        /// Sprint ID
        sprint_id: String,
    },
    /// Show current sprint
    ShowSprint,
    /// Start working on a task
    StartTask {
        /// Task ID
        task_id: String,
    },
    /// Get LLM assistance for a task
    AssistTask {
        /// Task ID
        task_id: String,
    },
    /// Complete a task
    CompleteTask {
        /// Task ID
        task_id: String,
    },
    /// Add a user story manually
    AddStory {
        /// Story title
        #[arg(short, long)]
        title: String,
        /// Story description
        #[arg(short, long)]
        description: String,
    },
    /// List all user stories
    ListStories,
    /// Send custom prompt to LLM
    LlmCycle {
        /// Prompt text or file path
        #[arg(short, long)]
        prompt: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, description, problem, metrics } => { // Added problem, metrics
            scripts::init::run(name, description, problem, metrics)?; // Pass new args
        }
        Commands::AddRequirement { requirement } => {
            scripts::requirements::process_requirement(requirement).await?;
        }
        Commands::ListBacklog => {
            scripts::backlog::run()?;
        }
        Commands::Status => {
            scripts::status::run().await?;
        }
        Commands::PlanSprint { goal, days } => {
            scripts::sprint::plan(goal, days).await?; // Added .await
        }
        Commands::StartSprint { sprint_id } => {
            scripts::sprint::start_sprint(sprint_id)?;
        }
        Commands::ShowSprint => {
            scripts::sprint::show_current_sprint()?;
        }
        Commands::StartTask { task_id } => {
            scripts::tasks::start_task(task_id)?;
        }
        Commands::AssistTask { task_id } => {
            scripts::tasks::assist_task(task_id).await?;
        }
        Commands::CompleteTask { task_id } => {
            scripts::tasks::complete_task(task_id)?;
        }
        Commands::AddStory { title, description } => {
            scripts::stories::add_manual_story(title, description)?;
        }
        Commands::ListStories => {
            scripts::stories::list_stories()?;
        }
        Commands::LlmCycle { prompt } => {
            scripts::llm_cycle::run(prompt).await?;
        }
    }

    Ok(())
}