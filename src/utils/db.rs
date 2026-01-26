use tracing::{info, warn, debug};

use crate::config::Config;
use crate::utils::blocks::Block;
use crate::utils::references::get_reference;

#[derive(Clone)]
pub enum DbPool {
    Postgres(sqlx::PgPool),
    MySql(sqlx::MySqlPool),
    //Mssql(sqlx::MssqlPool),
    DryRun,
}

/// Get a database connection pool from configuration
pub async fn get_db_pool(config: &Config) -> Result<DbPool, sqlx::Error> {
    if config.dry_run {
        return Ok(DbPool::DryRun);
    }

    let database_url = config.database_url();
    
    match config.sql_dialect.as_str() {
        "postgres" | "postgresql" => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await?;
            Ok(DbPool::Postgres(pool))
        }
        "mysql" => {
            let pool = sqlx::mysql::MySqlPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await?;
            Ok(DbPool::MySql(pool))
        }
        // "mssql" => {
        //     let pool = sqlx::mssql::MssqlPoolOptions::new()
        //         .max_connections(5)
        //         .connect(&database_url)
        //         .await?;
        //     Ok(DbPool::Mssql(pool))
        // }
        dialect => {
            Err(sqlx::Error::Configuration(format!("Unsupported SQL dialect: {}", dialect).into()))
        }
    }
}

pub fn inject_variables(sql: &str, config: &Config) -> String {
    let mut result = sql.to_string();
    
    // Inject database configuration variables using dot notation
    // Support both {{database.X}} and {{db.X}} as aliases
    result = result
        .replace("{{database.sa.user}}", &config.database.sa.user)
        .replace("{{db.sa.user}}", &config.database.sa.user)
        .replace("{{database.sa.password}}", &config.database.sa.password)
        .replace("{{db.sa.password}}", &config.database.sa.password)
        .replace("{{database.api.user}}", &config.database.api.user)
        .replace("{{db.api.user}}", &config.database.api.user)
        .replace("{{database.api.password}}", &config.database.api.password)
        .replace("{{db.api.password}}", &config.database.api.password)
        .replace("{{database.host}}", &config.database.host)
        .replace("{{db.host}}", &config.database.host)
        .replace("{{database.port}}", &config.database.port.to_string())
        .replace("{{db.port}}", &config.database.port.to_string())
        .replace("{{database.name}}", &config.database.name)
        .replace("{{db.name}}", &config.database.name)
        .replace("{{database.ssl}}", if config.database.ssl { "true" } else { "false" })
        .replace("{{db.ssl}}", if config.database.ssl { "true" } else { "false" });
    
    // Inject references using dot notation
    // Support both {{references.<name>}} and {{ref.<name>}} as aliases
    for (key, _) in &config.references {
        let placeholder_long = format!("{{{{references.{}}}}}", key);
        let placeholder_short = format!("{{{{ref.{}}}}}", key);
        if let Ok(reference) = get_reference(config, key) {
            result = result.replace(&placeholder_long, &reference);
            result = result.replace(&placeholder_short, &reference);
        }
    }
    
    result
}

