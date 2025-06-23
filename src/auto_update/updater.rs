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
                // This method generates files based on project context and llm_response
                code_gen::generate_code_files(&self.project, &task_id, llm_response)?;
            },
            UpdateContext::Documentation => { // General documentation update
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
}
