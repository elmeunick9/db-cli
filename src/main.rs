use clap::{Parser, Subcommand};

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

    match cli.command {
        Commands::Init { version } => {
            println!("TODO: init {:?}", version.unwrap_or_else(|| "next".to_string()));
        }
        Commands::Plan { version } => {
            println!("TODO: plan {:?}", version.unwrap_or_else(|| "next".to_string()));
        }
        Commands::Migration { plan, apply, version } => {
            if plan {
                println!("TODO: migration --plan {:?}", version.unwrap_or_else(|| "next".to_string()));
            } else if apply {
                println!("TODO: migration --apply {:?}", version.unwrap_or_else(|| "next".to_string()));
            } else {
                println!("TODO: migration {:?}", version.unwrap_or_else(|| "next".to_string()));
            }
        }
        Commands::Release { version } => {
            println!("TODO: release {:?}", version.unwrap_or_else(|| "next".to_string()));
        }
        Commands::Check { version } => {
            println!("TODO: check {:?}", version.unwrap_or_else(|| "next".to_string()));
        }
        Commands::Generate { version } => {
            println!("TODO: generate {:?}", version.unwrap_or_else(|| "next".to_string()));
        }
    }
}
