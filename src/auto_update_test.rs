// src/auto_update_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_update::{AutoUpdater, UpdateContext};
    use crate::config::{Project, ItemType, Priority, Status};
    use std::fs;
    use chrono::Utc;

    #[test]
    fn test_json_user_story_parsing() {
        let project = create_test_project();
        let mut updater = AutoUpdater::new(project);

        let llm_response = r#"
        {
            "user_stories": [
                {
                    "title": "User Interface",
                    "story": "As a user, I want an interactive command line interface so that I can play the game",
                    "priority": "high",
                    "effort": 3,
                    "acceptance_criteria": [
                        "Display game board clearly",
                        "Accept user input for moves",
                        "Show game status"
                    ]
                },
                {
                    "title": "Game Logic",
                    "story": "As a user, I want the game to detect wins and draws so that I know when the game ends",
                    "priority": "medium",
                    "effort": 5,
                    "acceptance_criteria": [
                        "Detect win conditions",
                        "Detect draw conditions",
                        "Display game results"
                    ]
                }
            ]
        }
        "#;

        let result = updater.process_llm_response(llm_response, UpdateContext::RequirementAnalysis);
        assert!(result.is_ok());

        let updated_project = updater.get_project();
        assert_eq!(updated_project.backlog.len(), 2);
        
        let first_story = &updated_project.backlog[0];
        assert_eq!(first_story.title, "User Interface");
        assert_eq!(first_story.id, "US-001");
        assert!(matches!(first_story.priority, Priority::High));
        assert_eq!(first_story.effort, 3);
        assert_eq!(first_story.acceptance_criteria.len(), 3);
    }

    #[test]
    fn test_text_user_story_extraction() {
        let project = create_test_project();
        let mut updater = AutoUpdater::new(project);

        let llm_response = r#"
        Here are the user stories for your requirement:

        User Interface
        As a user, I want an interactive command line interface so that I can play the game.

        Game Logic  
        As a user, I want the game to detect wins and draws so that I know when the game ends.
        "#;

        let result = updater.process_llm_response(llm_response, UpdateContext::RequirementAnalysis);
        assert!(result.is_ok());

        let updated_project = updater.get_project();
        assert_eq!(updated_project.backlog.len(), 2);
        
        let first_story = &updated_project.backlog[0];
        assert_eq!(first_story.title, "User Interface");
        assert!(first_story.story.contains("As a user, I want an interactive command line interface"));
    }

    #[test]
    fn test_code_block_extraction() {
        let project = create_test_project();
        let updater = AutoUpdater::new(project);

        let llm_response = r#"
        Here's the implementation:

        ```rust
        fn main() {
            println!("Hello, world!");
        }
        ```

        And here's a test:

        ```rust
        #[cfg(test)]
        mod tests {
            #[test]
            fn test_example() {
                assert_eq!(2 + 2, 4);
            }
        }
        ```
        "#;

        let code_blocks = updater.extract_code_blocks(llm_response);
        assert_eq!(code_blocks.len(), 2);
        
        let (filename1, code1) = &code_blocks[0];
        assert_eq!(filename1, "src/main.rs");
        assert!(code1.contains("fn main()"));
        
        let (filename2, code2) = &code_blocks[1];
        assert_eq!(filename2, "src/tests.rs");
        assert!(code2.contains("#[cfg(test)]"));
    }

    #[test]
    fn test_documentation_update() {
        let project = create_test_project();
        let updater = AutoUpdater::new(project);

        // Create a sample completed task
        let task = crate::config::BacklogItem {
            id: "US-001".to_string(),
            item_type: ItemType::UserStory,
            title: "User Interface".to_string(),
            story: "As a user, I want a CLI interface".to_string(),
            acceptance_criteria: vec!["Working CLI".to_string()],
            priority: Priority::Medium,
            effort: 3,
            status: Status::Done,
            created: Utc::now(),
            sprint: None,
            dependencies: Vec::new(),
        };

        // This would normally update README.md and CHANGELOG.md
        let result = updater.update_readme(&task);
        // Note: In a real test, we'd set up a temp directory
        // For now, just verify the method doesn't panic
        println!("README update test completed");
    }

    fn create_test_project() -> Project {
        Project::new(
            "Test Project".to_string(),
            "A test project for auto-update".to_string()
        )
    }

    #[test]
    fn test_filename_guessing() {
        let project = create_test_project();
        let updater = AutoUpdater::new(project);

        // Test main.rs detection
        let main_code = "fn main() { println!(); }";
        assert_eq!(updater.guess_filename(main_code, 0), "src/main.rs");

        // Test test file detection
        let test_code = "#[cfg(test)] mod tests { }";
        assert_eq!(updater.guess_filename(test_code, 0), "src/tests.rs");

        // Test struct detection
        let struct_code = "pub struct Game { }";
        assert_eq!(updater.guess_filename(struct_code, 0), "src/lib_0.rs");
    }
}