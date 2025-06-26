// src/auto_update/code_gen.rs
use std::fs;
use std::path::Path;
use anyhow::{Context as AnyhowContext, Result}; // Import Context for .context() method
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

pub(crate) fn extract_code_blocks(project_meta: &ProjectMeta, text: &str) -> Vec<(String, String)> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    // use std::path::Path; // Path is used via temp_dir.path() or as &Path in function args

    #[test]
    fn test_create_file_success() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("new_file.txt");
        let content = "Hello, world!";

        create_file(&file_path, content)?;

        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path)?, content);
        Ok(())
    }

    #[test]
    fn test_create_file_creates_parent_dirs() -> Result<()> {
        let dir = tempdir()?;
        let nested_path = dir.path().join("parent").join("child").join("new_file.txt");
        let content = "Nested content";

        create_file(&nested_path, content)?;

        assert!(nested_path.exists());
        assert_eq!(fs::read_to_string(&nested_path)?, content);
        Ok(())
    }

    #[test]
    fn test_create_file_already_exists_fails() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("existing_file.txt");
        fs::write(&file_path, "initial content")?; // Create the file first

        let result = create_file(&file_path, "new content");
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&file_path)?, "initial content"); // Ensure not overwritten
        Ok(())
    }

    #[test]
    fn test_replace_file_content_success() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("to_replace.txt");
        fs::write(&file_path, "old content")?;
        let new_content = "completely new content";

        replace_file_content(&file_path, new_content)?;
        assert_eq!(fs::read_to_string(&file_path)?, new_content);
        Ok(())
    }

    #[test]
    fn test_replace_file_content_not_exists_fails() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("non_existent.txt");
        let result = replace_file_content(&file_path, "content");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_append_to_file_success_existing() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("append_to_me.txt");
        fs::write(&file_path, "Line 1\n")?;
        let content_to_append = "Line 2";

        append_to_file(&file_path, content_to_append)?;
        let expected_content = "Line 1\nLine 2\n"; // append_to_file adds a newline
        assert_eq!(fs::read_to_string(&file_path)?, expected_content);
        Ok(())
    }

    #[test]
    fn test_append_to_file_creates_new() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("newly_created_for_append.txt");
        let content_to_append = "First line";

        append_to_file(&file_path, content_to_append)?;
        let expected_content = "First line\n";
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path)?, expected_content);
        Ok(())
    }
     #[test]
    fn test_append_to_file_creates_parent_dirs() -> Result<()> {
        let dir = tempdir()?;
        let nested_path = dir.path().join("parent").join("child").join("append_file.txt");
        let content = "Nested append";

        append_to_file(&nested_path, content)?;

        assert!(nested_path.exists());
        let expected_content = "Nested append\n";
        assert_eq!(fs::read_to_string(&nested_path)?, expected_content);
        Ok(())
    }
}

// --- Basic Source Code File Operations ---

/// Creates a new file with the given content.
/// Parent directories will be created if they don't exist.
/// Fails if the file already exists.
pub fn create_file(file_path: &Path, content: &str) -> anyhow::Result<()> {
    if file_path.exists() {
        return Err(anyhow::anyhow!("File already exists at path: {:?}", file_path));
    }
    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)
            .with_context(|| format!("Failed to create parent directories for {:?}", file_path))?;
    }
    fs::write(file_path, content)
        .with_context(|| format!("Failed to write new file to {:?}", file_path))?;
    println!("✅ Created file: {:?}", file_path);
    Ok(())
}

/// Replaces the entire content of an existing file.
/// Fails if the file does not exist.
pub fn replace_file_content(file_path: &Path, new_content: &str) -> anyhow::Result<()> {
    if !file_path.exists() {
        return Err(anyhow::anyhow!("File not found for replacement at path: {:?}", file_path));
    }
    // Ensure parent directory exists (though it should if file exists, good practice)
    if let Some(parent_dir) = file_path.parent() {
         if !parent_dir.exists() { // Should not happen if file_path.exists() is true unless it's root
            fs::create_dir_all(parent_dir)
                .with_context(|| format!("Failed to create parent directories for {:?}", file_path))?;
        }
    }
    fs::write(file_path, new_content)
        .with_context(|| format!("Failed to write (replace) file content to {:?}", file_path))?;
    println!("🔄 Replaced content of file: {:?}", file_path);
    Ok(())
}

/// Appends content to an existing file. Creates the file if it does not exist.
/// Parent directories will be created if they don't exist.
pub fn append_to_file(file_path: &Path, content_to_append: &str) -> anyhow::Result<()> {
    if let Some(parent_dir) = file_path.parent() {
        fs::create_dir_all(parent_dir)
            .with_context(|| format!("Failed to create parent directories for {:?}", file_path))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true) // Create if it doesn't exist
        .append(true) // Append if it does
        .open(file_path)
        .with_context(|| format!("Failed to open or create file for appending at {:?}", file_path))?;

    use std::io::Write;
    file.write_all(content_to_append.as_bytes())
        .with_context(|| format!("Failed to append content to file {:?}", file_path))?;
    file.write_all(b"\n") // Ensure a newline after appended content, if desired
        .with_context(|| format!("Failed to append newline to file {:?}", file_path))?;

    println!("📝 Appended content to file: {:?}", file_path);
    Ok(())
}
