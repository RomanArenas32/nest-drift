use colored::Colorize;

use crate::core::diff as core_diff;
use crate::core::{FieldChangeKind, SchemaChangeKind, SchemaType};

pub fn run(snapshot_path: &str, path: &str) {
    println!("{}", "nest-drift diff".bold());
    println!("Snapshot: {}", snapshot_path.dimmed());
    println!("Project:  {}\n", path.dimmed());

    let report = match core_diff::run(snapshot_path, path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} {}", "ERROR".red().bold(), e);
            if e.contains("Could not read snapshot") {
                eprintln!("Run `nest-drift snapshot` first to generate one.");
            }
            std::process::exit(1);
        }
    };

    let entity_changes: Vec<_> = report
        .changes
        .iter()
        .filter(|c| matches!(c.schema_type, SchemaType::Entity))
        .collect();

    let dto_changes: Vec<_> = report
        .changes
        .iter()
        .filter(|c| matches!(c.schema_type, SchemaType::Dto))
        .collect();

    println!("{}", "Entities".bold().underline());
    for change in &entity_changes {
        print_schema_change(change);
    }
    if entity_changes.is_empty() {
        println!("  {}", "No changes.".dimmed());
    }

    println!();
    println!("{}", "DTOs".bold().underline());
    for change in &dto_changes {
        print_schema_change(change);
    }
    if dto_changes.is_empty() {
        println!("  {}", "No changes.".dimmed());
    }

    println!();
    if report.has_changes() {
        println!("{}", "Schema has changed since last snapshot.".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "No changes detected.".green().bold());
    }
}

fn print_schema_change(change: &crate::core::SchemaChange) {
    match change.kind {
        SchemaChangeKind::Added => {
            println!("  {} {} (new)", "+".green().bold(), change.name.green());
        }
        SchemaChangeKind::Removed => {
            println!("  {} {} (removed)", "-".red().bold(), change.name.red());
        }
        SchemaChangeKind::Modified => {
            println!("  {} {}", "~".yellow().bold(), change.name.yellow());
            for fc in &change.field_changes {
                match fc.kind {
                    FieldChangeKind::Added => println!(
                        "    {} field '{}': {} (added)",
                        "+".green(),
                        fc.field.green(),
                        fc.after.as_deref().unwrap_or("")
                    ),
                    FieldChangeKind::Removed => println!(
                        "    {} field '{}': {} (removed)",
                        "-".red(),
                        fc.field.red(),
                        fc.before.as_deref().unwrap_or("")
                    ),
                    FieldChangeKind::TypeChanged => println!(
                        "    {} field '{}': {} -> {} (type changed)",
                        "~".yellow(),
                        fc.field.yellow(),
                        fc.before.as_deref().unwrap_or("").red(),
                        fc.after.as_deref().unwrap_or("").green()
                    ),
                    FieldChangeKind::OptionalityChanged => println!(
                        "    {} field '{}': {} -> {} (optionality changed)",
                        "~".yellow(),
                        fc.field.yellow(),
                        fc.before.as_deref().unwrap_or("").red(),
                        fc.after.as_deref().unwrap_or("").green()
                    ),
                }
            }
        }
    }
}
