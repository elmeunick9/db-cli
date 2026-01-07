use std::fs;
use std::path::{Path, PathBuf};

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