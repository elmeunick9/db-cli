use std::fs;
use std::path::Path;
use chrono::Datelike;
use tracing::info;

use crate::utils::fs as fs_utils;
use crate::config::Config;

/// Create a new release
pub async fn execute(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let sql_base = config.sql_base();

    // Check if "next" directory exists
    let next_dir = Path::new(&sql_base).join("next");
    if !next_dir.exists() {
        return Err("next directory does not exist".into());
    }

    // Generate new version number (YYYYMMDD)
    let now = chrono::Utc::now();
    let new_version = format!("{:04}{:02}{:02}", now.year(), now.month(), now.day());

    // Check if version already exists
    let new_version_dir = Path::new(&sql_base).join(&new_version);
    if new_version_dir.exists() {
        return Err(format!("version {} already exists", new_version).into());
    }

    info!("Creating release: {}", new_version);

    // Rename "next" to new version
    fs::rename(&next_dir, &new_version_dir)?;

    // Create new "next" directory as a copy of the released version
    fs_utils::copy_dir_recursive(&new_version_dir, &next_dir)?;

    // Clean migration files in the new "next" directory
    clean_migration_files(&next_dir)?;

    // Handle keep_max if set
    let keep_max = config.keep_max_releases;
    let versions = fs_utils::list_versions(&sql_base)?;
    let mut numeric_versions: Vec<String> = versions
        .into_iter()
        .filter(|v| v != "next" && v.chars().all(|c| c.is_ascii_digit()))
        .collect();
    numeric_versions.sort_by(|a, b| b.cmp(a)); // Sort descending

    if numeric_versions.len() > keep_max && keep_max > 0 {
        for old_version in numeric_versions.iter().skip(keep_max) {
            let old_dir = Path::new(&sql_base).join(old_version);
            if old_dir.exists() {
                info!("Removing old version: {}", old_version);
                fs::remove_dir_all(&old_dir)?;
            }
        }
    }

    info!("Release {} created successfully", new_version);
    Ok(())
}

fn clean_migration_files(dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Delete <YYYYMMDD.sql> files in subdirectories, one level deep.
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                for sub_entry in fs::read_dir(&path)? {
                    let sub_entry = sub_entry?;
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        if let Some(file_name) = sub_path.file_name().and_then(|n| n.to_str()) {
                            if file_name.ends_with(".sql") {
                                let prefix = &file_name[..file_name.len() - 4]; // Remove ".sql"
                                if prefix.len() == 8 && prefix.chars().all(|c| c.is_ascii_digit()) {
                                    fs::remove_file(&sub_path)?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
