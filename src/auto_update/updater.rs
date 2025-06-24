// src/auto_update/updater.rs

use crate::config::Project;
use super::{llm_parsers, text_utils, code_gen, doc_gen}; // Import other submodules

#[derive(Debug)]
pub enum UpdateContext {
    RequirementAnalysis,
    TaskCompletion(String),
    CodeGeneration(String),
    #[allow(dead_code)]
    Documentation, // General documentation update context
}

pub struct AutoUpdater {
    project: Project,
}

impl AutoUpdater {
    pub fn new(project: Project) -> Self {
        Self { project }
    }

    pub fn process_llm_response(&mut self, llm_response: &str, context: UpdateContext) -> anyhow::Result<()> {
        match context {
            UpdateContext::RequirementAnalysis => {
                // This method will parse and add stories to self.project.backlog
                self.update_project_from_requirement_analysis(llm_response)?;
            },
            UpdateContext::TaskCompletion(task_id) => {
                // This method will update docs based on self.project and task_id
                doc_gen::update_docs_for_task_completion(&self.project, &task_id, llm_response)?;
            },
            UpdateContext::CodeGeneration(task_id) => {
                self.handle_code_generation_suggestions(&task_id, llm_response)?;
            },
            UpdateContext::Documentation => { // General documentation update
                // Assuming this might also use a structured response in the future or specific logic
                doc_gen::update_documentation(&self.project, llm_response)?;
            }
        }

        self.project.save()?; // Save project after any modification
        Ok(())
    }

    // Renamed from update_from_requirement_analysis to avoid conflict if we directly use submodule function
    // This method now lives in updater.rs and calls the necessary parsing and conversion functions.
    fn update_project_from_requirement_analysis(&mut self, llm_response: &str) -> anyhow::Result<()> {
        println!("🔄 Auto-updating project.json from LLM analysis...");

        match serde_json::from_str::<llm_parsers::LlmUserStoryResponse>(llm_response) {
            Ok(parsed_response) => {
                if parsed_response.user_stories.is_empty() {
                    println!("ℹ️ LLM response parsed successfully but contained no user stories.");
                    return Ok(());
                }

                let mut added_count = 0;
                let initial_story_count = self.project.backlog
                    .iter()
                    .filter(|item| item.id.starts_with("US-"))
                    .count();

                for llm_story in parsed_response.user_stories {
                    let story_id_num = initial_story_count + 1 + added_count;
                    let story_id = format!("US-{:03}", story_id_num);

                    // Call the function from llm_parsers module
                    match llm_parsers::convert_llm_story_to_backlog_item(llm_story, story_id) {
                        Ok(backlog_item) => {
                            self.project.backlog.push(backlog_item);
                            added_count += 1;
                        }
                        Err(e) => {
                            eprintln!("⚠️ Failed to convert an LLM user story to backlog item: {}", e);
                        }
                    }
                }

                if added_count > 0 {
                    println!("✅ Auto-added {} user stories to project.json", added_count);
                } else {
                    println!("ℹ️ No user stories were added from the LLM response despite successful parsing.");
                }
            }
            Err(e) => {
                println!("⚠️ Failed to parse LLM response as structured JSON: {}", e);
                println!("ℹ️ Attempting to extract stories from text format as a fallback...");
                // Call the function from text_utils module, passing &mut self.project
                text_utils::extract_stories_from_text(&mut self.project, llm_response)?;
            }
        }
        Ok(())
    }

    /// Get the project (immutable)
    pub fn get_project(&self) -> &Project {
        &self.project
    }

    /// Get the project (mutable - primarily for tests or internal trusted operations)
    #[cfg(test)]
    pub(crate) fn get_project_mut(&mut self) -> &mut Project {
        &mut self.project
    }

