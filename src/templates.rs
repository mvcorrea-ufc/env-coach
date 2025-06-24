// src/templates.rs
pub struct Templates;

impl Templates {
    #[allow(dead_code)]
    pub fn project_json_template(project_name: &str) -> String {
        format!(
            r#"{{
  "meta": {{
    "name": "{}",
    "created": "{}",
    "version": "0.1.0",
    "llm": {{
      "host": "localhost",
      "port": 11434,
      "model": "deepseek-coder:6.7b",
      "timeout_ms": 30000
    }}
  }},
  "prd": null,
  "backlog": [],
  "sprints": [],
  "current_sprint": null,
  "releases": []
}}"#,
            project_name,
            chrono::Utc::now().format("%Y-%m-%d")
        )
    }

    #[allow(dead_code)]
    pub fn requirements_analyst_prompt() -> &'static str {
        r#"You are a skilled requirements analyst for software projects.

TASK: Transform the following natural language description into structured project requirements.

USER INPUT: "{input}"

Please respond with a JSON object containing:
1. "problem": A clear, one-paragraph problem statement
2. "solution": High-level solution approach  
3. "success_metrics": Array of 3-5 measurable success criteria
4. "constraints": Array of technical/business constraints
5. "user_stories": Array of user stories

Each user story should include:
- "title": Short descriptive title
- "story": "As a [user] I want [goal] so that [benefit]"
- "acceptance_criteria": Array of testable conditions
- "priority": "high", "medium", or "low"
- "effort": Estimate 1-8 story points

Keep stories focused and implementable. Aim for 2-5 user stories.
Return only valid JSON."#
    }

    #[allow(dead_code)]
    pub fn code_reviewer_prompt() -> &'static str {
        r#"You are an experienced Rust code reviewer.

REQUIREMENTS: {requirements}

CODE TO REVIEW:
```rust
{code}
```

Please provide a code review focusing on:
1. Correctness and adherence to requirements
2. Rust best practices and idioms
3. Error handling
4. Code organization and readability
5. Performance considerations

Respond with:
- Overall quality score (1-10)
- Specific strengths
- Specific areas for improvement
- Suggested fixes (if any)

Be constructive and specific in your feedback."#
    }

    #[allow(dead_code)]
    pub fn task_assistant_prompt() -> &'static str {
        r#"You are a Rust development assistant.

TASK: {task_description}

ACCEPTANCE CRITERIA:
{acceptance_criteria}

Please help with implementation by providing:
1. Suggested approach and architecture
2. Key Rust crates/dependencies needed
3. Code skeleton with proper error handling
4. Unit test examples
5. Implementation tips specific to Rust

Focus on practical, working code that follows Rust best practices."#
    }

    pub fn gitignore_additions() -> &'static str {
        r#"
