use crate::config::Config;
use crate::utils::blocks::Block;

#[derive(Clone)]
pub enum DbPool {
    Postgres(sqlx::PgPool),
    // MySql(sqlx::MySqlPool),
    DryRun,
}

/// Get a database connection pool from configuration
pub async fn get_db_pool(config: &Config) -> Result<DbPool, sqlx::Error> {
    if config.dry_run {
        return Ok(DbPool::DryRun);
    }

    let database_url = config.database_url();
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
        
    Ok(DbPool::Postgres(pool))
}

fn inject_variables(sql: &str, config: &Config) -> String {
    sql
        .replace("{{DB_SA_USER}}", &config.database.sa.user)
        .replace("{{DB_SA_PASSWORD}}", &config.database.sa.password)
        .replace("{{DB_API_USER}}", &config.database.api.user)
        .replace("{{DB_API_PASSWORD}}", &config.database.api.password)
}

/// Execute a prepared SQL statement
pub async fn run(pool: &DbPool, config: &Config, sql: &str) -> Result<(), sqlx::Error> {
    if config.log_sql {
        tracing::debug!(target: "sql", "{};", sql);
    }

    let sql = inject_variables(sql, config);
    match pool {
        DbPool::Postgres(p) => {
            sqlx::query(&sql).execute(p).await?;
        }
        DbPool::DryRun => {}
    }
    Ok(())
}

/// Execute a raw SQL statement
pub async fn run_raw(pool: &DbPool, config: &Config, sql: &str) -> Result<(), sqlx::Error> {
    if config.log_sql {
        tracing::debug!(target: "sql", "{}", sql);
        tracing::debug!(target: "sql", "");
    }

    let sql = inject_variables(sql, config);
    match pool {
        DbPool::Postgres(p) => {
            sqlx::raw_sql(&sql).execute(p).await?;
        }
        DbPool::DryRun => {}
    }
    Ok(())
}

/// Check if the target database exists
async fn db_exists(pool: &DbPool, config: &Config) -> Result<bool, Box<dyn std::error::Error>> {
    if config.dry_run { return Ok(false); }

    match pool {
        DbPool::Postgres(p) => {
            let db_name = &config.database.name;
            let exists = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
                .bind(db_name)
                .fetch_optional(p)
                .await?
                .is_some();

            Ok(exists)
        },
        DbPool::DryRun => return Ok(false),
    }
}

/// Check if the role exists
async fn role_exists(pool: &DbPool, config: &Config, role_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    if config.dry_run { return Ok(false); }

    match pool {
        DbPool::Postgres(p) => {
            let exists = sqlx::query("SELECT 1 FROM pg_roles WHERE rolname = $1")
                .bind(role_name)
                .fetch_optional(p)
                .await?
                .is_some();

            Ok(exists)
        },
        DbPool::DryRun => return Ok(false),
    }
}

/// Check if the target database is empty (has no user-created objects and only the public schema)
async fn is_database_empty(pool: &DbPool, config: &Config) -> Result<bool, Box<dyn std::error::Error>> {
    if config.dry_run { return Ok(true); }

    match pool {
        DbPool::Postgres(p) => {
            // Check for user-created schemas (excluding system schemas and public)
            let schema_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM information_schema.schemata WHERE schema_name NOT IN ('information_schema', 'pg_catalog', 'pg_toast', 'pg_temp_1', 'pg_toast_temp_1') AND schema_name != 'public'"
            )
            .fetch_one(p)
            .await?;

            // Check for tables in public schema
            let table_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
            )
            .fetch_one(p)
            .await?;

            // Check for views in public schema
            let view_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM information_schema.views WHERE table_schema = 'public'"
            )
            .fetch_one(p)
            .await?;

            // Check for functions in public schema
            let function_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM information_schema.routines WHERE routine_schema = 'public' AND routine_type = 'FUNCTION'"
            )
            .fetch_one(p)
            .await?;

            Ok(schema_count == 0 && table_count == 0 && view_count == 0 && function_count == 0)
        },
        DbPool::DryRun => return Ok(true)
    }
}

/// Create a database, using the connection in Config (connects to the maintenance DB)
pub async fn create_db(
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to maintenance DB to create or manage target DB
    let mut maintenance = config.clone();
    maintenance.database.name = config.database.maintenance_db_name.clone();
    let pool = get_db_pool(&maintenance).await?;

    let db_name = &config.database.name;
    let exists = db_exists(&pool, &maintenance).await?;

    if config.is_dev() {
        // In dev mode, drop and recreate if exists
        if exists {
            let drop_sql = format!("DROP DATABASE \"{}\"", db_name);
            run(&pool, &maintenance, &drop_sql).await?;
        }
        let sql = format!("CREATE DATABASE \"{}\"", db_name);
        run(&pool, &maintenance, &sql).await?;
    } else {
        // In production, check if exists
        if exists {
            // Check if the database is empty
            if is_database_empty(&pool, &maintenance).await? {
                println!("Warning: Database '{}' already exists but is empty. Skipping CREATE DATABASE.", db_name);
            } else {
                return Err(format!("Database '{}' already exists and is not empty in production mode", db_name).into());
            }
        } else {
            let sql = format!("CREATE DATABASE \"{}\"", db_name);
            run(&pool, &maintenance, &sql).await?;
        }
    }

    // Create API role if not exists
    let api_user = &config.database.api.user;
    let role_exists = role_exists(&pool, &maintenance, api_user).await?;

    if !role_exists {
        let sql = "CREATE ROLE '{{DB_API_USER}}' WITH LOGIN PASSWORD '{{DB_API_PASSWORD}}'";
        run(&pool, config, &sql).await?;
    }

    Ok(())
}