    fn handle_code_generation_suggestions(&mut self, task_id: &str, llm_response_str: &str) -> anyhow::Result<()> {
        use std::io::{self, Write};
        use super::cargo_toml_updater; // To call add_cargo_dependencies

        println!("💻 Processing LLM suggestions for task {}...", task_id);

        match llm_parsers::parse_assist_task_response(llm_response_str) {
            Ok(parsed_response) => {
                if let Some(summary) = &parsed_response.overall_summary {
                    println!("ℹ️ LLM Overall Summary: {}", summary);
                }

                let mut cargo_deps_to_add: Vec<String> = Vec::new();
                let mut source_code_suggestions: Vec<&llm_parsers::LlmSourceCodeSuggestion> = Vec::new();
                let mut general_advice: Vec<String> = Vec::new();

                for suggestion in &parsed_response.suggestions {
                    match suggestion {
                        llm_parsers::LlmSingleSuggestion::CargoDependency(deps) => {
                            println!("  - LLM suggests adding {} Cargo dependenc(ies).", deps.dependency_lines.len());
                            if let Some(notes) = &deps.notes { println!("    Notes: {}", notes); }
                            cargo_deps_to_add.extend(deps.dependency_lines.iter().cloned());
                        }
                        llm_parsers::LlmSingleSuggestion::SourceCode(code_sugg) => {
                            println!("  - LLM suggests code for '{}' (action: {:?}).", code_sugg.target_file, code_sugg.action);
                            if let Some(notes) = &code_sugg.notes { println!("    Notes: {}", notes); }
                            source_code_suggestions.push(code_sugg);
                        }
                        llm_parsers::LlmSingleSuggestion::GeneralAdvice(advice) => {
                            println!("  - LLM general advice: {}", advice.content);
                            if let Some(notes) = &advice.notes { println!("    Notes: {}", notes); }
                            general_advice.push(advice.content.clone());
                        }
                    }
                }

                // 1. Handle Cargo Dependencies with User Confirmation
                if !cargo_deps_to_add.is_empty() {
                    println!("\nProposed Cargo.toml dependencies to add:");
                    for dep in &cargo_deps_to_add {
                        println!("  - {}", dep);
                    }
                    print!("👉 Add these dependencies to Cargo.toml? (yes/no): ");
                    io::stdout().flush()?;
                    let mut user_choice = String::new();
                    io::stdin().read_line(&mut user_choice)?;

                    if user_choice.trim().to_lowercase() == "yes" || user_choice.trim().to_lowercase() == "y" {
                        // Assuming current directory is project root for finding Cargo.toml
                        let project_root = std::env::current_dir().map_err(anyhow::Error::from)?;
                        match cargo_toml_updater::add_cargo_dependencies(&project_root, &cargo_deps_to_add) {
                            Ok(_) => println!("✅ Cargo.toml updated successfully."),
                            Err(e) => eprintln!("⚠️ Failed to update Cargo.toml: {}", e),
                        }
                    } else {
                        println!("Skipped adding Cargo dependencies.");
                    }
                }

                // 2. Handle Source Code Suggestions (Placeholder for now)
                if !source_code_suggestions.is_empty() {
                    println!("\nLLM suggested the following source code changes:");
                    for (idx, code_sugg) in source_code_suggestions.iter().enumerate() {
                        println!("  {}. Action: {:?} for file: {}", idx + 1, code_sugg.action, code_sugg.target_file);
                        println!("     Content (first 80 chars): {:.80}...", code_sugg.content.chars().take(80).collect::<String>());
                         // Here, you would implement logic to apply these changes,
                         // potentially with user confirmation for each.
                         // For now, we just print them.
                         // The old `code_gen::generate_code_files` could be a temporary fallback
                         // for simple "create" actions if no target_file is specified,
                         // but it's better to build out proper handling.
                    }
                    println!("👉 Source code modifications based on LLM suggestions need to be reviewed and applied (partially or fully automated in future).");
                    // As a fallback, call the old code_gen for any raw code blocks if desired,
                    // or simply state that these need manual application for now.
                    // For this phase, we are focusing on Cargo.toml.
                    // The old `code_gen::generate_code_files` took the whole llm_response string.
                    // We might keep it as a fallback if no structured suggestions are found,
                    // or if only general_advice is present with code blocks.
                    // For now, let's rely on structured output.
                    println!("   (Skipping direct file modifications in this phase, focusing on Cargo.toml changes first).");
                }
                 if !general_advice.is_empty() && source_code_suggestions.is_empty() && cargo_deps_to_add.is_empty() {
                     println!("\nℹ️ LLM provided general advice but no specific code or dependency changes were parsed for automation.");
                     // Fallback to old raw code block extraction if only general advice was found,
                     // as the advice might contain unformatted code blocks.
                     println!("   Attempting fallback to raw code block extraction for any embedded code snippets...");
                     code_gen::generate_code_files(&self.project, task_id, llm_response_str)?;
                 }


            }
            Err(e) => {
                eprintln!("⚠️ Failed to parse structured LLM response for task {}: {}", task_id, e);
                eprintln!("   Falling back to raw code block extraction for task {}...", task_id);
                // Fallback to the old generate_code_files if parsing the new structure fails
                code_gen::generate_code_files(&self.project, task_id, llm_response_str)?;
            }
        }
        Ok(())
    }
}
