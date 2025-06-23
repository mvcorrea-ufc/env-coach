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
}