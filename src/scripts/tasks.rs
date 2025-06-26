// src/scripts/tasks.rs
use anyhow::{Context, Result};
// use reqwest; // Unused
// use serde_json::Value; // Unused
use crate::config::{Project, Status}; // Removed FinalLlmConfig as it's not directly used here
use crate::auto_update::{AutoUpdater, UpdateContext}; // NEW: Import auto-update

pub fn start_task(id: String) -> Result<()> {
    let mut project = Project::load()
        .context("Failed to load project. Run 'env-coach init <n>' first")?;

    // Find the task
    let task_index = project.backlog
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;

    // Update status to In Progress
    project.backlog[task_index].status = Status::InProgress;

    // Store task details for printing (before saving)
    let task_title = project.backlog[task_index].title.clone();
    let task_story = project.backlog[task_index].story.clone();
    let task_priority = project.backlog[task_index].priority.clone();
    let task_effort = project.backlog[task_index].effort;
    let task_criteria = project.backlog[task_index].acceptance_criteria.clone();

    project.save()
        .context("Failed to save project")?;

    println!("🚀 Starting task: {}", id);
    println!("✅ Task {} status updated to 'In Progress'", id);
    println!("📋 Task Details:");
    println!("   Title: {}", task_title);
    println!("   Story: {}", task_story);
    println!("   Priority: {:?}", task_priority);
    println!("   Effort: {} points", task_effort);
    println!("   Acceptance Criteria:");
    for (i, criteria) in task_criteria.iter().enumerate() {
        println!("     {}. {}", i + 1, criteria);
    }
    
    println!("🤖 Need LLM assistance?");
    println!("   env-coach assist-task {}", id);
    println!("⏯️  When done:");
    println!("   env-coach complete-task {}", id);

    Ok(())
}

