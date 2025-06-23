# env-coach Examples

## Basic Workflow

### 1. Initialize Project
```bash
mkdir my-rust-app && cd my-rust-app
cargo init
env-coach init
```

### 2. Add Requirements
```bash
env-coach add-requirement "I want a CLI tool for managing tasks with categories and due dates"
```

### 3. Plan Development
```bash
env-coach list-backlog
env-coach plan-sprint --goal "Basic task management" --days 7
env-coach start-sprint S-001
```

### 4. Development Loop
```bash
env-coach start-task US-001
env-coach assist-task US-001  # Get LLM help
env-coach complete-task US-001
```

## Project Types

### Rust CLI Application
```bash
env-coach add-requirement "Command-line tool for processing CSV files with filtering and aggregation"
```

### Web API
```bash
env-coach add-requirement "REST API for managing inventory with authentication and real-time updates"
```

### Embedded System
```bash
env-coach add-requirement "Microcontroller firmware for sensor data collection with LoRa communication"
```
