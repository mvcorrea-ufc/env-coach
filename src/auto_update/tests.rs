// src/auto_update/tests.rs

use crate::config::{GlobalConfig, Project, Priority, Status, ItemType, PartialLlmConfig};
use crate::auto_update::updater::{AutoUpdater, UpdateContext};
use crate::auto_update::llm_parsers::LlmUserStory;
use tempfile::tempdir;
use std::fs;
use std::path::Path;

fn create_test_project_with_temp_dir(temp_dir_path: &Path) -> Project {
    let global_config = GlobalConfig::load().unwrap_or_default();
    let mut project = Project::new(
        "TestProject".to_string(),
        "A test project".to_string(),
        global_config.llm.as_ref()
    );
    // Save project to the temp_dir so cargo_toml_updater can find project root
    // However, cargo_toml_updater uses std::env::current_dir().
    // For these tests, we ensure Cargo.toml is in the CWD (which is the temp_dir).
    fs::write(temp_dir_path.join("Cargo.toml"), "[dependencies]\n").unwrap();
    project
}


// --- Tests for RequirementAnalysis ---
#[test]
fn test_process_llm_requirement_analysis_valid_json() {
    let project = create_test_project_with_temp_dir(&tempdir().unwrap().path()); // Path won't be used here
    let mut updater = AutoUpdater::new(project);
    let llm_response_json = r#"{"user_stories": [{"title": "Login","story": "As a user...","priority": "High","effort": 3,"acceptance_criteria": ["AC1"]},{"title": "Signup","story": "As a new user...","priority": "Critical","effort": 5,"acceptance_criteria": ["AC1"]}]}"#;
    updater.process_llm_response(llm_response_json, UpdateContext::RequirementAnalysis, false, false).unwrap();
    let updated_project = updater.get_project();
    assert_eq!(updated_project.backlog.len(), 2);
    let story1 = updated_project.backlog.get(0).unwrap();
    assert_eq!(story1.id, "US-001");
    assert_eq!(story1.title, "Login");
    assert_eq!(story1.priority, Priority::High);
    assert_eq!(story1.status, Status::Todo);
    assert_eq!(story1.item_type, ItemType::UserStory);
    let story2 = updated_project.backlog.get(1).unwrap();
    assert_eq!(story2.id, "US-002");
    assert_eq!(story2.title, "Signup");
    assert_eq!(story2.priority, Priority::Critical);
}

#[test]
fn test_process_llm_requirement_analysis_empty_stories_array() {
    let project = create_test_project_with_temp_dir(&tempdir().unwrap().path());
    let mut updater = AutoUpdater::new(project);
    let llm_response_json = r#"{ "user_stories": [] }"#;
    updater.process_llm_response(llm_response_json, UpdateContext::RequirementAnalysis, false, false).unwrap();
    assert_eq!(updater.get_project().backlog.len(), 0);
}

#[test]
fn test_process_llm_requirement_analysis_malformed_json_fallback() {
    let project1 = create_test_project_with_temp_dir(&tempdir().unwrap().path());
    let mut updater1 = AutoUpdater::new(project1);
    let llm_response_malformed_json = r#"{ "user_stories": [ { "title": "Test" ... ] }"#;
    updater1.process_llm_response(llm_response_malformed_json, UpdateContext::RequirementAnalysis, false, false).unwrap();
    assert_eq!(updater1.get_project().backlog.len(), 0);

    let temp_dir_fallback = tempdir().unwrap();
    let project2 = create_test_project_with_temp_dir(temp_dir_fallback.path());
    let mut updater2 = AutoUpdater::new(project2);
    let llm_response_text_with_story = "Some text. As a user, I want a feature. More text.";
    updater2.get_project_mut().backlog.clear();
    updater2.process_llm_response(llm_response_text_with_story, UpdateContext::RequirementAnalysis, false, false).unwrap();
    assert_eq!(updater2.get_project().backlog.len(), 1);
}

#[test]
fn test_convert_llm_story_priority_mapping() {
    let prio_map = vec![("High", Priority::High), ("Critical", Priority::Critical), ("Medium", Priority::Medium), ("Low", Priority::Low), ("Unknown", Priority::Medium)];
    for (p_str, p_enum) in prio_map {
        let llm_story = LlmUserStory {title: "T".to_string(), story: "S".to_string(), priority: p_str.to_string(), effort: 1, acceptance_criteria: vec![]};
        let item = crate::auto_update::llm_parsers::convert_llm_story_to_backlog_item(llm_story, "ID".to_string()).unwrap();
        assert_eq!(item.priority, p_enum);
    }
}

