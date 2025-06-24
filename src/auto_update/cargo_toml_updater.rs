// src/auto_update/cargo_toml_updater.rs
use std::fs;
use std::path::Path;
use anyhow::{Context, Result, anyhow};
use toml_edit::{DocumentMut, table, value as toml_value}; // Removed Item, Value

/// Adds specified dependencies to the `Cargo.toml` file in the project root.
/// `dependency_lines` should be an array of strings, where each string is a
/// full dependency line as it would appear in Cargo.toml, e.g.,
/// `clap = { version = "4.0", features = ["derive"] }` or `serde = "1.0"`.
pub fn add_cargo_dependencies(project_root: &Path, dependency_lines: &[String]) -> Result<()> {
    let cargo_toml_path = project_root.join("Cargo.toml");

    if !cargo_toml_path.exists() {
        return Err(anyhow!("Cargo.toml not found at {:?}", cargo_toml_path));
    }

    let content = fs::read_to_string(&cargo_toml_path) // Removed mut
        .with_context(|| format!("Failed to read Cargo.toml from {:?}", cargo_toml_path))?;

    let mut doc = content.parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse Cargo.toml at {:?}", cargo_toml_path))?;

    let deps_table = doc
        .entry("dependencies")
        .or_insert_with(|| table()) // Creates an empty table if [dependencies] doesn't exist
        .as_table_mut()
        .ok_or_else(|| anyhow!("[dependencies] section in Cargo.toml is not a table"))?;

    let mut added_any = false;
    for line in dependency_lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Attempt to parse the line like "key = value"
        let parts: Vec<&str> = line.splitn(2, '=').map(str::trim).collect();
        if parts.len() != 2 {
            eprintln!("⚠️ Skipping malformed dependency line: {}", line);
            continue;
        }
        let key = parts[0];
        let value_str = parts[1];

        if deps_table.contains_key(key) {
            println!("ℹ️ Dependency '{}' already exists in Cargo.toml. Skipping.", key);
            continue;
        }

        // Attempt to parse the value_str as a TOML Value.
        // This is a bit tricky because `toml_edit::Item::from_str` or `toml_edit::Value::from_str`
        // are not straightforward for arbitrary TOML structures like inline tables.
        // A robust way is to create a dummy TOML doc with `temp_key = actual_value_str` and parse that.
        let dummy_doc_str = format!("temp_key = {}", value_str);
        match dummy_doc_str.parse::<DocumentMut>() {
            Ok(dummy_doc) => {
                if let Some(item) = dummy_doc.get("temp_key").cloned() {
                    deps_table.insert(key, item);
                    println!("✅ Added dependency to Cargo.toml: {}", line);
                    added_any = true;
                } else {
                    eprintln!("⚠️ Failed to parse value for dependency line (internal error): {}", line);
                }
            }
            Err(e) => {
                 // If parsing fails, try to add as a simple string if it looks like one
                if value_str.starts_with('"') && value_str.ends_with('"') {
                    let simple_value_str = value_str.trim_matches('"').to_string();
                    deps_table.insert(key, toml_value(simple_value_str));
                    println!("✅ Added dependency to Cargo.toml (as simple string): {}", line);
                    added_any = true;
                } else {
                    eprintln!("⚠️ Failed to parse value for dependency line '{}': {}. It might be a malformed inline table or require specific formatting.", line, e);
                }
            }
        }
    }

    if added_any {
        fs::write(&cargo_toml_path, doc.to_string())
            .with_context(|| format!("Failed to write updated Cargo.toml to {:?}", cargo_toml_path))?;
        println!("Successfully updated Cargo.toml.");
    } else {
        println!("No new dependencies were added to Cargo.toml.");
    }

    Ok(())
}