pub fn inject_secrets(sql: &str, config: &Config) -> String {
    let mut result = sql.to_string();
    
    // Inject secrets - always plain strings with no transformation
    for (key, value) in &config.secrets {
        let placeholder = format!("{{{{secrets.{}}}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    result
}

/// Execute a prepared SQL statement
pub async fn run(pool: &DbPool, config: &Config, sql: &str) -> Result<(), sqlx::Error> {
    let sql_with_vars = inject_variables(sql, config);
    let sql_final = inject_secrets(&sql_with_vars, config);
    
    // Log SQL, redacting secrets if log_secrets is false
    if config.log_sql {
        let sql_to_log = if config.log_secrets {
            sql_final.clone()
        } else {
            let mut redacted = sql_final.clone();
            for (key, _) in &config.secrets {
                let placeholder = format!("{{{{secrets.{}}}}}", key);
                redacted = redacted.replace(&placeholder, "REDACTED");
            }
            redacted
        };
        tracing::debug!(target: "sql", "{};", sql_to_log);
    }

    match pool {
        DbPool::Postgres(p) => {
            sqlx::query(&sql_final).execute(p).await?;
        }
        DbPool::MySql(p) => {
            sqlx::query(&sql_final).execute(p).await?;
        }
        // DbPool::Mssql(p) => {
        //     sqlx::query(&sql_final).execute(p).await?;
        // }
        DbPool::DryRun => {}
    }
    Ok(())
}

/// Execute a raw SQL statement with optional schema context
pub async fn run_raw(pool: &DbPool, config: &Config, sql: &str) -> Result<(), sqlx::Error> {
    run_raw_with_schema(pool, config, sql, None).await
}

/// Execute a raw SQL statement with optional schema context
pub async fn run_raw_with_schema(pool: &DbPool, config: &Config, sql: &str, schema: Option<&str>) -> Result<(), sqlx::Error> {
    // Optionally prepend SET search_path if schema is provided and auto_set_search_path is enabled
    let sql = if let Some(s) = schema {
        if config.auto_set_search_path {
            let set_search_path_sql = format!("SET search_path TO {};", s);
            format!("{}\n{}", set_search_path_sql, sql)
        } else {
            sql.to_string()
        }
    } else {
        sql.to_string()
    };

    let sql_with_vars = inject_variables(&sql, config);
    let sql_final = inject_secrets(&sql_with_vars, config);
    
    // Log SQL, redacting secrets if log_secrets is false
    if config.log_sql {
        let sql_to_log = if config.log_secrets {
            sql_final.clone()
        } else {
            let mut redacted = sql_final.clone();
            for (key, _) in &config.secrets {
                let placeholder = format!("{{{{secrets.{}}}}}", key);
                redacted = redacted.replace(&placeholder, "REDACTED");
            }
            redacted
        };
        tracing::debug!(target: "sql", "{}", sql_to_log);
        tracing::debug!(target: "sql", "");
    }

    match pool {
        DbPool::Postgres(p) => {
            sqlx::raw_sql(&sql_final).execute(p).await?;
        }
        DbPool::MySql(p) => {
            sqlx::raw_sql(&sql_final).execute(p).await?;
        }
        // DbPool::Mssql(p) => {
        //     sqlx::raw_sql(&sql_final).execute(p).await?;
        // }
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
        DbPool::MySql(p) => {
            let db_name = &config.database.name;
            let exists = sqlx::query("SELECT 1 FROM information_schema.schemata WHERE schema_name = ?")
                .bind(db_name)
                .fetch_optional(p)
                .await?
                .is_some();

            Ok(exists)
        },
        // DbPool::Mssql(p) => {
        //     let db_name = &config.database.name;
        //     let exists = sqlx::query("SELECT 1 FROM sys.databases WHERE name = ?")
        //         .bind(db_name)
        //         .fetch_optional(p)
        //         .await?
        //         .is_some();

        //     Ok(exists)
        // },
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
        DbPool::MySql(_p) => {
            // MySQL: roles/users are simplified, not implemented for this check
            Ok(false)
        },
        // DbPool::Mssql(_p) => {
        //     // MSSQL: principals/logins handling is different, not implemented for this check
        //     Ok(false)
        // },
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
        DbPool::MySql(_p) => {
            // MySQL: Check if database is empty - simplified check
            Ok(false)  // TODO: implement proper check
        },
        // DbPool::Mssql(_p) => {
        //     // MSSQL: Check if database is empty - simplified check
        //     Ok(false)  // TODO: implement proper check
        // },
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
            // Terminate all connections to the database before dropping
            let terminate_sql = format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}' AND pid <> pg_backend_pid()",
                db_name
            );
            if let DbPool::Postgres(p) = &pool {
                let _ = sqlx::query(&terminate_sql).execute(p).await;
            }
            
            let drop_sql = format!("DROP DATABASE IF EXISTS \"{}\"", db_name);
            run(&pool, &maintenance, &drop_sql).await?;
        }
        let sql = format!("CREATE DATABASE \"{}\"", db_name);
        run(&pool, &maintenance, &sql).await?;
    } else {
        // In production, check if exists
        if exists {
            // Check if the database is empty
            if is_database_empty(&pool, &maintenance).await? {
                warn!("Warning: Database '{}' already exists but is empty. Skipping CREATE DATABASE.", db_name);
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
                    // Execute block with its schema context
                    if config.log_sql && !config.dry_run {
                        debug!("-- @block {}", id);
                    }

                    run_raw_with_schema(&pool, config, &block_map[&id].sql, Some(&block_map[&id].schema)).await?;
                    
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
                info!("(Err) Block: {} depends on: {:?}", block_id, deps);
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

    #[test]
    fn test_inject_variables_with_new_format() {
        let mut config = Config::default();
        config.sql_dialect = "postgres".to_string();
        config.database.sa.user = "sa_user".to_string();
        config.database.sa.password = "sa_pass".to_string();
        config.database.api.user = "api_user".to_string();
        config.database.api.password = "api_pass".to_string();
        config.database.host = "localhost".to_string();
        config.database.port = 5432;
        config.database.name = "mydb".to_string();

        let sql = "CONNECT {{database.host}}:{{database.port}}/{{database.name}} AS {{database.sa.user}}:{{database.sa.password}}";
        let result = inject_variables(sql, &config);
        assert_eq!(result, "CONNECT localhost:5432/mydb AS sa_user:sa_pass");
    }

    #[test]
    fn test_inject_variables_with_database_alias() {
        let mut config = Config::default();
        config.database.sa.user = "sa_user".to_string();
        config.database.host = "localhost".to_string();
        config.database.port = 5432;

        let sql = "CONNECT {{db.host}}:{{db.port}} AS {{db.sa.user}}";
        let result = inject_variables(sql, &config);
        assert_eq!(result, "CONNECT localhost:5432 AS sa_user");
    }

    #[test]
    fn test_inject_variables_with_references_new_format() {
        let mut config = Config::default();
        config.sql_dialect = "postgres".to_string();
        config.references.insert("meta_key".to_string(), crate::config::ReferenceValue::StringArray(vec![
            "public".to_string(),
            "meta".to_string(),
        ]));

        let sql = "SELECT * FROM {{references.meta_key}}";
        let result = inject_variables(sql, &config);
        assert_eq!(result, "SELECT * FROM \"public\".\"meta\"");
    }

    #[test]
    fn test_inject_variables_with_references_short_alias() {
        let mut config = Config::default();
        config.sql_dialect = "postgres".to_string();
        config.references.insert("meta_key".to_string(), crate::config::ReferenceValue::StringArray(vec![
            "public".to_string(),
            "meta".to_string(),
        ]));

        let sql = "SELECT * FROM {{ref.meta_key}}";
        let result = inject_variables(sql, &config);
        assert_eq!(result, "SELECT * FROM \"public\".\"meta\"");
    }

    #[test]
    fn test_inject_secrets() {
        let mut config = Config::default();
        config.secrets.insert("api_key".to_string(), "secret123".to_string());
        config.secrets.insert("password".to_string(), "pass456".to_string());

        let sql = "CONNECT WITH {{secrets.api_key}} AND {{secrets.password}}";
        let result = inject_secrets(sql, &config);
        assert_eq!(result, "CONNECT WITH secret123 AND pass456");
    }

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
