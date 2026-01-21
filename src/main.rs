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
        version: Option<String>,
    },
    /// Run tests in the database
    Test { version: Option<String> },
    /// Create a new release
    Release { },
    /// Run code generation
    Generate { version: Option<String> },
}

fn main() {
    let cli = Cli::parse();

    // Load configuration before running any commands
    let config = match config::Config::load(None) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            std::process::exit(1);
        }
    };

    log::init_logging(&config);
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    
    let result = rt.block_on(async {
        match cli.command {
            Commands::Init { version, dry_run } => {
                let config = config::Config {
                    dry_run: config.dry_run || dry_run,
                    ..config.clone()
                };
                commands::init::execute(&config, version).await
            }
        Commands::Plan { from, to } => {
            commands::plan::execute(&config, from, to).await
        }
        Commands::Migration { plan, apply, version } => {
            if plan {
                println!("TODO: migration --plan {:?}", version.unwrap_or_else(|| "next".to_string()));
            } else if apply {
                println!("TODO: migration --apply {:?}", version.unwrap_or_else(|| "next".to_string()));
            } else {
                println!("TODO: migration {:?}", version.unwrap_or_else(|| "next".to_string()));
            }
            Ok(())
        }
        Commands::Release {} => {
            commands::release::execute(&config).await
        }
        Commands::Test { version } => {
            println!("TODO: check {:?}", version.unwrap_or_else(|| "next".to_string()));
            Ok(())
        }
        Commands::Generate { version } => {
            println!("TODO: generate {:?}", version.unwrap_or_else(|| "next".to_string()));
            Ok(())
        }
        }
    });
    
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
