// src/auto_update/tests.rs

use crate::config::{GlobalConfig, Project, Priority, Status, ItemType};
use crate::auto_update::updater::{AutoUpdater, UpdateContext}; // Assuming AutoUpdater and UpdateContext are in updater.rs and re-exported by mod.rs or directly used
use crate::auto_update::llm_parsers::LlmUserStory; // Assuming LlmUserStory is in llm_parsers.rs

fn create_test_project() -> Project {
    let global_config = GlobalConfig::load().unwrap_or_default();
    Project::new("Test Project".to_string(), "A test project".to_string(), global_config.llm.as_ref())
}

#[test]
fn test_process_llm_requirement_analysis_valid_json() {
    let project = create_test_project();
    let mut updater = AutoUpdater::new(project);

    let llm_response_json = r#"
    {
        "user_stories": [
            {
                "title": "Login Feature",
                "story": "As a user, I want to log in.",
                "priority": "High",
                "effort": 3,
                "acceptance_criteria": ["AC1 for Login", "AC2 for Login"]
            },
            {
                "title": "Signup Feature",
                "story": "As a new user, I want to sign up.",
                "priority": "Critical",
                "effort": 5,
                "acceptance_criteria": ["AC1 for Signup"]
            }
        ]
    }
    "#;

    updater.process_llm_response(llm_response_json, UpdateContext::RequirementAnalysis).unwrap();

    let updated_project = updater.get_project();
    assert_eq!(updated_project.backlog.len(), 2);

    let story1 = updated_project.backlog.get(0).unwrap();
    assert_eq!(story1.id, "US-001");
    assert_eq!(story1.title, "Login Feature");
    assert_eq!(story1.story, "As a user, I want to log in.");
    assert_eq!(story1.priority, Priority::High); // These will require PartialEq on enums
    assert_eq!(story1.effort, 3);
    assert_eq!(story1.acceptance_criteria, &["AC1 for Login", "AC2 for Login"]);
    assert_eq!(story1.status, Status::Todo);
    assert_eq!(story1.item_type, ItemType::UserStory);

    let story2 = updated_project.backlog.get(1).unwrap();
    assert_eq!(story2.id, "US-002");
    assert_eq!(story2.title, "Signup Feature");
    assert_eq!(story2.priority, Priority::Critical);
}

#[test]
fn test_process_llm_requirement_analysis_empty_stories_array() {
    let project = create_test_project();
    let mut updater = AutoUpdater::new(project);
    let llm_response_json = r#"{ "user_stories": [] }"#;
    updater.process_llm_response(llm_response_json, UpdateContext::RequirementAnalysis).unwrap();
    let updated_project = updater.get_project();
    assert_eq!(updated_project.backlog.len(), 0);
}

#[test]
fn test_process_llm_requirement_analysis_malformed_json_fallback() {
    let project = create_test_project();
    let mut updater = AutoUpdater::new(project);
    let llm_response_malformed_json = r#"{ "user_stories": [ { "title": "Test" ... ] }"#;
    let llm_response_text_with_story = "Some text. As a user, I want a feature. More text.";

    updater.process_llm_response(llm_response_malformed_json, UpdateContext::RequirementAnalysis).unwrap();
    assert_eq!(updater.get_project().backlog.len(), 0, "Malformed JSON should not add stories via JSON path");

    // Reset project backlog for text extraction test
    let project_for_text_fallback = create_test_project();
    let mut updater_for_text_fallback = AutoUpdater::new(project_for_text_fallback);
    // get_project_mut() is on AutoUpdater, so we use it on updater_for_text_fallback
    updater_for_text_fallback.get_project_mut().backlog.clear();

    updater_for_text_fallback.process_llm_response(llm_response_text_with_story, UpdateContext::RequirementAnalysis).unwrap();
    assert_eq!(updater_for_text_fallback.get_project().backlog.len(), 1, "Text fallback should add a story");
    let story = updater_for_text_fallback.get_project().backlog.get(0).unwrap();
    assert!(story.story.contains("As a user, I want a feature."));
}

#[test]
fn test_convert_llm_story_priority_mapping() {
    // let _project = create_test_project(); // Not strictly needed if convert_... is pure
    // let updater = AutoUpdater::new(project); // Not needed if convert_... is a free function

    let prio_map = vec![
        ("High", Priority::High), ("Medium", Priority::Medium),
        ("Low", Priority::Low), ("Critical", Priority::Critical),
        ("UnknownValue", Priority::Medium) // Fallback case
    ];

    for (p_str, p_enum) in prio_map {
        let llm_story = LlmUserStory {
            title: "T".to_string(),
            story: "S".to_string(),
            priority: p_str.to_string(),
            effort: 1,
            acceptance_criteria: vec![]
        };
        // Call the free function directly from the llm_parsers module
        let item = crate::auto_update::llm_parsers::convert_llm_story_to_backlog_item(llm_story, "ID".to_string()).unwrap();
        assert_eq!(item.priority, p_enum); // Requires PartialEq for Priority
    }
}
