use std::path::{Path};
use crate::utils::db;
use crate::utils::fs;
use crate::utils::blocks;
use crate::config::Config;
use tracing::{info, debug};

/// Initialize the database
pub async fn execute(config: &Config, version: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let sql_base = config.sql_base();
    let mut version = version.unwrap_or_else(|| "next".to_string());

    // Resolve version aliases like "latest" to a concrete version folder
    if version == "latest" {
        match fs::get_latest_version(sql_base) {
            Ok(v) => {
                version = v;
            }
            Err(e) => {
                return Err(format!("failed to resolve 'latest' version: {}", e).into());
            }
        }
    }

    info!("Initializing database for version: {}", version);
    info!("Mode: {}", if config.is_dev() { "development" } else { "production" });
    if config.dry_run {
        println!("Dry run mode enabled.");
    }
    
    let version_dir = Path::new(sql_base).join(&version);
    if !version_dir.exists() {
        return Err(format!("Version directory not found: {}", version_dir.display()).into());
    }
    
    // 1) List schemas
    let schemas = fs::list_schemas(sql_base, &version)?;
    info!("Found schemas: {:?}", schemas);
    
    // 2) Create DB
    println!("--------");
    db::create_db(config).await?;

    // 3) Create schemas
    for schema in &schemas {
        db::create_schema(config, schema).await?;
    }

    // 4) Execute db.sql
    let db_sql_path = version_dir.join("db.sql");
    if db_sql_path.exists() {
        let db_sql_content = std::fs::read_to_string(&db_sql_path)?;
        let pool = db::get_db_pool(config).await?;
        db::run_raw(&pool, config, &db_sql_content).await?;
    }

    // // 3) Read all files and parse blocks per schema
    let mut blocks: Vec<blocks::Block> = vec![];
    let mut files_count = 0;
    for schema in &schemas {
        let files = fs::read_schema_files(sql_base, &version, schema)?;
        files_count += files.len();
        for file in files {
            let bks = blocks::parse_blocks(schema, &file)?;
            blocks.extend(bks);
        }
    }

    debug!("-- Found {} files and {} blocks", files_count, blocks.len());

    // Normalize filepaths in requirements to absolute block identifiers
    blocks::normalize(&mut blocks)?;

    // Group blocks into layers based on file patterns
    let mut layers: Vec<Vec<blocks::Block>> = vec![vec![], vec![], vec![], vec![]];
    for block in &blocks {
        let file = &block.file;
        if file.ends_with("schema.sql") || file.ends_with(".table.sql") || file.ends_with(".function.sql") || file.ends_with(".view.sql") {
            layers[0].push(block.clone());
        } else if file.ends_with(".trigger.sql") || file.ends_with(".index.sql") {
            layers[1].push(block.clone());
        } else if file.ends_with("default.sql") || file.ends_with(".default.sql") {
            layers[2].push(block.clone());
        } else if file.ends_with("insert.sql") || file.ends_with(".insert.sql") {
            layers[3].push(block.clone());
        }
        // Note: Assuming all files match one of the patterns; if not, they are ignored
    }

    // First, check for cycles in all layers (dry run)
    for (i, layer) in layers.iter().enumerate() {
        debug!("-- Checking cycles for layer {}", i + 1);
        let config = &Config { dry_run: true, log_sql: false, ..config.clone() };
        db::execute_blocks(&config, layer).await?;
    }

    // Then, execute all layers
    for (i, layer) in layers.iter().enumerate() {
        debug!("-- Executing layer {}", i + 1);
        db::execute_blocks(config, layer).await?;
    }

    println!("------------------");
    println!("Initialization complete");
    Ok(())
}
