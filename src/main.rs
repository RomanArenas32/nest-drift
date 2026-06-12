mod commands;
mod core;
mod parser;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "nest-drift")]
#[command(about = "NestJS schema consistency validator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate entities vs DTOs consistency
    Check {
        /// Path to the NestJS project
        #[arg(default_value = ".")]
        path: String,
    },
    /// Generate a snapshot of the current schema
    Snapshot {
        /// Path to the NestJS project
        #[arg(default_value = ".")]
        path: String,
        /// Output file for the snapshot
        #[arg(short, long, default_value = "nest-drift.snapshot.json")]
        output: String,
    },
    /// Compare current schema against a snapshot
    Diff {
        /// Snapshot file to compare against
        #[arg(default_value = "nest-drift.snapshot.json")]
        snapshot: String,
        /// Path to the NestJS project
        #[arg(default_value = ".")]
        path: String,
    },
    /// Validate LLM tool definitions against the codebase
    Validate {
        /// Path to the LLM tools definition file (JSON/YAML)
        tools: String,
        /// Path to the NestJS project
        #[arg(default_value = ".")]
        path: String,
    },
    /// Watch for file changes and re-run check automatically
    Watch {
        /// Path to the NestJS project
        #[arg(default_value = ".")]
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { path } => commands::check::run(&path),
        Commands::Snapshot { path, output } => commands::snapshot::run(&path, &output),
        Commands::Diff { snapshot, path } => commands::diff::run(&snapshot, &path),
        Commands::Validate { tools, path } => commands::validate::run(&tools, &path),
        Commands::Watch { path } => commands::watch::run(&path),
    }
}