// --- Tests for assist-task / UpdateContext::CodeGeneration ---
#[test]
fn test_assist_task_approve_all_changes() {
    let temp_dir = tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let project = create_test_project_with_temp_dir(temp_dir.path());
    let mut updater = AutoUpdater::new(project);
    let task_id = "US-123".to_string();

    let llm_response_json = r#"
    {
        "suggestions": [
            { "type": "cargo_dependency", "dependency_lines": ["test_dep = \"0.1\""], "notes": "A test dependency." },
            { "type": "source_code", "target_file": "src/new_mod.rs", "action": "create", "content": "pub fn hello() {}", "notes": "New module." }
        ]
    }"#;

    // Simulate auto-approval
    let result = updater.process_llm_response_for_test(
        llm_response_json,
        UpdateContext::CodeGeneration(task_id.clone()),
        true, // auto_approve_deps
        true, // auto_approve_code
        None // No test input iterator needed for auto-approve
    );
    assert!(result.is_ok(), "Processing with auto-approve failed: {:?}", result.err());

    // Verify Cargo.toml
    let cargo_content = fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("test_dep = \"0.1\""));

    // Verify new source file
    let new_file_path = temp_dir.path().join("src/new_mod.rs");
    assert!(new_file_path.exists());
    assert_eq!(fs::read_to_string(new_file_path).unwrap(), "pub fn hello() {}");

    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_assist_task_interactive_confirmations() {
    let temp_dir = tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let project = create_test_project_with_temp_dir(temp_dir.path());
    let mut updater = AutoUpdater::new(project);
    let task_id = "US-124".to_string();

    let llm_response_json = r#"
    {
        "suggestions": [
            { "type": "cargo_dependency", "dependency_lines": ["dep_yes = \"1.0\""] },
            { "type": "source_code", "target_file": "src/file_yes.rs", "action": "create", "content": "content_yes" },
            { "type": "cargo_dependency", "dependency_lines": ["dep_no = \"2.0\""] },
            { "type": "source_code", "target_file": "src/file_no.rs", "action": "create", "content": "content_no" },
            { "type": "source_code", "target_file": "src/file_details_then_yes.rs", "action": "create", "content": "content_details_yes" },
            { "type": "source_code", "target_file": "src/file_skip_all.rs", "action": "create", "content": "content_skip_all" },
            { "type": "source_code", "target_file": "src/file_after_skip.rs", "action": "create", "content": "content_after_skip" }
        ]
    }"#;

    let inputs = vec![
        "yes".to_string(), // Confirm first dep
        "yes".to_string(), // Confirm first source file
        "no".to_string(),  // Deny second dep
        "no".to_string(),  // Deny second source file
        "details".to_string(), "yes".to_string(), // Details then yes for third source file
        "s".to_string(),   // Skip all further code changes
    ];
    let mut input_iter = inputs.into_iter();

    let result = updater.process_llm_response_for_test(
        llm_response_json,
        UpdateContext::CodeGeneration(task_id),
        false, // auto_approve_deps
        false, // auto_approve_code
        Some(&mut input_iter)
    );
    assert!(result.is_ok(), "Interactive processing failed: {:?}", result.err());

    let cargo_content = fs::read_to_string(temp_dir.path().join("Cargo.toml")).unwrap();
    assert!(cargo_content.contains("dep_yes = \"1.0\""));
    assert!(!cargo_content.contains("dep_no = \"2.0\""));

    assert!(temp_dir.path().join("src/file_yes.rs").exists());
    assert_eq!(fs::read_to_string(temp_dir.path().join("src/file_yes.rs")).unwrap(), "content_yes");
    assert!(!temp_dir.path().join("src/file_no.rs").exists());
    assert!(temp_dir.path().join("src/file_details_then_yes.rs").exists());
    assert_eq!(fs::read_to_string(temp_dir.path().join("src/file_details_then_yes.rs")).unwrap(), "content_details_yes");
    assert!(!temp_dir.path().join("src/file_skip_all.rs").exists());
    assert!(!temp_dir.path().join("src/file_after_skip.rs").exists());

    std::env::set_current_dir(original_dir).unwrap();
}


#[test]
fn test_assist_task_non_json_response_triggers_fallback() {
    let temp_dir = tempfile::tempdir().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(temp_dir.path()).unwrap();

    let mut project = create_test_project_with_temp_dir(temp_dir.path());
    project.meta.name = "FallbackTest".to_string();
    let mut updater = AutoUpdater::new(project);
    let task_id = "US-456".to_string();
    let llm_response_plain_text_with_code = "```rust\nfn example_func_fallback() {}\n```";

    let extracted = crate::auto_update::code_gen::extract_code_blocks(&updater.get_project().meta, llm_response_plain_text_with_code);
    assert_eq!(extracted.len(), 1);
    if !extracted.is_empty() { assert!(extracted[0].1.contains("fn example_func_fallback()")); }

    let result = updater.process_llm_response(llm_response_plain_text_with_code, UpdateContext::CodeGeneration(task_id), false, false);
    assert!(result.is_ok(), "Fallback processing failed: {:?}", result.err());

    let src_path = temp_dir.path().join("src");
    let mut generated_file_found_and_correct = false;
    if src_path.exists() && src_path.is_dir() {
        for entry in fs::read_dir(src_path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
                if path.file_name().unwrap_or_default().to_string_lossy().contains("generated_") {
                    let content = fs::read_to_string(path).unwrap_or_default();
                    if content.contains("fn example_func_fallback()") { generated_file_found_and_correct = true; break; }
                }
            }
        }
    }
    assert!(generated_file_found_and_correct, "Fallback did not create expected .rs file in src/");
    std::env::set_current_dir(original_dir).unwrap();
}

#[test]
fn test_assist_task_json_with_only_general_advice_no_fallback() {
    let project = create_test_project_with_temp_dir(&tempdir().unwrap().path());
    let mut updater = AutoUpdater::new(project);
    let task_id = "US-789".to_string();
    let llm_response_json_advice_only = r#"{"suggestions": [{"type": "general_advice","content": "Refactor auth."}],"overall_summary": "Advice."}"#;
    let result = updater.process_llm_response(llm_response_json_advice_only, UpdateContext::CodeGeneration(task_id), false, false);
    assert!(result.is_ok(), "Processing JSON with only advice failed: {:?}", result.err());
}
