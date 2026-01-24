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

/// Update the database version in the meta table
pub async fn set_current_version(pool: &DbPool, config: &Config, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    if config.dry_run {
        return Ok(());
    }

    match pool {
        DbPool::Postgres(p) => {
            let sql = &db::inject_variables(r#"INSERT INTO {{ref.meta}} ("key", "value") VALUES ('db_version', $1)"#, config);
            tracing::debug!(target: "sql", "{};", sql);

            sqlx::query(
                &db::inject_variables(sql, config)
            )
                .bind(version)
                .execute(p)
                .await?;
            Ok(())
        }
        DbPool::DryRun => Ok(()),
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
    
    for version in &numeric_versions {
        let version_dir = Path::new(sql_base).join(version);
        if !version_dir.exists() {
            continue;
        }
        
        let mut edges = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&version_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.ends_with(".sql") && file_name.len() > 4 {
                            let base_name = &file_name[..file_name.len() - 4];
                            if base_name.chars().all(|c| c.is_ascii_digit()) {
                                edges.push(base_name.to_string());
                            }
                        }
                    }
                }
            }
        }
        migration_graph.insert(version.clone(), edges);
    }

    // Check for "next" folder which might have migration files to numeric versions
    let next_dir = Path::new(sql_base).join("next");
    if next_dir.exists() {
        let mut next_migrations = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&next_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if file_name.ends_with(".sql") && file_name.len() > 4 {
                            let base_name = &file_name[..file_name.len() - 4];
                            if base_name.chars().all(|c| c.is_ascii_digit()) {
                                next_migrations.push(base_name.to_string());
                            }
                        }
                    }
                }
            }
        }
        migration_graph.insert("next".to_string(), next_migrations);
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
        migrations.push(format!("{}/{}.sql", prev, current));
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
        db::run_raw(&pool, config, &migration_sql).await?;

        // Extract target version from migration file path (last part before .sql)
        if let Some(target_v) = migration_file.split('/').last().and_then(|f| f.strip_suffix(".sql")) {
            set_current_version(&pool, config, target_v).await?;
            info!("Updated version to '{}'", target_v);
        }
    }

    info!("Successfully migrated to version '{}'", target);
    Ok(())
}

