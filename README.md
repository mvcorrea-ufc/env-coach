# env-coach 🔧

Your local LLM project assistant for Rust development. Transform natural language requirements into structured development workflows with AI assistance.

## Overview

`env-coach` is a standalone CLI tool that integrates Large Language Models (LLMs) into your development workflow. It helps you:

- 📝 Convert natural language requirements into user stories
- 📋 Manage project backlogs and sprints  
- 🤖 Get LLM assistance during development
- 📊 Track progress and velocity
- 🏗️ Maintain project documentation

## Installation

### From Source
```bash
git clone https://github.com/yourusername/env-coach.git
cd env-coach
cargo install --path .
```

### Development
```bash
cargo build --release
export PATH="$PATH:$(pwd)/target/release"
```

## Quick Start

### 1. Initialize a Project
```bash
# In any Rust project directory
cd my-rust-project
env-coach init
```

### 2. Configure LLM Connection
Edit LLM connection settings. `env-coach` uses a hierarchical configuration:
1.  **Project-specific (`project.json`):** Settings here override global and default values.
2.  **Global (`~/.config/env-coach/config.json` on Linux/macOS, or equivalent user config directory on Windows):** Sets default LLM parameters for all your projects.
3.  **Built-in defaults:** If a setting is not found in project or global configs, a sensible default is used (e.g., `localhost:11434` for Ollama).

**Global Configuration (Optional):**

Create `~/.config/env-coach/config.json` (or your OS's equivalent config path) with your preferred default LLM settings. Example:
```json
{
  "llm": {
    "host": "192.168.1.100",
    "port": 11434,
    "model": "mistral:latest",
    "timeout_ms": 60000
  }
}
```
All fields within `"llm"` are optional. If this file or any field is omitted, built-in defaults will be used.

**Project-Specific Configuration (`project.json`):**

When you run `env-coach init`, a `project.json` is created. You can add an `"llm"` object within the `"meta"` section to override global settings or defaults for this specific project.
Example `project.json` snippet:
```json
{
  "meta": {
    "name": "my-rust-project",
    "description": "...",
    "created": "...",
    "tech_stack": ["rust"],
    "tags": [],
    // Project-specific LLM overrides
    "llm": {
      "model": "deepseek-coder:33b", // Override global/default model for this project
      "timeout_ms": 120000          // Custom timeout for this project
      // Host and port might be inherited from global or default if not specified here
    }
  },
  "backlog": [],
  "sprints": [],
  "current_sprint": null
}
```
If the `"llm"` object or any of its fields are absent in `project.json`, `env-coach` will look at the global configuration, and then fall back to built-in defaults.

Use `env-coach status` to see the resolved LLM configuration and the source of each setting (Default, Global, or Project).

### 3. Add Requirements
```bash
env-coach add-requirement "I want a CLI tool that manages my book collection with CRUD operations and search functionality"
```

### 4. Plan & Execute
```bash
# View generated backlog
env-coach list-backlog

# Plan a sprint
env-coach plan-sprint --goal "MVP book management" --days 7

# Start working
env-coach start-task US-001
env-coach assist-task US-001  # Get LLM coding help
env-coach complete-task US-001

# Track progress
env-coach show-sprint
```

## Commands

### Project Management
- `init` - Initialize LLM workflow in current project
- `status` - Check LLM connectivity
- `add-requirement <text>` - Process natural language requirements
- `list-backlog` - Show current backlog
- `add-story --title <title> --description <desc>` - Manually add user story

### Sprint Management  
- `plan-sprint --goal <goal> --days <days>` - Plan development sprint
- `start-sprint <id>` - Activate a sprint
- `show-sprint` - Show current sprint status
- `list-stories` - List all user stories

### Development Workflow
- `start-task <id>` - Begin working on a task
- `assist-task <id>` - Get LLM assistance with implementation
- `complete-task <id>` - Mark task complete and update metrics

### LLM Interaction
- `llm-cycle --prompt <text>` - Send custom prompt to LLM

## Examples

### Complete Workflow Example
```bash
# 1. Initialize project
mkdir book-manager && cd book-manager
cargo init --name book-manager
env-coach init

# 2. Add requirements
env-coach add-requirement "A REST API for managing a personal book collection with authentication, CRUD operations, search by title/author, and reading progress tracking"

# 3. Add manual stories (until auto-generation is implemented)
env-coach add-story --title "User Authentication" --description "As a user, I want to register and login securely so that I can access my personal book collection"

env-coach add-story --title "Book CRUD Operations" --description "As a user, I want to add, edit, delete, and view books so that I can manage my collection"

# 4. Plan sprint
env-coach plan-sprint --goal "Authentication and basic CRUD" --days 7

# 5. Start development
env-coach start-task US-001
env-coach assist-task US-001

# 6. Complete and continue
env-coach complete-task US-001
env-coach show-sprint
```

### Project Structure After Init
```
my-project/
├── project.json              # Project configuration and backlog
├── .env-coach/               # Tool-specific files
│   ├── prompts/              # Customizable LLM prompts
│   └── templates/            # Project templates
├── docs/
│   └── adr/                  # Architecture Decision Records
└── src/                      # Your application code
```

## Configuration

### LLM Models
Tested with:
- `deepseek-coder:6.7b` (recommended for code tasks)
- `llama2:7b` (general purpose)
- `codellama:7b` (code-focused)

### Customizing Prompts
Edit files in `.env-coach/prompts/` to customize LLM behavior:
- `requirements_analyst.md` - Requirements processing
- `code_reviewer.md` - Code review assistance  
- `task_assistant.md` - Development assistance

## Development Phases

### Phase 1 (Current) ✅
- Basic project management
- Manual story creation
- LLM integration for assistance
- Sprint planning and tracking

### Phase 2 (Planned)
- Automatic JSON parsing from LLM responses
- Auto-population of backlog from requirements
- Enhanced code generation and review

### Phase 3 (Planned)  
- Git integration
- Automated testing assistance
- Advanced metrics and reporting

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Links

- [Documentation](docs/)
- [Examples](examples/)
- [Issues](https://github.com/yourusername/env-coach/issues)
- [Releases](https://github.com/yourusername/env-coach/releases)