// src/auto_update/tests.rs

use crate::config::{GlobalConfig, Project, Priority, Status, ItemType};
use crate::auto_update::updater::{AutoUpdater, UpdateContext};
use crate::auto_update::llm_parsers::LlmUserStory;

fn create_test_project() -> Project {
    let global_config = GlobalConfig::load().unwrap_or_default();
    Project::new("Test Project".to_string(), "A test project".to_string(), global_config.llm.as_ref())
}

// --- Tests for RequirementAnalysis ---
#[test]
fn test_process_llm_requirement_analysis_valid_json() {
    let project = create_test_project();
    let mut updater = AutoUpdater::new(project);
    let llm_response_json = r#"{"user_stories": [{"title": "Login","story": "As a user...","priority": "High","effort": 3,"acceptance_criteria": ["AC1"]},{"title": "Signup","story": "As a new user...","priority": "Critical","effort": 5,"acceptance_criteria": ["AC1"]}]}"#;
    updater.process_llm_response(llm_response_json, UpdateContext::RequirementAnalysis).unwrap();
    let updated_project = updater.get_project();
    assert_eq!(updated_project.backlog.len(), 2);

    let story1 = updated_project.backlog.get(0).unwrap();
    assert_eq!(story1.id, "US-001");
    assert_eq!(story1.title, "Login"); // Title from simplified JSON
    assert_eq!(story1.priority, Priority::High);
    assert_eq!(story1.status, Status::Todo);
    assert_eq!(story1.item_type, ItemType::UserStory);

    let story2 = updated_project.backlog.get(1).unwrap();
    assert_eq!(story2.id, "US-002");
    assert_eq!(story2.title, "Signup"); // Title from simplified JSON
    assert_eq!(story2.priority, Priority::Critical);
}

#[test]
fn test_process_llm_requirement_analysis_empty_stories_array() {
    let project = create_test_project();
    let mut updater = AutoUpdater::new(project);
    let llm_response_json = r#"{ "user_stories": [] }"#;
    updater.process_llm_response(llm_response_json, UpdateContext::RequirementAnalysis).unwrap();
    assert_eq!(updater.get_project().backlog.len(), 0);
}

#[test]
fn test_process_llm_requirement_analysis_malformed_json_fallback() {
    let project1 = create_test_project();
    let mut updater1 = AutoUpdater::new(project1);
    let llm_response_malformed_json = r#"{ "user_stories": [ { "title": "Test" ... ] }"#;
    updater1.process_llm_response(llm_response_malformed_json, UpdateContext::RequirementAnalysis).unwrap();
    assert_eq!(updater1.get_project().backlog.len(), 0);

    let project2 = create_test_project();
    let mut updater2 = AutoUpdater::new(project2);
    let llm_response_text_with_story = "Some text. As a user, I want a feature. More text.";
    updater2.get_project_mut().backlog.clear();
    updater2.process_llm_response(llm_response_text_with_story, UpdateContext::RequirementAnalysis).unwrap();
    assert_eq!(updater2.get_project().backlog.len(), 1);
}

#[test]
fn test_convert_llm_story_priority_mapping() {
    let prio_map = vec![("High", Priority::High), ("Critical", Priority::Critical)];
    for (p_str, p_enum) in prio_map {
        let llm_story = LlmUserStory {title: "T".to_string(), story: "S".to_string(), priority: p_str.to_string(), effort: 1, acceptance_criteria: vec![]};
        let item = crate::auto_update::llm_parsers::convert_llm_story_to_backlog_item(llm_story, "ID".to_string()).unwrap();
        assert_eq!(item.priority, p_enum);
    }
}

// --- Tests for assist-task / UpdateContext::CodeGeneration ---
#[test]
fn test_assist_task_valid_structured_json_cargo_and_code() {
    let project = create_test_project();
    let mut updater = AutoUpdater::new(project);
    let task_id = "US-123".to_string();
    let llm_response_json = r#"{"suggestions": [{"type": "cargo_dependency","dependency_lines": ["toml_edit = \"0.22\""]},{"type": "source_code","target_file": "src/new_util.rs","action": "create","content": "pub fn new_helper() {}"},{"type": "general_advice","content": "Remember to add mod new_util;"}],"overall_summary": "Suggestions."}"#;
    let result = updater.process_llm_response(llm_response_json, UpdateContext::CodeGeneration(task_id.clone()));
    assert!(result.is_ok(), "Processing valid structured JSON failed: {:?}", result.err());
}

#[test]
fn test_assist_task_non_json_response_triggers_fallback() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let mut project = create_test_project();
    project.meta.name = "FallbackTest".to_string();
    let mut updater = AutoUpdater::new(project);
    let task_id = "US-456".to_string();

    let llm_response_plain_text_with_code = "```rust\nfn example_func_fallback() {}\n```";

    // Test extract_code_blocks directly (it's pub(crate))
    let extracted = crate::auto_update::code_gen::extract_code_blocks(&updater.get_project().meta, llm_response_plain_text_with_code);
    assert_eq!(extracted.len(), 1);
    if !extracted.is_empty() {
        assert!(extracted[0].1.contains("fn example_func_fallback()"));
    }

    // This single call uses task_id once.
    let result = updater.process_llm_response(llm_response_plain_text_with_code, UpdateContext::CodeGeneration(task_id));
    assert!(result.is_ok(), "Fallback processing failed: {:?}", result.err());

    let src_path = temp_dir.path().join("src");
    let mut generated_file_found_and_correct = false;
    if src_path.exists() && src_path.is_dir() {
        for entry in std::fs::read_dir(src_path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                if path.file_name().unwrap_or_default().to_string_lossy().contains("generated_") {
                    let content = std::fs::read_to_string(path).unwrap_or_default();
                    if content.contains("fn example_func_fallback()") {
                        generated_file_found_and_correct = true;
                        break;
                    }
                }
            }
        }
    }
    assert!(generated_file_found_and_correct, "Fallback did not create expected .rs file in src/");

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_assist_task_json_with_only_general_advice_no_fallback() {
    let project = create_test_project();
    let mut updater = AutoUpdater::new(project);
    let task_id = "US-789".to_string();
    let llm_response_json_advice_only = r#"{"suggestions": [{"type": "general_advice","content": "Refactor auth."}],"overall_summary": "Advice."}"#;
    let result = updater.process_llm_response(llm_response_json_advice_only, UpdateContext::CodeGeneration(task_id));
    assert!(result.is_ok(), "Processing JSON with only advice failed: {:?}", result.err());
}
