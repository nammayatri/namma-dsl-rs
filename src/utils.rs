use anyhow::{Context, Result};
use heck::{ToSnakeCase, ToUpperCamelCase};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn to_snake(s: &str) -> String {
    s.to_snake_case()
}

#[allow(dead_code)]
pub fn to_pascal(s: &str) -> String {
    s.to_upper_camel_case()
}

pub fn write_to_file(dir: &Path, filename: &str, contents: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("creating dir {}", dir.display()))?;
    let path = dir.join(filename);
    fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

#[allow(dead_code)]
pub fn write_if_not_exists(dir: &Path, filename: &str, contents: &str) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("creating dir {}", dir.display()))?;
    let path = dir.join(filename);
    if !path.exists() {
        fs::write(&path, contents).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub enum FileState {
    New,
    Changed,
    Unchanged,
    NotExist,
}

pub fn get_file_state(path: &Path) -> FileState {
    if !path.exists() {
        return FileState::NotExist;
    }

    let head_hash = get_hash_at_head(path);
    let current_hash = get_hash_object(path);

    match (head_hash, current_hash) {
        (None, _) => FileState::New,
        (Some(h), Some(c)) if h != c => FileState::Changed,
        (Some(_), Some(_)) => FileState::Unchanged,
        _ => FileState::New,
    }
}

fn get_hash_at_head(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "HEAD", &path.to_string_lossy()])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.is_empty() {
        return None;
    }
    stdout.split_whitespace().nth(2).map(String::from)
}

fn get_hash_object(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["hash-object", &path.to_string_lossy()])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

pub fn find_git_root() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse")?;
    if !output.status.success() {
        anyhow::bail!("not inside a git repository");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Checks whether a type string is wrapped in `Option<...>`.
pub fn is_option_type(ty: &str) -> bool {
    ty.starts_with("Option<") && ty.ends_with('>')
}

/// Extracts the inner type from `Option<X>` -> `X`.
pub fn unwrap_option(ty: &str) -> &str {
    if is_option_type(ty) {
        &ty[7..ty.len() - 1]
    } else {
        ty
    }
}

/// Checks whether a type string is `Id<...>`.
pub fn is_id_type(ty: &str) -> bool {
    let inner = unwrap_option(ty);
    inner.starts_with("Id<") && inner.ends_with('>')
}

/// Checks whether a type string is `Vec<...>`.
pub fn is_vec_type(ty: &str) -> bool {
    let inner = unwrap_option(ty);
    inner.starts_with("Vec<") && inner.ends_with('>')
}

/// Extracts `X` from `Id<X>` (works on already-unwrapped-from-Option types too).
pub fn extract_id_inner(ty: &str) -> &str {
    let inner = unwrap_option(ty);
    if inner.starts_with("Id<") && inner.ends_with('>') {
        &inner[3..inner.len() - 1]
    } else {
        inner
    }
}

/// Maps a domain Rust type to the "flattened" DB model type used in diesel structs.
/// `Id<X>` -> `String`, enums -> `String`, `DateTime<Utc>` -> `chrono::NaiveDateTime`.
pub fn domain_type_to_db_type(rust_type: &str, enum_names: &[String]) -> String {
    if is_option_type(rust_type) {
        let inner = unwrap_option(rust_type);
        let mapped = domain_type_to_db_type(inner, enum_names);
        format!("Option<{}>", mapped)
    } else if is_id_type(rust_type) {
        "String".to_string()
    } else if rust_type == "chrono::DateTime<chrono::Utc>" {
        "chrono::NaiveDateTime".to_string()
    } else if rust_type == "chrono::NaiveDate" {
        "chrono::NaiveDate".to_string()
    } else if rust_type == "chrono::NaiveTime" {
        "chrono::NaiveTime".to_string()
    } else if enum_names.contains(&rust_type.to_string()) {
        "String".to_string()
    } else if is_vec_type(rust_type) {
        "serde_json::Value".to_string()
    } else {
        rust_type.to_string()
    }
}

/// Maps a domain Rust type to the diesel column type used in `table!` macros.
pub fn rust_type_to_diesel_type(
    rust_type: &str,
    enum_names: &[String],
    type_mappings: &[(String, String)],
) -> String {
    if is_option_type(rust_type) {
        let inner = unwrap_option(rust_type);
        let mapped = rust_type_to_diesel_type(inner, enum_names, type_mappings);
        format!("Nullable<{}>", mapped)
    } else if is_id_type(rust_type) {
        "Varchar".to_string()
    } else if enum_names.contains(&rust_type.to_string()) {
        "Text".to_string()
    } else if is_vec_type(rust_type) {
        "Jsonb".to_string()
    } else {
        for (rust, diesel) in type_mappings {
            if rust == rust_type {
                return diesel.clone();
            }
        }
        "Text".to_string()
    }
}

/// Maps a domain Rust type to a SQL column type.
pub fn rust_type_to_sql_type(
    rust_type: &str,
    enum_names: &[String],
    type_mappings: &[(String, String)],
) -> String {
    let inner = unwrap_option(rust_type);
    if is_id_type(inner) {
        return "character varying(36)".to_string();
    }
    if enum_names.contains(&inner.to_string()) {
        return "text".to_string();
    }
    if is_vec_type(inner) {
        return "jsonb".to_string();
    }
    for (rust, sql) in type_mappings {
        if rust == inner {
            return sql.clone();
        }
    }
    "text".to_string()
}
