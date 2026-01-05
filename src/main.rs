use clap::{Parser, Subcommand};

mod config;
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
    Init { version: Option<String> },
    /// Create a migration plan
    Plan { version: Option<String> },
    /// Manage migrations (use --plan or --apply)
    Migration { 
        #[arg(long)]
        plan: bool,
        #[arg(long)]
        apply: bool,
        version: Option<String>,
    },
    /// Create a new release
    Release { version: Option<String> },
    /// Run tests/checks
    Check { version: Option<String> },
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

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
    
    let result = rt.block_on(async {
        match cli.command {
            Commands::Init { version } => {
                commands::init::execute(&config, version).await
            }
        Commands::Plan { version } => {
            println!("TODO: plan {:?}", version.unwrap_or_else(|| "next".to_string()));
            Ok(())
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
        Commands::Release { version } => {
            println!("TODO: release {:?}", version.unwrap_or_else(|| "next".to_string()));
            Ok(())
        }
        Commands::Check { version } => {
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