/// Create a schema if it doesn't exist, set sa as owner, and grant api user data permissions
pub async fn create_schema(
    config: &Config,
    schema_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let pool = get_db_pool(config).await?;

    // Create schema if not exists
    let create_schema_sql = format!("CREATE SCHEMA IF NOT EXISTS \"{}\"", schema_name);
    run(&pool, config, &create_schema_sql).await?;

    // Set sa as owner
    let sa_user = &config.database.sa.user;
    let set_owner_sql = format!("ALTER SCHEMA \"{}\" OWNER TO \"{}\"", schema_name, sa_user);
    run(&pool, config, &set_owner_sql).await?;

    // Grant api user USAGE on schema
    let api_user = &config.database.api.user;
    let grant_usage_sql = format!("GRANT USAGE ON SCHEMA \"{}\" TO \"{}\"", schema_name, api_user);
    run(&pool, config, &grant_usage_sql).await?;

    // Grant data modification permissions on all existing tables in the schema
    let grant_table_sql = format!(
        "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA \"{}\" TO \"{}\"",
        schema_name, api_user
    );
    run(&pool, config, &grant_table_sql).await?;

    // Set default privileges for future tables
    let default_priv_sql = format!(
        "ALTER DEFAULT PRIVILEGES FOR ROLE {} IN SCHEMA \"{}\" GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO \"{}\"",
        sa_user, schema_name, api_user
    );
    run(&pool, config, &default_priv_sql).await?;

    Ok(())
}

/// Execute blocks in the correct order respecting requires dependencies
pub async fn execute_blocks(config: &Config, blocks: &Vec<Block>) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;

    // Build dependency graph: key is absolute block id, value is list of dependencies
    let mut dep_graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut block_map: HashMap<String, Block> = HashMap::new();

    for block in blocks {
        dep_graph.insert(block.name.clone(), block.requires.clone());
        block_map.insert(block.name.clone(), block.clone());
    }

    // Validate that all dependencies exist
    let all_block_ids: std::collections::HashSet<String> = dep_graph.keys().cloned().collect();
    let mut missing_deps = vec![];
    for (block_id, deps) in &dep_graph {
        for dep in deps {
            if !all_block_ids.contains(dep) {
                let block = &block_map[block_id];
                missing_deps.push(format!("Block '{}' in {}:{} requires '{}', but it does not exist", block_id, block.file, block.line_number, dep));
            }
        }
    }
    if !missing_deps.is_empty() {
        return Err(format!("Missing dependencies:\n{}", missing_deps.join("\n")).into());
    }

    // Get pool if not dry run
    let pool = get_db_pool(config).await?;

    // Execute blocks in dependency order
    while !dep_graph.is_empty() {
        let mut removed = false;
        let keys: Vec<String> = dep_graph.keys().cloned().collect();

        for id in keys {
            if let Some(deps) = dep_graph.get(&id) {
                if deps.is_empty() {
                    // Execute block
                    // Set search path to the correct schema
                    let set_search_path_sql = format!("SET search_path TO {};", block_map[&id].schema);
                    let sql = if config.auto_set_search_path {
                        format!("{}\n{}", set_search_path_sql, block_map[&id].sql)
                    } else {
                        format!("{}", block_map[&id].sql)
                    };

                    if config.log_sql && !config.dry_run {
                        println!("-- @block {}", id);
                    }

                    run_raw(&pool, config, &sql).await?;
                    
                    // Remove from graph
                    dep_graph.remove(&id);
                    // Remove this id from all other dependencies
                    for (_, deps) in dep_graph.iter_mut() {
                        deps.retain(|d| d != &id);
                    }
                    removed = true;
                }
            }
        }

        if !removed {
            // Print debug info for remaining blocks and their dependencies
            for (block_id, deps) in &dep_graph {
                println!("Block: {} depends on: {:?}", block_id, deps);
            }
            return Err("Cycle detected in block dependencies".into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::blocks::Block;

    #[tokio::test]
    async fn test_cycle_detection_dry_run() {
        let blocks = vec![
            Block {
                schema: "test".to_string(),
                file: "test.sql".to_string(),
                name: "test.a".to_string(),
                requires: vec!["test.b".to_string()],
                sql: "SELECT 1;".to_string(),
                line_number: 20,
            },
            Block {
                schema: "test".to_string(),
                file: "test.sql".to_string(),
                name: "test.b".to_string(),
                requires: vec!["test.a".to_string()],
                sql: "SELECT 2;".to_string(),
                line_number: 10,
            },
        ];

        let mut config = Config::default();
        config.dry_run = true;
        let result = execute_blocks(&config, &blocks).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().starts_with("Cycle detected in block dependencies"));
    }
}
