use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub schema: String,
    pub file: String,
    pub name: Option<String>,
    pub requires: Vec<String>,
    pub sql: String,
}



/// Read all SQL files for a given schema (recursively optional). Returns map of filepath -> content
pub fn read_schema_files(sql_base: &str, version: &str, schema: &str) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let base = Path::new(sql_base).join(version).join(schema);
    let mut files = vec![];
    if !base.exists() {
        return Ok(files);
    }
    for entry in fs::read_dir(base)? {
        let e = entry?;
        let p = e.path();
        if p.is_file() {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if ext == "sql" {
                    files.push(p);
                }
            }
        }
    }
    Ok(files)
}

/// Parse blocks and directives from a SQL file content
pub fn parse_blocks(file_path: &Path) -> Result<Vec<Block>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(file_path)?;
    let mut file_requires: Vec<String> = vec![];
    let mut blocks: Vec<Block> = vec![];

    let mut in_block: Option<String> = None;
    let mut current_sql = String::new();
    let mut current_requires: Vec<String> = vec![];
    let mut file_sql = String::new();

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("-- @requires") {
            // parse requires
            if let Some(req) = trimmed.split_whitespace().nth(2) {
                let dep = req.trim().to_string();
                if in_block.is_some() {
                    current_requires.push(dep);
                } else {
                    file_requires.push(dep);
                }
            }
            continue;
        }
        if trimmed.starts_with("-- @block") {
            // start block
            if let Some(name) = trimmed.split_whitespace().nth(2) {
                in_block = Some(name.to_string());
                current_sql.clear();
                current_requires.clear();
            }
            continue;
        }
        if trimmed.starts_with("-- @endblock") {
            // end the block
            let mut requires = file_requires.clone();
            requires.extend(current_requires.clone());
            let blk = Block {
                schema: file_path.parent().unwrap().file_name().unwrap().to_str().unwrap().to_string(),
                file: file_path.to_string_lossy().to_string(),
                name: in_block.clone(),
                requires,
                sql: current_sql.clone(),
            };
            blocks.push(blk);
            in_block = None;
            current_sql.clear();
            current_requires.clear();
            continue;
        }

        // Append SQL to current context
        if in_block.is_some() {
            current_sql.push_str(line);
            current_sql.push('\n');
        } else {
            file_sql.push_str(line);
            file_sql.push('\n');
        }
    }

    // file-level SQL goes into a block with name=None if non-empty
    if !file_sql.trim().is_empty() {
        blocks.insert(0, Block {
            schema: file_path.parent().unwrap().file_name().unwrap().to_str().unwrap().to_string(),
            file: file_path.to_string_lossy().to_string(),
            name: None,
            requires: file_requires.clone(),
            sql: file_sql,
        });
    }

    Ok(blocks)
}
