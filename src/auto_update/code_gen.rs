// src/auto_update/code_gen.rs
use std::fs;
use std::path::Path;
use crate::config::{Project, ProjectMeta}; // ProjectMeta for get_primary_language

// --- Pure helper functions ---
fn extract_language_from_line(line: &str) -> Option<String> {
    if line.starts_with("```") {
        let lang = line.trim_start_matches("```").trim().to_lowercase();
        if !lang.is_empty() { Some(lang) } else { None }
    } else { None }
}

fn extract_java_class_name(code: &str) -> Option<String> {
    for line in code.lines() {
        if line.trim().starts_with("public class ") || line.trim().starts_with("class ") {
            if let Some(class_part) = line.split_whitespace().nth(2) {
                return Some(class_part.trim_end_matches('{').to_string());
            }
        }
    }
    None
}

// --- Functions depending on ProjectMeta or Project ---
pub fn get_primary_language(project_meta: &ProjectMeta) -> String {
    for tech in &project_meta.tech_stack {
        match tech.as_str() {
            "rust" => return "rust".to_string(),
            "nodejs" => return "javascript".to_string(),
            "python" => return "python".to_string(),
            "go" => return "go".to_string(),
            "java" => return "java".to_string(),
            _ => continue,
        }
    }
    "rust".to_string() // Default fallback
}

fn guess_filename_for_language(project_meta: &ProjectMeta, code: &str, language: &str, index: usize) -> String {
    match language {
        "rust" => {
            if code.contains("fn main()") { "src/main.rs".to_string() }
            else if code.contains("#[cfg(test)]") || code.contains("mod tests") { "src/tests.rs".to_string() }
            else if code.contains("pub struct") || code.contains("pub enum") { format!("src/lib_{}.rs", index) }
            else if code.contains("impl ") { format!("src/module_{}.rs", index) }
            else { format!("src/generated_{}.rs", index) }
        }
        "javascript" | "js" => {
            if code.contains("module.exports") || code.contains("export") { format!("src/module_{}.js", index) }
            else { format!("src/generated_{}.js", index) }
        }
        "python" | "py" => {
            if code.contains("if __name__ == \"__main__\"") { "main.py".to_string() }
            else if code.contains("class ") { format!("src/class_{}.py", index) }
            else { format!("src/module_{}.py", index) }
        }
        "go" => {
            if code.contains("func main()") { "main.go".to_string() }
            else { format!("src/module_{}.go", index) }
        }
        "java" => {
            if let Some(class_name) = extract_java_class_name(code) { format!("src/{}.java", class_name) }
            else { format!("src/Generated_{}.java", index) }
        }
        _ => {
            let primary_lang = get_primary_language(project_meta);
            let extension = match primary_lang.as_str() {
                "rust" => "rs", "javascript" => "js", "python" => "py",
                "go" => "go", "java" => "java", _ => "txt",
            };
            format!("src/generated_{}.{}", index, extension)
        }
    }
}

fn extract_code_blocks(project_meta: &ProjectMeta, text: &str) -> Vec<(String, String)> {
    let mut code_blocks = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if let Some(lang) = extract_language_from_line(lines[i]) {
            let mut code = String::new();
            i += 1;
            while i < lines.len() && !lines[i].starts_with("```") {
                code.push_str(lines[i]);
                code.push('\n');
                i += 1;
            }
            if !code.trim().is_empty() {
                let filename = guess_filename_for_language(project_meta, &code, &lang, code_blocks.len());
                code_blocks.push((filename, code));
            }
        }
        i += 1;
    }
    code_blocks
}

pub fn generate_code_files(project: &Project, task_id: &str, llm_response: &str) -> anyhow::Result<()> {
    println!("💻 Auto-generating code files for task {}...", task_id);
    let code_blocks = extract_code_blocks(&project.meta, llm_response);
    if code_blocks.is_empty() {
        println!("ℹ️ No code blocks found in LLM response");
        return Ok(());
    }
    for (filename, code) in code_blocks {
        let file_path = Path::new(&filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !file_path.exists() {
            fs::write(file_path, code)?;
            println!("✅ Generated: {}", filename);
        } else {
            println!("⚠️ File {} already exists - skipping generation", filename);
            println!("💡 To regenerate, delete the file and run assist-task again");
        }
    }
    Ok(())
}
