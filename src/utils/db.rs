use sqlx::postgres::PgPoolOptions;
use crate::config::Config;

/// Get a database connection pool from configuration
pub async fn get_db_pool(config: &Config) -> Result<sqlx::PgPool, sqlx::Error> {
    let database_url = config.database_url();
    
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}

/// Create a database, using the connection in Config (connects to the maintenance DB)
pub async fn create_db(
    config: &Config,
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

    if config.is_dev() {
        // In dev mode, drop and recreate if exists
        if exists {
            let drop_sql = format!("DROP DATABASE \"{}\"", db_name);
            sqlx::query(&drop_sql).execute(&pool).await?;
        }
        let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
        sqlx::query(&create_sql).execute(&pool).await?;
    } else {
        // In production, fail if exists
        if exists {
            return Err(format!("Database '{}' already exists in production mode", db_name).into());
        }
        let create_sql = format!("CREATE DATABASE \"{}\"", db_name);
        sqlx::query(&create_sql).execute(&pool).await?;
    }

    // Create API role if not exists
    let api_user = &config.database.api.user;
    let api_password = &config.database.api.password;
    let role_exists = sqlx::query("SELECT 1 FROM pg_roles WHERE rolname = $1")
        .bind(api_user)
        .fetch_optional(&pool)
        .await?
        .is_some();

    if !role_exists {
        let create_role_sql = format!("CREATE ROLE \"{}\" LOGIN PASSWORD '{}'", api_user, api_password);
        sqlx::query(&create_role_sql).execute(&pool).await?;
    }

    Ok(())
}
