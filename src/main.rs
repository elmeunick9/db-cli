use clap::{Parser, Subcommand};

mod config;
mod log;
mod commands;
mod utils;

#[derive(Parser)]
#[command(name = "db")]
#[command(about = "DB-CLI - manage SQL databases", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the database
    Init {
        version: Option<String>,
        #[arg(long, default_value = "false")]
        dry_run: bool
    },
    /// Create a migration plan
    Plan {
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>
    },
    /// Manage migrations (use --plan or --apply)
    Migration {
        #[arg(long)]
        plan: bool,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        to: Option<String>,
        version: Option<String>,
    },
    /// Run tests in the database
    Test { version: Option<String> },
    /// Create a new release
    Release { },
}

fn main() {
    let cli = Cli::parse();

    let root_config_path = find_root_config_path();

    // Load root configuration (from repo root, if found)
    let root_config = match config::Config::load(root_config_path.as_deref()) {
        Ok((cfg, _)) => cfg,
        Err(e) => {
            eprintln!("Failed to load root configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Load configuration from current working directory if it has a db.toml
    let cwd_config = if std::path::Path::new("db.toml").exists() {
        match config::Config::load(Some("db.toml")) {
            Ok((cfg, _)) => cfg,
            Err(e) => {
                eprintln!("Failed to load configuration: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        root_config.clone()
    };

    // Determine which databases to work with
    let working_dbs = determine_working_dbs(&cwd_config);
    
    if working_dbs.is_empty() {
        eprintln!("No databases configured. Set working_db in db.toml or use WORKING_DB environment variable.");
        std::process::exit(1);
    }

    let db_count = working_dbs.len();
    log::init_logging(&cwd_config);
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    
    let mut any_failed = false;
    for db_path in working_dbs {
        // Load config for this specific database (with hierarchical overrides)
        let config = match root_config.clone().merge_from(&db_path) {
            Ok(cfg) => config::Config { base: config::SqlBase::Single(db_path.clone()), ..cfg },
            Err(e) => {
                eprintln!("Failed to load configuration for '{}': {}", db_path, e);
                any_failed = true;
                continue;
            }
        };

        if db_count > 1 {
            tracing::info!("Processing database: {}", db_path);
        }

        let result = rt.block_on(async {
            match cli.command {
                Commands::Init { ref version, dry_run } => {
                    let config = config::Config {
                        dry_run: config.dry_run || dry_run,
                        ..config.clone()
                    };
                    commands::init::execute(&config, version.clone()).await
                }
                Commands::Plan { ref from, ref to } => {
                    commands::plan::execute(&config, from.clone(), to.clone()).await
                }
                Commands::Migration { plan, apply, ref from, ref to , ref version } => {
                    if plan {
                        commands::plan::execute(&config, from.clone(), to.clone()).await?;
                    } else if apply {
                        commands::apply::execute(&config, version.clone()).await?;
                    } else {
                        tracing::info!("Please specify either --plan or --apply.");
                    }
                    Ok(())
                }
                Commands::Release {} => {
                    commands::release::execute(&config).await
                }
                Commands::Test { ref version } => {
                    commands::test::execute(&config, version.clone()).await
                }
            }
        });
        
        if let Err(e) = result {
            eprintln!("Error processing '{}': {}", db_path, e);
            any_failed = true;
        }
    }

    if any_failed {
        std::process::exit(1);
    }
}

fn find_root_config_path() -> Option<String> {
    let mut dir = std::env::current_dir().ok()?;
    let mut last_found: Option<String> = None;

    loop {
        let candidate = dir.join("db.toml");
        if candidate.exists() {
            last_found = Some(candidate.to_string_lossy().to_string());
        }

        if !dir.pop() {
            break;
        }
    }

    last_found
}

/// Determine which databases to work with based on env var and config semantics
fn determine_working_dbs(cwd_config: &config::Config) -> Vec<String> {
    // Check environment variable first
    if let Ok(env_dbs) = std::env::var("WORKING_DB") {
        return env_dbs.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    let sql_base_list = cwd_config.sql_base_list();
    if sql_base_list.is_empty() {
        return vec![];
    }

    let is_local = sql_base_list.len() == 1 && {
        let base = sql_base_list[0].trim();
        base.is_empty() || base == "." || base == "./"
    };

    if is_local {
        let base = sql_base_list[0].trim();
        return vec![if base.is_empty() { ".".to_string() } else { base.to_string() }];
    }

    // Root config: working_db is a subset of sql_base
    if cwd_config.working_db.is_empty() {
        return sql_base_list;
    }

    cwd_config.working_db.clone()
}
