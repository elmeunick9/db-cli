use sqlx::postgres::PgPoolOptions;
use std::fs;
use std::path::Path;
use crate::config::Config;

/// Get a database connection pool from configuration
pub async fn get_db_pool(config: &Config) -> Result<sqlx::PgPool, sqlx::Error> {
    let database_url = config.database_url();
    
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}

/// Execute a raw SQL string using sqlx
pub async fn execute_sql_string(
    pool: &sqlx::PgPool,
    sql: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if sql.trim().is_empty() {
        return Ok(());
    }
    // sqlx supports executing multiple statements in a single query for Postgres when using `execute` on raw SQL
    // We'll use `sqlx::query` with the full SQL string.
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

/// Create a database if it does not exist, using the connection in Config (connects to the maintenance DB)
pub async fn create_db_if_missing(
    config: &Config,
    db_sql_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to maintenance DB (postgres) to create or manage target DB
    let mut maintenance = config.clone();
    maintenance.database.name = "postgres".to_string();
    let pool = get_db_pool(&maintenance).await?;

    let db_name = &config.database.name;
    let exists = sqlx::query("SELECT 1 FROM pg_database WHERE datname = $1")
        .bind(db_name)
        .fetch_optional(&pool)
        .await?
        .is_some();

    if !exists {
        if let Some(path) = db_sql_path {
            // If db.sql provided, execute it on maintenance DB (it may contain CREATE DATABASE or other setup)
            let sql = fs::read_to_string(path)?;
            if !sql.trim().is_empty() {
                sqlx::query(&sql).execute(&pool).await?;
            }
        } else {
            let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
            sqlx::query(&create_sql).execute(&pool).await?;
        }
    }

    Ok(())
}

/// Execute a single SQL file against the database
pub async fn execute_sql_file(
    pool: &sqlx::PgPool,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Check if file exists
    if !Path::new(file_path).exists() {
        // Some files may not exist for all versions, skip them
        return Ok(());
    }
    
    // Read the SQL file
    let sql_content = fs::read_to_string(file_path)?;
    
    if sql_content.trim().is_empty() {
        return Ok(());
    }
    
    println!("Executing: {}", file_path);
    
    // Execute using sqlx
    sqlx::raw_sql(&sql_content)
        .execute(pool)
        .await?;
    
    Ok(())
}
