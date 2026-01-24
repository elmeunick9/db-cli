use std::path::Path;
use std::collections::VecDeque;
use tracing::{info, debug};

use crate::config::Config;
use crate::utils::fs;
use crate::utils::db::{self, DbPool};

/// Get the current database version from the meta table
pub async fn get_current_version(pool: &DbPool, config: &Config) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if config.dry_run {
        return Ok(None);
    }

    match pool {
        DbPool::Postgres(p) => {
            let sql = "SELECT \"value\" FROM {{ref.meta}} WHERE \"key\" = 'db_version'";
            let result: Option<String> = sqlx::query_scalar(
                &db::inject_variables(sql, config)
            )
                .fetch_optional(p)
                .await?;
            Ok(result)
        }
        DbPool::DryRun => Ok(None),
    }
}

/// Find migration files needed to reach target version using BFS to find shortest path
fn find_migrations_to_target(sql_base: &str, current_version: Option<String>, target_version: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let versions = fs::list_versions(sql_base)?;
    let mut numeric_versions: Vec<String> = versions
        .iter()
        .filter(|v| v != &"next" && v.chars().all(|c| c.is_ascii_digit()))
        .cloned()
        .collect();
    numeric_versions.sort();

    let actual_target = if target_version == "next" {
        target_version.to_string()
    } else if target_version == "latest" {
        numeric_versions.last().cloned().ok_or("No numeric versions found")?
    } else {
        target_version.to_string()
    };

    let start_version = current_version.unwrap_or_else(|| {
        numeric_versions.first().cloned().unwrap_or_else(|| "".to_string())
    });

    debug!("Finding migrations from '{}' to '{}'", start_version, actual_target);

    // Build a graph of available migrations
    let mut migration_graph: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut migration_files: std::collections::HashMap<(String, String), String> = std::collections::HashMap::new();
    
    // Initialize all versions in the graph
    for version in &numeric_versions {
        migration_graph.entry(version.clone()).or_insert_with(Vec::new);
    }
    migration_graph.entry("next".to_string()).or_insert_with(Vec::new);
    
    // For each target version (both numeric and "next"), find migration files
    // Structure: sql/<target>/<source>.sql means migration from source to target
    let mut target_versions: Vec<&str> = numeric_versions.iter().map(|v| v.as_str()).collect();
    target_versions.push("next");
    
    for target_version in target_versions {
        let version_dir = Path::new(sql_base).join(target_version);
        if !version_dir.exists() {
            continue;
        }
        
        if let Ok(entries) = std::fs::read_dir(&version_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.ends_with(".sql") && file_name.len() > 4 {
                            let base_name = &file_name[..file_name.len() - 4];
                            if base_name == "next" || base_name.chars().all(|c| c.is_ascii_digit()) {
                                // sql/<target>/<source>.sql means: source -> target
                                // So add target to source's neighbors
                                let edges = migration_graph.entry(base_name.to_string()).or_insert_with(Vec::new);
                                if !edges.contains(&target_version.to_string()) {
                                    edges.push(target_version.to_string());
                                }
                                // Store the migration file path for this edge
                                migration_files.insert(
                                    (base_name.to_string(), target_version.to_string()),
                                    format!("{}/{}.sql", target_version, base_name)
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // BFS to find shortest path
    let mut queue = VecDeque::new();
    let mut visited = std::collections::HashSet::new();
    let mut parent_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    queue.push_back(start_version.clone());
    visited.insert(start_version.clone());

    while let Some(current) = queue.pop_front() {
        if current == actual_target {
            break;
        }

        if let Some(neighbors) = migration_graph.get(&current) {
            for next in neighbors {
                if !visited.contains(next) {
                    visited.insert(next.clone());
                    parent_map.insert(next.clone(), current.clone());
                    queue.push_back(next.clone());
                }
            }
        }
    }

    // Reconstruct path
    let mut migrations = Vec::new();
    let mut current = actual_target.clone();
    
    while let Some(prev) = parent_map.get(&current) {
        if let Some(file_path) = migration_files.get(&(prev.clone(), current.clone())) {
            migrations.push(file_path.clone());
        }
        current = prev.clone();
    }
    migrations.reverse();

    if migrations.is_empty() && start_version != actual_target {
        return Err(format!("No migration path found from '{}' to '{}'", start_version, actual_target).into());
    }

    Ok(migrations)
}

/// Apply migrations to reach the target version
pub async fn execute(config: &Config, target_version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    if !config.is_dev() && target_version.as_deref() == Some("next") {
        return Err("Cannot migrate to 'next' in production mode. Please create a release first.".into());
    }

    let target = target_version.unwrap_or_else(|| "next".to_string());
    
    let mut config_no_dry: Config = config.clone();
    config_no_dry.dry_run = false;

    let pool = db::get_db_pool(&config_no_dry).await?;
    let sql_base = config.sql_base();

    // Get current version
    let current_version = get_current_version(&pool, &config_no_dry).await?;
    info!("Current database version: {:?}", current_version.as_deref().unwrap_or("none"));

    // Find migration files needed
    let migrations = find_migrations_to_target(&sql_base, current_version.clone(), &target)?;

    if migrations.is_empty() {
        if current_version.as_deref() == Some(&target) {
            info!("Already at version '{}'", target);
        } else {
            info!("No migrations needed to reach version '{}'", target);
        }
        return Ok(());
    }

    info!("Found {} migration(s) to apply", migrations.len());

    // Apply each migration
    for migration_file in migrations {
        let full_path = Path::new(&sql_base).join(&migration_file);
        info!("Applying migration: {}", migration_file);

        if !full_path.exists() {
            return Err(format!("Migration file not found: {}", full_path.display()).into());
        }

        let migration_sql = std::fs::read_to_string(&full_path)?;
        
        // Extract target version from migration file path (folder name is the target)
        // Structure: sql/<target>/<source>.sql means migration from source to target
        let target_version = migration_file.split('/').next();
        
        // Wrap migration and version update in a transaction
        let transactional_sql = if let Some(target_v) = target_version {
            format!(
                "BEGIN;\n{}\nINSERT INTO {{{{ref.meta}}}} (\"key\", \"value\") VALUES ('db_version', '{}') ON CONFLICT (\"key\") DO UPDATE SET \"value\" = '{}';\nCOMMIT;",
                migration_sql,
                target_v,
                target_v
            )
        } else {
            format!("BEGIN;\n{}\nCOMMIT;", migration_sql)
        };
        
        db::run_raw(&pool, config, &transactional_sql).await?;
        
        if let Some(target_v) = target_version {
            info!("Updated version to '{}'", target_v);
        }
    }

    info!("Successfully migrated to version '{}'", target);
    Ok(())
}