pub async fn assist_task(task_id: String, user_prompt_override: Option<String>) -> Result<()> {
    use crate::templates::Templates; // For default prompt
    use crate::ollama; // For send_generation_prompt
    use crate::config::BacklogItem; // To type hint `task`

    let project = Project::load().context("Failed to load project. Run 'env-coach init' first.")?;

    println!("🤖 Providing LLM assistance for task: {}", task_id);

    let task: &BacklogItem = project.backlog.iter()
        .find(|item| item.id == task_id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found in backlog.", task_id))?;

    println!("📋 Task Details:");
    println!("   Title: {}", task.title);
    println!("   Story: {}", task.story); // Assuming BacklogItem has a story field
    // ... print other task details as before ...
    println!("   Priority: {:?}", task.priority);
    println!("   Effort: {} points", task.effort);
    if !task.acceptance_criteria.is_empty() {
        println!("   Acceptance Criteria:");
        for (i, criteria) in task.acceptance_criteria.iter().enumerate() {
            println!("     {}. {}", i + 1, criteria);
        }
    }

    println!("🔍 Preparing prompt and asking LLM for assistance...");

    // 1. Load Prompt Template
    let prompt_template_path = std::path::Path::new(".env-coach/prompts/task_assistant.md");
    let prompt_template = match std::fs::read_to_string(prompt_template_path) {
        Ok(template) => template,
        Err(_) => {
            println!("⚠️ Task assistant prompt not found at {:?}. Using default.", prompt_template_path);
            Templates::default_task_assistant_prompt_content()
        }
    };

    // 2. Format Prompt
    // Determine primary language (this function needs to be accessible, e.g. from auto_update::code_gen or a shared util)
    // For now, let's assume it's available via project or a new helper here.
    // We'll use the one from auto_update::code_gen for consistency.
    let primary_language = crate::auto_update::code_gen::get_primary_language(&project.meta);

    let ac_string = task.acceptance_criteria.iter()
        .map(|ac| format!("  - {}", ac))
        .collect::<Vec<String>>().join("\n");

    let mut filled_prompt = prompt_template;
    filled_prompt = filled_prompt.replace("{{project_name}}", &project.meta.name);
    filled_prompt = filled_prompt.replace("{{project_description}}", &project.meta.description);
    filled_prompt = filled_prompt.replace("{{tech_stack}}", &project.meta.tech_stack.join(", "));
    filled_prompt = filled_prompt.replace("{{primary_language}}", &primary_language);
    filled_prompt = filled_prompt.replace("{{tags}}", &project.get_tags_display());
    filled_prompt = filled_prompt.replace("{{task_id}}", &task.id);
    filled_prompt = filled_prompt.replace("{{task_title}}", &task.title);
    filled_prompt = filled_prompt.replace("{{task_story}}", &task.story);
    filled_prompt = filled_prompt.replace("{{#each task_acceptance_criteria}}", ""); // Remove loop markers
    filled_prompt = filled_prompt.replace("{{/each}}", "");
    filled_prompt = filled_prompt.replace("  - {{this}}", &ac_string); // Replace the iterated part

    let user_query = user_prompt_override.unwrap_or_else(|| "Provide general assistance and next steps for this task.".to_string());
    filled_prompt = filled_prompt.replace("{{user_prompt}}", &user_query);

    // 3. Send to LLM
    let llm_response_str = ollama::send_generation_prompt(project.llm(), &filled_prompt)
        .await
        .context("Failed to get LLM assistance for task")?;

    // 4. Process with AutoUpdater
    // We print the raw response for now, AutoUpdater will handle parsing and actions.
    println!("\n🤖 LLM Raw Response (JSON expected):");
    println!("{}", llm_response_str);

    let mut updater = AutoUpdater::new(project); // project is moved here
    // assist_task is for assistance; it won't auto-approve. Pass false for auto-approve flags.
    updater.process_llm_response(&llm_response_str, UpdateContext::CodeGeneration(task_id.clone()), false, false)
        .context("Failed to process LLM suggestions or auto-update files")?;
    // Note: `project` is consumed by AutoUpdater. If we need it afterwards, AutoUpdater must return it or operate on &mut.
    // Current AutoUpdater::new takes ownership, and save is called internally.

    println!("\n💡 Review the LLM suggestions and generated/modified files (if any).");
    println!("💡 When ready to mark task complete: env-coach complete-task {}", task_id);

    Ok(())
}

pub fn complete_task(id: String) -> Result<()> {
    let mut project = Project::load()
        .context("Failed to load project. Run 'env-coach init <n>' first")?;

    // Find the task
    let task = project.backlog
        .iter_mut()
        .find(|item| item.id == id)
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found", id))?;

    // Update status to Done
    task.status = Status::Done;
    
    // Update sprint progress if task is in a sprint
    if let Some(sprint_id) = &task.sprint {
        if let Some(sprint) = project.sprints.iter_mut().find(|s| s.id == *sprint_id) {
            sprint.completed_points += task.effort;
        }
    }

    println!("✅ Completing task: {}", id);
    println!("📋 Task '{}' marked as Done", task.title);

    // NEW: Auto-update documentation
    let mut updater = AutoUpdater::new(project);
    // Pass default false for auto_approve flags
    updater.process_llm_response("", UpdateContext::TaskCompletion(id.clone()), false, false)
        .context("Failed to auto-update documentation")?;

    // Save the updated project
    let updated_project = updater.get_project();
    updated_project.save()
        .context("Failed to save project")?;

    println!("📝 Documentation auto-updated (README.md, CHANGELOG.md)");
    
    // Show updated sprint progress if applicable
    if let Some(task) = updated_project.backlog.iter().find(|item| item.id == id) {
        if let Some(sprint_id) = &task.sprint {
            if let Some(sprint) = updated_project.sprints.iter().find(|s| s.id == *sprint_id) {
                let progress_percent = if sprint.total_points > 0 {
                    (sprint.completed_points * 100) / sprint.total_points
                } else {
                    0
                };
                
                println!("📊 Sprint Progress: {} / {} points ({}%)", 
                    sprint.completed_points, sprint.total_points, progress_percent);
            }
        }
    }

    println!("🎯 Next steps:");
    println!("   env-coach show-sprint               # View current sprint status");
    println!("   env-coach start-task <next-id>      # Start next task");

    Ok(())
}

pub async fn execute_task(
    task_id: String,
    user_prompt_override: Option<String>,
    auto_approve_deps: bool,
    auto_approve_code: bool
) -> Result<()> {
    use crate::templates::Templates;
    use crate::ollama;
    use crate::config::BacklogItem;

    println!("🚀 Executing task: {} (Auto-approve deps: {}, Auto-approve code: {})", task_id, auto_approve_deps, auto_approve_code);

    let mut project = Project::load().context("Failed to load project. Run 'env-coach init' first.")?;

    // Mark task as InProgress and save early
    let task_idx = project.backlog.iter_mut()
        .position(|item| item.id == task_id && matches!(item.status, Status::Todo)) // Only start if Todo
        .ok_or_else(|| anyhow::anyhow!("Task '{}' not found or not in 'Todo' status.", task_id))?;

    project.backlog[task_idx].status = Status::InProgress;
    let task_title_clone = project.backlog[task_idx].title.clone(); // Clone before project is moved
    project.save().context("Failed to save task status update to InProgress")?;

    println!("🔄 Task '{}' status updated to InProgress.", task_id);

    // Reload project to pass ownership to AutoUpdater later, or clone necessary parts.
    // For simplicity, let's reload. A more optimized way would be to manage ownership carefully.
    let project = Project::load().context("Failed to reload project after status update.")?;

    // Get task details again from the reloaded project
    let task: &BacklogItem = project.backlog.iter()
        .find(|item| item.id == task_id) // Status is now InProgress
        .ok_or_else(|| anyhow::anyhow!("Task '{}' disappeared after reload. This should not happen.", task_id))?;

    println!("📋 Task Details for LLM Assistance:");
    println!("   Title: {}", task.title);
    // ... (print other details as in assist_task) ...

    // 1. Load Prompt Template
    let prompt_template_path = std::path::Path::new(".env-coach/prompts/task_assistant.md");
    let prompt_template = match std::fs::read_to_string(prompt_template_path) {
        Ok(template) => template,
        Err(_) => {
            println!("⚠️ Task assistant prompt not found at {:?}. Using default.", prompt_template_path);
            Templates::default_task_assistant_prompt_content()
        }
    };

    // 2. Format Prompt (similar to assist_task)
    let primary_language = crate::auto_update::code_gen::get_primary_language(&project.meta);
    let ac_string = task.acceptance_criteria.iter().map(|ac| format!("  - {}", ac)).collect::<Vec<String>>().join("\n");
    let mut filled_prompt = prompt_template;
    // ... (all placeholder replacements as in assist_task's refactored version) ...
    filled_prompt = filled_prompt.replace("{{project_name}}", &project.meta.name);
    filled_prompt = filled_prompt.replace("{{project_description}}", &project.meta.description);
    filled_prompt = filled_prompt.replace("{{tech_stack}}", &project.meta.tech_stack.join(", "));
    filled_prompt = filled_prompt.replace("{{primary_language}}", &primary_language);
    filled_prompt = filled_prompt.replace("{{tags}}", &project.get_tags_display());
    filled_prompt = filled_prompt.replace("{{task_id}}", &task.id);
    filled_prompt = filled_prompt.replace("{{task_title}}", &task.title);
    filled_prompt = filled_prompt.replace("{{task_story}}", &task.story);
    filled_prompt = filled_prompt.replace("{{#each task_acceptance_criteria}}", "");
    filled_prompt = filled_prompt.replace("{{/each}}", "");
    filled_prompt = filled_prompt.replace("  - {{this}}", &ac_string);
    let user_query = user_prompt_override.unwrap_or_else(|| format!("Provide implementation steps and code for task: {}", task.title));
    filled_prompt = filled_prompt.replace("{{user_prompt}}", &user_query);

    println!("🔍 Sending request to LLM for task '{}' ({}) ...", task.title, task_id);

    // 3. Send to LLM
    let llm_response_str = ollama::send_generation_prompt(project.llm(), &filled_prompt)
        .await
        .context(format!("Failed to get LLM assistance for task {}", task_id))?;

    println!("\n🤖 LLM Raw Response (JSON expected):"); // Good for debugging
    println!("{}", llm_response_str);


    // 4. Process with AutoUpdater, passing auto-approve flags
    let mut updater = AutoUpdater::new(project);
    // AutoUpdater::process_llm_response needs to be aware of these flags.
    // This requires modifying AutoUpdater::process_llm_response or its sub-methods.
    // For now, let's assume process_llm_response is modified or a new method is added.
    // Let's define a new method in AutoUpdater: process_structured_assist_response

    // This call will be updated once AutoUpdater is modified.
    // For now, it will use the existing process_llm_response which doesn't know about auto_approve.
    updater.process_llm_response(&llm_response_str, UpdateContext::CodeGeneration(task_id.clone()), auto_approve_deps, auto_approve_code)
        .context(format!("Failed to process LLM suggestions or auto-update files for task {}", task_id))?;

    println!("\n✅ Task execution phase for '{}' ({}) complete.", task_title_clone, task_id);
    println!("   Review any changes made (e.g., Cargo.toml, generated files).");
    println!("   Manually apply any source code suggestions if not auto-applied.");
    println!("💡 When task is fully implemented and tested, run: env-coach complete-task {}", task_id);

    Ok(())
}


// Old `send_llm_assistance_request` and its helpers (`get_primary_language`,
// `get_language_guidance`, `get_code_block_language`) are removed.
// The new `assist_task` directly loads the prompt template, formats it,
// calls `ollama::send_generation_prompt`, and then passes the response
// to `AutoUpdater`. The `get_primary_language` logic is now centralized
// in `auto_update::code_gen`.