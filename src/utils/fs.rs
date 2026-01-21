use std::fs;
use std::path::{Path, PathBuf};
use similar::{TextDiff};
use walkdir::WalkDir;

/// List version directories under the sql base directory
pub fn list_versions(sql_base: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut res = vec![];
    let base = Path::new(sql_base);
    if !base.exists() {
        return Ok(res);
    }
    for entry in fs::read_dir(base)? {
        let e = entry?;
        let p = e.path();
        if p.is_dir() {
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                res.push(name.to_string());
            }
        }
    }
    Ok(res)
}

/// Return the latest numeric version (YYYYMMDD) found under `sql_base`.
/// Ignores the special `next` folder.
pub fn get_latest_version(sql_base: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut versions = list_versions(sql_base)?;
    // Keep only numeric-looking versions (YYYYMMDD) and exclude "next"
    versions.retain(|v| v != "next" && v.chars().all(|c| c.is_ascii_digit()));
    // Sort descending to get the latest first
    versions.sort_by(|a, b| b.cmp(a));
    if let Some(latest) = versions.into_iter().next() {
        Ok(latest)
    } else {
        Err("no numeric versions found".into())
    }
}

/// List schema directories under sql/<version>
pub fn list_schemas(sql_base: &str, version: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let path = Path::new(sql_base).join(version);
    let mut res = vec![];
    if !path.exists() {
        return Ok(res);
    }
    for entry in fs::read_dir(path)? {
        let e = entry?;
        let p = e.path();
        if p.is_dir() {
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                res.push(name.to_string());
            }
        }
    }
    Ok(res)
}

/// Represents a file with its path and content
#[derive(Debug, Clone)]
pub struct FileContent {
    pub path: PathBuf,
    pub content: String,
}

/// Read all SQL files for a given schema (recursively optional). Returns Vec<FileContent>
pub fn read_schema_files(sql_base: &str, version: &str, schema: &str) -> Result<Vec<FileContent>, Box<dyn std::error::Error>> {
    let base = Path::new(sql_base).join(version);
    let mut files = vec![];
    if !base.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(base.join(schema))? {
        let e = entry?;
        let p = e.path();
        if p.is_file() {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if ext == "sql" {
                    let content = fs::read_to_string(&p)?;
                    // Compute full path from sql_base/version root, POSIX style
                    let relative_path = p.strip_prefix(&base)?;
                    let posix_path = format!("/{}", relative_path.to_string_lossy().replace('\\', "/"));
                    files.push(FileContent {
                        path: PathBuf::from(posix_path),
                        content,
                    });
                }
            }
        }
    }
    Ok(files)
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !src.is_dir() {
        return Err("source is not a directory".into());
    }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Diffs two directories recursively.
/// - `source`: The base directory (left side)
/// - `target`: The new directory (right side)
/// - `filter`: A closure that returns true if the file should be processed
pub fn diff_dir_recursive<F>(source: &Path, target: &Path, filter: F) -> Result<String, Box<dyn std::error::Error>>
where
    F: Fn(&Path) -> bool,
{
    use std::collections::HashSet;
    let mut output = String::new();
    let mut seen_paths = HashSet::new();

    // 1. Process Source (Handles Modified and Deleted files)
    for entry in WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
        let path_a = entry.path();
        if path_a.is_file() && filter(path_a) {
            let rel_path = path_a.strip_prefix(source)?;
            seen_paths.insert(rel_path.to_path_buf());
            
            let path_b = target.join(rel_path);
            let content_a = fs::read_to_string(path_a)?;
            
            if path_b.exists() {
                let content_b = fs::read_to_string(&path_b)?;
                if content_a != content_b {
                    output.push_str(&generate_diff(path_a, &path_b, &content_a, &content_b));
                }
            } else {
                // File deleted in target (exists only in source)
                output.push_str(&generate_diff(path_a, &path_b, &content_a, ""));
            }
        }
    }

    // 2. Process Target (Handles New files)
    for entry in WalkDir::new(target).into_iter().filter_map(|e| e.ok()) {
        let path_b = entry.path();
        if path_b.is_file() {
            let rel_path = path_b.strip_prefix(target)?;
            if !seen_paths.contains(rel_path) && filter(path_b) {
                let path_a = source.join(rel_path);
                let content_b = fs::read_to_string(path_b)?;
                
                // File added in target (exists only in target)
                output.push_str(&generate_diff(&path_a, path_b, "", &content_b));
            }
        }
    }

    Ok(output)
}

fn generate_diff(path_a: &Path, path_b: &Path, old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(100)
        .header(&path_a.to_string_lossy(), &path_b.to_string_lossy())
        .to_string();
    
    if diff.is_empty() { return String::new(); }

    let status = match (old.is_empty(), new.is_empty()) {
        (true, false) => "NEW FILE",
        (false, true) => "DELETED FILE",
        _             => "MODIFIED",
    };

    format!(
        "############################################################\n\
         # STATUS: {}\n\
         # FILE:   {}\n\
         ############################################################\n\
         {}\n",
        status, path_b.display(), diff
    )
}