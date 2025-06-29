// src/auto_update/llm_parsers.rs
use serde::Deserialize;
use chrono::Utc; // May not be needed here if only parsing LLM response
use crate::config::{BacklogItem, ItemType, Priority, Status}; // For convert_llm_story_to_backlog_item

// --- Structs for parsing `add-requirement` LLM response ---
#[derive(Deserialize, Debug)]
pub struct LlmUserStory {
    pub title: String,
    pub story: String,
    pub priority: String,
    pub effort: u32,
    pub acceptance_criteria: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct LlmUserStoryResponse {
    pub user_stories: Vec<LlmUserStory>,
}

/// Converts an LlmUserStory (from LLM JSON) into a BacklogItem.
/// ID is generated by the caller and passed in.
pub fn convert_llm_story_to_backlog_item(llm_story: LlmUserStory, id: String) -> anyhow::Result<BacklogItem> {
    let priority = match llm_story.priority.as_str() {
        "Critical" => Priority::Critical,
        "High" => Priority::High,
        "Medium" => Priority::Medium,
        "Low" => Priority::Low,
        _ => {
            eprintln!("⚠️ Unknown priority value '{}' from LLM for story '{}'. Defaulting to Medium.", llm_story.priority, llm_story.title);
            Priority::Medium
        }
    };

    Ok(BacklogItem {
        id,
        item_type: ItemType::UserStory,
        title: llm_story.title,
        story: llm_story.story,
        acceptance_criteria: llm_story.acceptance_criteria,
        priority,
        effort: llm_story.effort,
        status: Status::Todo,
        created: Utc::now(),
        sprint: None,
        dependencies: Vec::new(),
    })
}

// --- New structs and function for parsing `assist-task` LLM response ---

#[derive(Deserialize, Debug, Clone, PartialEq)] // Added PartialEq for potential test assertions
#[serde(rename_all = "snake_case")]
pub enum SuggestionAction {
    Create,
    Replace,
    AppendToFile,
    ReplaceFunction,
    AppendToFunction,
    AddImport,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LlmSourceCodeSuggestion {
    pub target_file: String,
    pub action: SuggestionAction,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub function_name: Option<String>,
    #[serde(default)]
    pub import_statement: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LlmCargoDependencySuggestion {
    pub dependency_lines: Vec<String>, // Expecting full lines like "serde = \"1.0\""
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LlmGeneralAdviceSuggestion {
    pub content: String,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmSingleSuggestion {
    CargoDependency(LlmCargoDependencySuggestion),
    SourceCode(LlmSourceCodeSuggestion),
    GeneralAdvice(LlmGeneralAdviceSuggestion),
}

#[derive(Deserialize, Debug)]
pub struct LlmAssistTaskResponse {
    // This is the top-level structure expected from the LLM for assist-task
    pub suggestions: Vec<LlmSingleSuggestion>,
    #[serde(default)]
    pub overall_summary: Option<String>,
}

/// Parses the structured JSON response from an LLM for the `assist-task` command.
pub fn parse_assist_task_response(json_string: &str) -> anyhow::Result<LlmAssistTaskResponse> {
    use anyhow::Context;
    serde_json::from_str(json_string)
        .with_context(|| {
            let snippet = json_string.chars().take(500).collect::<String>();
            format!("Failed to deserialize LLM response for assist-task. Ensure LLM provides valid JSON matching the expected structure. Response (first 500 chars): '{}'", snippet)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_parse_valid_assist_task_response() -> Result<()> {
        let json_str = r#"
        {
            "suggestions": [
                {
                    "type": "cargo_dependency",
                    "dependency_lines": ["serde = \"1.0\""],
                    "notes": "For serialization"
                },
                {
                    "type": "source_code",
                    "target_file": "src/main.rs",
                    "action": "create",
                    "content": "fn main() {}",
                    "notes": "Main function"
                },
                {
                    "type": "general_advice",
                    "content": "Remember to test.",
                    "notes": "Testing is important"
                }
            ],
            "overall_summary": "Added serde, created main.rs, advised testing."
        }
        "#;
        let response = parse_assist_task_response(json_str)?;
        assert_eq!(response.suggestions.len(), 3);
        assert_eq!(response.overall_summary.as_deref(), Some("Added serde, created main.rs, advised testing."));

        // Check first suggestion (CargoDependency)
        if let LlmSingleSuggestion::CargoDependency(dep_sugg) = &response.suggestions[0] {
            assert_eq!(dep_sugg.dependency_lines, vec!["serde = \"1.0\""]);
            assert_eq!(dep_sugg.notes.as_deref(), Some("For serialization"));
        } else {
            panic!("Expected CargoDependency suggestion first.");
        }

        // Check second suggestion (SourceCode)
        if let LlmSingleSuggestion::SourceCode(code_sugg) = &response.suggestions[1] {
            assert_eq!(code_sugg.target_file, "src/main.rs");
            assert_eq!(code_sugg.action, SuggestionAction::Create);
            assert_eq!(code_sugg.content, "fn main() {}");
            assert_eq!(code_sugg.notes.as_deref(), Some("Main function"));
        } else {
            panic!("Expected SourceCode suggestion second.");
        }

        // Check third suggestion (GeneralAdvice)
        if let LlmSingleSuggestion::GeneralAdvice(advice_sugg) = &response.suggestions[2] {
            assert_eq!(advice_sugg.content, "Remember to test.");
            assert_eq!(advice_sugg.notes.as_deref(), Some("Testing is important"));
        } else {
            panic!("Expected GeneralAdvice suggestion third.");
        }
        Ok(())
    }

    #[test]
    fn test_parse_assist_task_response_empty_suggestions() -> Result<()> {
        let json_str = r#"
        {
            "suggestions": [],
            "overall_summary": "No specific suggestions."
        }
        "#;
        let response = parse_assist_task_response(json_str)?;
        assert!(response.suggestions.is_empty());
        assert_eq!(response.overall_summary.as_deref(), Some("No specific suggestions."));
        Ok(())
    }

    #[test]
    fn test_parse_assist_task_response_malformed_json() {
        let json_str = r#"
        {
            "suggestions": [
                {"type": "cargo_dependency", "dependency_lines": ["serde = \"1.0\""],
            ], // Malformed: missing closing brace for suggestion, trailing comma
            "overall_summary": "..."
        }
        "#;
        assert!(parse_assist_task_response(json_str).is_err());
    }

    #[test]
    fn test_parse_assist_task_response_missing_suggestions_field() {
        let json_str = r#"
        {
            "overall_summary": "Missing suggestions field."
        }
        "#;
        // This should fail because "suggestions" is not Option and has no default
        assert!(parse_assist_task_response(json_str).is_err());
    }

    #[test]
    fn test_parse_assist_task_response_unknown_suggestion_type() {
        let json_str = r#"
        {
            "suggestions": [
                {
                    "type": "unknown_type",
                    "data": "some data"
                }
            ]
        }
        "#;
        // serde(tag = "type") will fail if "unknown_type" is not a variant of LlmSingleSuggestion
        assert!(parse_assist_task_response(json_str).is_err());
    }

     #[test]
    fn test_parse_source_code_suggestion_all_fields() -> Result<()> {
        let json_str = r#"
        {
            "type": "source_code",
            "target_file": "src/module.rs",
            "action": "replace_function",
            "content": "fn new_func() {}",
            "function_name": "old_func",
            "import_statement": "use std::collections::HashMap;",
            "notes": "Complete replacement"
        }
        "#;
        let suggestion: LlmSingleSuggestion = serde_json::from_str(json_str)?;
        if let LlmSingleSuggestion::SourceCode(sugg) = suggestion {
            assert_eq!(sugg.target_file, "src/module.rs");
            assert_eq!(sugg.action, SuggestionAction::ReplaceFunction);
            assert_eq!(sugg.content, "fn new_func() {}");
            assert_eq!(sugg.function_name.as_deref(), Some("old_func"));
            assert_eq!(sugg.import_statement.as_deref(), Some("use std::collections::HashMap;"));
            assert_eq!(sugg.notes.as_deref(), Some("Complete replacement"));
        } else {
            panic!("Not a SourceCode suggestion");
        }
        Ok(())
    }

    #[test]
    fn test_parse_source_code_suggestion_minimal_fields() -> Result<()> {
        let json_str = r#"
        {
            "type": "source_code",
            "target_file": "src/minimal.rs",
            "action": "create"
        }
        "#;
        // content is defaulted by serde, others are Option
        let suggestion: LlmSingleSuggestion = serde_json::from_str(json_str)?;
        if let LlmSingleSuggestion::SourceCode(sugg) = suggestion {
            assert_eq!(sugg.target_file, "src/minimal.rs");
            assert_eq!(sugg.action, SuggestionAction::Create);
            assert_eq!(sugg.content, ""); // Default for String
            assert!(sugg.function_name.is_none());
            assert!(sugg.import_statement.is_none());
            assert!(sugg.notes.is_none());
        } else {
            panic!("Not a SourceCode suggestion");
        }
        Ok(())
    }
}
