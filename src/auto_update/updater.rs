// src/auto_update/updater.rs

use crate::config::Project;
use super::{llm_parsers, text_utils, code_gen, doc_gen, cargo_toml_updater};
use std::io::{self, Write, BufRead}; // Added BufRead for test input handling

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

    // Add auto_approve flags to this method's signature
    pub fn process_llm_response(
        &mut self,
        llm_response: &str,
        context: UpdateContext,
        auto_approve_deps: bool,
        auto_approve_code: bool
    ) -> anyhow::Result<()> {
        // For tests that don't care about input, provide a dummy empty iterator
        let mut dummy_test_input = Vec::<String>::new().into_iter(); // Consumed by type system if not &mut dyn
                                                                    // This needs to be handled carefully if test_input_lines is passed through.
                                                                    // Let's make test_input_lines specific to handle_code_generation_suggestions for now.

        match context {
            UpdateContext::RequirementAnalysis => {
                self.update_project_from_requirement_analysis(llm_response)?;
            },
            UpdateContext::TaskCompletion(task_id) => {
                doc_gen::update_docs_for_task_completion(&self.project, &task_id, llm_response)?;
            },
            UpdateContext::CodeGeneration(task_id) => {
                // Pass None for test_input_lines for real execution
                self.handle_code_generation_suggestions(&task_id, llm_response, auto_approve_deps, auto_approve_code, None)?;
            },
            UpdateContext::Documentation => {
                doc_gen::update_documentation(&self.project, llm_response)?;
            }
        }

        self.project.save()?;
        Ok(())
    }

    // Test-specific version of process_llm_response to inject input
    #[cfg(test)]
    pub fn process_llm_response_for_test(
        &mut self,
        llm_response: &str,
        context: UpdateContext,
        auto_approve_deps: bool,
        auto_approve_code: bool,
        mut test_input_iter: Option<&mut dyn Iterator<Item = String>>
    ) -> anyhow::Result<()> {
         match context {
            UpdateContext::RequirementAnalysis => {
                self.update_project_from_requirement_analysis(llm_response)?;
            },
            UpdateContext::TaskCompletion(task_id) => {
                doc_gen::update_docs_for_task_completion(&self.project, &task_id, llm_response)?;
            },
            UpdateContext::CodeGeneration(task_id) => {
                self.handle_code_generation_suggestions(&task_id, llm_response, auto_approve_deps, auto_approve_code, test_input_iter)?;
            },
            UpdateContext::Documentation => {
                doc_gen::update_documentation(&self.project, llm_response)?;
            }
        }
        self.project.save()?;
        Ok(())
    }


    fn update_project_from_requirement_analysis(&mut self, llm_response: &str) -> anyhow::Result<()> {
        println!("🔄 Auto-updating project.json from LLM analysis...");
        match serde_json::from_str::<llm_parsers::LlmUserStoryResponse>(llm_response) {
            Ok(parsed_response) => {
                if parsed_response.user_stories.is_empty() {
                    println!("ℹ️ LLM response parsed successfully but contained no user stories.");
                    return Ok(());
                }
                let mut added_count = 0;
                let initial_story_count = self.project.backlog.iter().filter(|item| item.id.starts_with("US-")).count();
                for llm_story in parsed_response.user_stories {
                    let story_id_num = initial_story_count + 1 + added_count;
                    let story_id = format!("US-{:03}", story_id_num);
                    match llm_parsers::convert_llm_story_to_backlog_item(llm_story, story_id) {
                        Ok(backlog_item) => {
                            self.project.backlog.push(backlog_item);
                            added_count += 1;
                        }
                        Err(e) => { eprintln!("⚠️ Failed to convert an LLM user story to backlog item: {}", e); }
                    }
                }
                if added_count > 0 { println!("✅ Auto-added {} user stories to project.json", added_count); }
                else { println!("ℹ️ No user stories were added from the LLM response despite successful parsing."); }
            }
            Err(e) => {
                println!("⚠️ Failed to parse LLM response as structured JSON: {}", e);
                println!("ℹ️ Attempting to extract stories from text format as a fallback...");
                text_utils::extract_stories_from_text(&mut self.project, llm_response)?;
            }
        }
        Ok(())
    }

    fn handle_code_generation_suggestions(
        &mut self,
        task_id: &str,
        llm_response_str: &str,
        auto_approve_deps: bool,
        auto_approve_code: bool,
        mut test_input_lines: Option<&mut dyn Iterator<Item = String>> // New parameter
    ) -> anyhow::Result<()> {
        println!("💻 Processing LLM suggestions for task {}...", task_id);

        // Helper function to get a line of input, from test iterator or stdin
        let mut read_testable_line = |prompt_text: &str| -> anyhow::Result<String> {
            if let Some(ref mut iter) = test_input_lines {
                if let Some(line) = iter.next() {
                    println!("{} (Mocked test input: {})", prompt_text, line.trim());
                    return Ok(line);
                } else {
                    return Err(anyhow::anyhow!("Test input iterator exhausted when expecting input for: {}", prompt_text));
                }
            } else {
                print!("{}", prompt_text);
                io::stdout().flush()?;
                let mut buffer = String::new();
                io::stdin().read_line(&mut buffer)?;
                Ok(buffer)
            }
        };

        match llm_parsers::parse_assist_task_response(llm_response_str) {
            Ok(parsed_response) => {
                if let Some(summary) = &parsed_response.overall_summary {
                    println!("ℹ️ LLM Overall Summary: {}", summary);
                }

                let mut cargo_deps_to_add: Vec<String> = Vec::new();
                let mut source_code_suggestions_collected: Vec<llm_parsers::LlmSourceCodeSuggestion> = Vec::new();
                let mut general_advice_collected: Vec<String> = Vec::new();

                for suggestion in parsed_response.suggestions {
                    match suggestion {
                        llm_parsers::LlmSingleSuggestion::CargoDependency(deps) => {
                            cargo_deps_to_add.extend(deps.dependency_lines);
                            if let Some(notes) = deps.notes { general_advice_collected.push(format!("Cargo dep notes: {}", notes));}
                        }
                        llm_parsers::LlmSingleSuggestion::SourceCode(code_sugg) => {
                            source_code_suggestions_collected.push(code_sugg);
                        }
                        llm_parsers::LlmSingleSuggestion::GeneralAdvice(advice) => {
                            general_advice_collected.push(advice.content);
                            if let Some(notes) = advice.notes { general_advice_collected.push(format!("Advice notes: {}", notes));}
                        }
                    }
                }

                if !cargo_deps_to_add.is_empty() {
                    println!("\nProposed Cargo.toml dependencies to add:");
                    for dep in &cargo_deps_to_add { println!("  - {}", dep); }
                    let mut confirmed_add_deps = auto_approve_deps;
                    if !auto_approve_deps {
                        let user_choice = read_testable_line("👉 Add these dependencies to Cargo.toml? (yes/no): ")?;
                        if user_choice.trim().to_lowercase() == "yes" || user_choice.trim().to_lowercase() == "y" {
                            confirmed_add_deps = true;
                        }
                    } else { println!("Auto-approving Cargo dependency changes."); }

                    if confirmed_add_deps {
                        let project_root = std::env::current_dir().map_err(anyhow::Error::from)?;
                        match cargo_toml_updater::add_cargo_dependencies(&project_root, &cargo_deps_to_add) {
                            Ok(_) => println!("✅ Cargo.toml updated successfully."),
                            Err(e) => eprintln!("⚠️ Failed to update Cargo.toml: {}", e),
                        }
                    } else { println!("Skipped adding Cargo dependencies."); }
                }

                if !source_code_suggestions_collected.is_empty() {
                    println!("\n--- Processing Source Code Suggestions ---");
                    let mut skip_remaining_code = false;
                    for (idx, code_sugg) in source_code_suggestions_collected.iter().enumerate() {
                        if skip_remaining_code {
                            println!("  Skipping remaining code suggestion for {} due to previous 'skip_all_code'.", code_sugg.target_file);
                            continue;
                        }
                        println!("\nSuggestion {}/{}: Action: {:?} for file: {}", idx + 1, source_code_suggestions_collected.len(), code_sugg.action, code_sugg.target_file);
                        if !code_sugg.content.is_empty() {
                             println!("  Content Snippet (first 200 chars):\n    {}", code_sugg.content.chars().take(200).collect::<String>().replace('\n', "\n    "));
                        } else if let Some(imp) = &code_sugg.import_statement {
                             println!("  Import: {}", imp);
                        }

                        let mut apply_this_change = auto_approve_code;
                        if !auto_approve_code {
                            loop {
                                let user_choice_code = read_testable_line(&format!("👉 Apply change to '{}'? (yes/no/details/skip_all_code) [y/n/d/s]: ", code_sugg.target_file))?;
                                match user_choice_code.trim().to_lowercase().as_str() {
                                    "yes" | "y" => { apply_this_change = true; break; }
                                    "no" | "n" => { apply_this_change = false; break; }
                                    "details" | "d" => {
                                        println!("--- Full Content for {} (Action: {:?}) ---", code_sugg.target_file, code_sugg.action);
                                        if let Some(imp_stmt) = &code_sugg.import_statement { println!("Import to add: {}", imp_stmt); }
                                        if !code_sugg.content.is_empty() { println!("{}", code_sugg.content); }
                                        println!("--- End Full Content ---");
                                    }
                                    "skip_all_code" | "s" => {
                                        println!("  Skipping this and all subsequent code changes for this task.");
                                        skip_remaining_code = true; apply_this_change = false; break;
                                    }
                                    _ => println!("Invalid choice."),
                                }
                            }
                        } else { println!("Auto-approving code change for: {}", code_sugg.target_file); }

                        if apply_this_change {
                            let target_path = std::path::Path::new(&code_sugg.target_file);
                            let op_result = match code_sugg.action {
                                llm_parsers::SuggestionAction::Create => code_gen::create_file(target_path, &code_sugg.content),
                                llm_parsers::SuggestionAction::Replace => code_gen::replace_file_content(target_path, &code_sugg.content),
                                llm_parsers::SuggestionAction::AppendToFile => code_gen::append_to_file(target_path, &code_sugg.content),
                                llm_parsers::SuggestionAction::AddImport => {
                                    if let Some(import_stmt) = &code_sugg.import_statement {
                                        let import_content_to_add = format!("{}\n", import_stmt);
                                        code_gen::append_to_file(target_path, &import_content_to_add)
                                    } else {
                                        eprintln!("⚠️ 'add_import' action missing 'import_statement'. Skipping."); Ok(())
                                    }
                                },
                                _ => {
                                    println!("ℹ️ Action {:?} for {} is advanced. Please apply manually.", code_sugg.action, code_sugg.target_file); Ok(())
                                }
                            };
                            if let Err(e) = op_result { eprintln!("⚠️ Failed to apply code change for {}: {}", code_sugg.target_file, e); }
                        } else if !skip_remaining_code { println!("  Skipped code change for {}.", code_sugg.target_file); }
                    }
                }

                if !general_advice_collected.is_empty() {
                    println!("\n--- General Advice from LLM ---");
                    for advice in general_advice_collected { println!("- {}", advice); }
                }

                if cargo_deps_to_add.is_empty() && source_code_suggestions_collected.is_empty() && general_advice_collected.is_empty() {
                     println!("\nℹ️ LLM response parsed successfully but contained no actionable suggestions.");
                }
            }
            Err(e) => {
                eprintln!("⚠️ Failed to parse LLM response as structured JSON for task {}: {}", task_id, e);
                eprintln!("   Falling back to raw code block extraction for task {}...", task_id);
                code_gen::generate_code_files(&self.project, task_id, llm_response_str)?;
            }
        }
        Ok(())
    }

    pub fn get_project(&self) -> &Project { &self.project }
    #[cfg(test)]
    pub(crate) fn get_project_mut(&mut self) -> &mut Project { &mut self.project }
}