# env-coach
.env-coach/cache/
.env-coach/logs/
"#
    }

    pub fn readme_template(project_name: &str) -> String {
        format!(
            r#"# {}

This project uses [env-coach](https://github.com/your-repo/env-coach) for AI-assisted development.

## Quick Start

```bash
# Initialize env-coach in your project (uses current directory name)
env-coach init

# Or specify a custom name
env-coach init "MyProject"

# Check project status and LLM connectivity
env-coach status

# Add requirements in natural language
env-coach add-requirement "I want to build a REST API for user management"

# View and manage backlog
env-coach list-backlog

# Work with user stories
env-coach add-story --title "User Authentication" --description "Login system"
env-coach list-stories

# Plan sprints (coming soon)
env-coach plan-sprint --goal "MVP development"

# Start development workflow
env-coach start-task <task-id>
env-coach assist-task <task-id>  # Get AI help
env-coach complete-task <task-id>

# Send custom prompts to AI
env-coach llm-cycle --prompt "How do I implement JWT authentication in Rust?"
```

## Project Structure

- `project.json` - Project configuration and backlog
- `.env-coach/` - env-coach specific files (cache, logs)
- `README.md` - This file
- `.gitignore` - Git ignore rules (includes env-coach entries)

## Commands Overview

| Command | Description |
|---------|-------------|
| `env-coach init` | Initialize project (uses current directory) |
| `env-coach status` | Show project status and LLM connectivity |
| `env-coach add-requirement "..."` | Convert natural language to user stories |
| `env-coach list-backlog` | View all backlog items |
| `env-coach list-stories` | View user stories only |
| `env-coach start-task <id>` | Start working on a task |
| `env-coach assist-task <id>` | Get AI assistance for a task |
| `env-coach complete-task <id>` | Mark task as completed |
| `env-coach llm-cycle --prompt "..."` | Send custom prompt to AI |

## LLM Configuration

env-coach uses Ollama by default. Make sure you have:

1. [Ollama installed](https://ollama.ai)
2. Ollama running: `ollama serve`
3. Model downloaded: `ollama pull deepseek-coder:6.7b`

The LLM configuration is stored in `project.json` and can be customized.

## Development Workflow

1. **Initialize**: `env-coach init`
2. **Add Requirements**: `env-coach add-requirement "feature description"`
3. **Review Backlog**: `env-coach list-backlog`
4. **Start Task**: `env-coach start-task US-001`
5. **Get Help**: `env-coach assist-task US-001`
6. **Complete**: `env-coach complete-task US-001`
7. **Repeat**: Continue with next tasks

Generated by env-coach v0.1.0
"#,
            project_name
        )
    }

    pub fn default_requirements_analyst_prompt_content() -> String {
        // This is the new default prompt content.
        // Note the change in "priority" to match the enum.
        r#"You are a software engineering expert analyzing requirements for a project.

PROJECT CONTEXT:
- Project Name: {{project_name}}
- Description: {{project_description}}
- Tech Stack: {{tech_stack}}
- Primary Language: {{primary_language}}
- Tags: {{tags}}

REQUIREMENT TO ANALYZE: "{{requirement}}"

Please respond with a JSON object containing a single key "user_stories".
The value of "user_stories" should be an array of JSON objects, where each object represents a user story.
Each user story object must have the following fields:
- "title": A brief, descriptive title for the user story.
- "story": The user story in the format "As a [user type], I want [goal] so that [reason/benefit]".
- "priority": The priority of the user story. Valid values are "Critical", "High", "Medium", "Low".
- "effort": An estimated effort for the user story, as an integer (e.g., 1, 2, 3, 5, 8).
- "acceptance_criteria": An array of strings, where each string is a specific, testable acceptance criterion.

Focus on:
1. Breaking down the requirement into appropriate user stories.
2. Writing clear and concise acceptance criteria.
3. Estimating effort and assigning priority based on typical software development projects.
4. Tailoring acceptance criteria to be actionable and testable, considering the project's tech stack if relevant.

Generate 2-5 user stories that comprehensively cover the requirement.
Return *only* the valid JSON object, starting with `{` and ending with `}`. Do not include any other text or explanations outside the JSON structure.

Example of a single user story object:
{
  "title": "User Login Feature",
  "story": "As a registered user, I want to log in to the application so that I can access my personalized content.",
  "priority": "High",
  "effort": 3,
  "acceptance_criteria": [
    "User can enter credentials (username/password).",
    "System validates credentials against stored user data.",
    "Successful login redirects to the user dashboard.",
    "Failed login shows an appropriate error message."
  ]
}
"#.to_string()
    }

    pub fn create_default_prompt_if_missing(prompts_dir: &std::path::Path, file_name: &str, content: String) -> anyhow::Result<()> {
        let prompt_path = prompts_dir.join(file_name);
        if !prompt_path.exists() {
            std::fs::write(&prompt_path, content)
                .map_err(|e| anyhow::anyhow!("Failed to write default prompt file at {:?}: {}", prompt_path, e))?;
            println!("📄 Created default prompt: {}", prompt_path.display());
        }
        Ok(())
    }

    pub fn default_sprint_planner_prompt_content() -> String {
        r#"You are an expert Agile Sprint Planner. Your task is to help select user stories from the project backlog that best fit the given sprint goal and constraints.

**Sprint Goal:**
{{sprint_goal}}

{{#if sprint_duration_days}}
**Sprint Duration:** {{sprint_duration_days}} days
{{/if}}

{{#if target_capacity_points}}
**Target Capacity (Story Points):** {{target_capacity_points}}
{{/if}}

**Project Backlog (User Stories):**
The following user stories are available in the backlog. Each item is listed with its ID, Title, Priority, and Effort (in story points).

{{#each backlog_items}}
- **ID:** {{this.id}}
  - **Title:** {{this.title}}
  {{#if this.story_summary}}
  - **Summary:** {{this.story_summary}}
  {{/if}}
  - **Priority:** {{this.priority}}
  - **Effort:** {{this.effort}} points
{{/each}}

**Instructions:**

1.  **Analyze the Sprint Goal:** Understand the primary objective for this sprint.
2.  **Review Backlog Items:** Evaluate each user story's relevance to the sprint goal.
3.  **Consider Constraints:**
    *   Prioritize higher priority items (Critical > High > Medium > Low).
    *   If target capacity is provided, try to select stories whose total effort is close to this capacity without significantly exceeding it.
    *   If sprint duration is provided, consider what can realistically be achieved.
4.  **Selection:** Choose a set of user story IDs that form a cohesive and achievable plan for the sprint, directly contributing to the sprint goal.
5.  **Output:** Respond with a JSON object containing a single key: `"suggested_story_ids"`. The value should be an array of strings, where each string is the ID of a suggested user story.
    Optionally, you can include a `"reasoning"` field with a brief explanation for your selection.

**Example Output Format:**
```json
{
  "suggested_story_ids": [
    "US-001",
    "US-003",
    "US-008"
  ],
  "reasoning": "Selected stories directly address the core aspects of the sprint goal 'Implement User Authentication'. US-001 is critical, and US-003 and US-008 are high-priority supporting features with a combined effort that fits the typical capacity."
}
```

Return *only* the valid JSON object. Do not include any other text or explanations outside the JSON structure.
If no stories seem appropriate or fit the capacity, return an empty array for `suggested_story_ids`.
"#.to_string()
    }

    pub fn default_task_assistant_prompt_content() -> String {
        r#"You are an expert pair programmer and software development assistant, specializing in {{primary_language}}.
Your goal is to provide actionable suggestions, including code, dependency updates, and file modifications, to help implement a given task.

**Project Context:**
- Project Name: {{project_name}}
- Description: {{project_description}}
- Tech Stack: {{tech_stack}}
- Primary Language: {{primary_language}}
- Tags: {{tags}}

**Current Task Details:**
- Task ID: {{task_id}}
- Title: {{task_title}}
- Story: {{task_story}}
- Acceptance Criteria:
{{#each task_acceptance_criteria}}
  - {{this}}
{{/each}}

**User's Specific Request/Question (if any):**
{{user_prompt}}

**Instructions:**

Provide your assistance as a single JSON object. The root object should have a key "suggestions" which is an array of suggestion objects. Each suggestion object must have a "type" field and other fields relevant to its type.

**Suggestion Types and Formats:**

1.  **`cargo_dependency`**: For adding dependencies to `Cargo.toml`.
    -   `type`: "cargo_dependency"
    -   `dependency_lines`: Array of strings. Each string is a complete line to be added under `[dependencies]` in `Cargo.toml` (e.g., "serde = { version = \"1.0\", features = [\"derive\"] }").
    -   `notes` (optional): Brief explanation.

2.  **`source_code`**: For providing source code for new or existing files.
    -   `type`: "source_code"
    -   `target_file`: String. The full suggested path for the file from the project root (e.g., "src/main.rs", "src/module/new_feature.rs").
    -   `action`: String. One of:
        -   `create`: Create a new file with the provided content. Should only be used if the file doesn't exist.
        -   `replace`: Replace the entire content of an existing file. Use with caution.
        -   `append_to_file`: Add the content to the end of an existing file.
        -   `replace_function`: Replace an entire existing function. Requires `function_name`.
        -   `append_to_function`: Add content inside an existing function. Requires `function_name`. (Less common, use carefully).
        -   `add_import`: Add an import statement. Requires `import_statement`.
    -   `content` (optional for some actions like `add_import`): String. The actual source code or text to use.
    -   `function_name` (optional): String. The name of the target function for actions like `replace_function` or `append_to_function`.
    -   `import_statement` (optional): String. The full import line (e.g., "use crate::my_module::MyStruct;").
    -   `notes` (optional): Brief explanation about this code or modification.

3.  **`general_advice`**: For textual explanations, architectural suggestions, best practices, or steps the user should take manually.
    -   `type`: "general_advice"
    -   `content`: String. The textual advice.
    -   `notes` (optional): Brief explanation.

**Example JSON Output Structure:**
```json
{
  "suggestions": [
    {
      "type": "cargo_dependency",
      "dependency_lines": [
        "clap = { version = \"4.0\", features = [\"derive\"] }"
      ],
      "notes": "Clap is used for command-line argument parsing."
    },
    {
      "type": "source_code",
      "target_file": "src/main.rs",
      "action": "replace",
      "content": "fn main() {\n    println!(\"Hello, new world!\");\n}",
      "notes": "Updated main function to reflect new requirements."
    },
    {
      "type": "source_code",
      "target_file": "src/utils.rs",
      "action": "create",
      "content": "pub fn helper_function() -> bool {\n    true\n}",
      "notes": "A new utility module."
    },
    {
      "type": "general_advice",
      "content": "Remember to run `cargo fmt` and `cargo clippy` after these changes. Consider adding more unit tests for the new utility functions.",
      "notes": "Good practices to follow."
    }
  ],
  "overall_summary": "Provided dependencies for CLI parsing, updated main.rs, created a new utils.rs, and gave some general advice."
}
```

**Guidelines for your response:**
-   Ensure the output is **only the valid JSON object** as described. Do not include any introductory text, apologies, or sign-offs outside the JSON structure.
-   If suggesting code for `{{primary_language}}`, ensure it is idiomatic and follows best practices for that language.
-   Be specific with file paths and actions. If modifying an existing file, try to be precise (e.g., suggest replacing a specific function if possible, rather than the whole file, unless necessary).
-   If the user's request is unclear or too broad for a direct code solution, provide `general_advice` on how to approach it or break it down.
-   Provide complete and runnable code examples where applicable.
"#.to_string()
    }
}