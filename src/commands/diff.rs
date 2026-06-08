use colored::Colorize;
use std::fs;

use crate::parser::{self, Field, ProjectSchema};

pub fn run(snapshot_path: &str, path: &str) {
    println!("{}", "nest-drift diff".bold());
    println!("Snapshot: {}", snapshot_path.dimmed());
    println!("Project:  {}\n", path.dimmed());

    let snapshot = load_snapshot(snapshot_path);

    let current = ProjectSchema {
        entities: parser::entity::parse_project(path),
        dtos: parser::dto::parse_project(path),
    };

    let mut changes_found = false;

    // --- Diff entities ---
    println!("{}", "Entities".bold().underline());
    changes_found |= diff_schemas(
        "Entity",
        &snapshot.entities.iter().map(|e| (&e.name, &e.fields)).collect::<Vec<_>>(),
        &current.entities.iter().map(|e| (&e.name, &e.fields)).collect::<Vec<_>>(),
    );

    println!();

    // --- Diff DTOs ---
    println!("{}", "DTOs".bold().underline());
    changes_found |= diff_schemas(
        "DTO",
        &snapshot.dtos.iter().map(|d| (&d.name, &d.fields)).collect::<Vec<_>>(),
        &current.dtos.iter().map(|d| (&d.name, &d.fields)).collect::<Vec<_>>(),
    );

    println!();
    if changes_found {
        println!("{}", "Schema has changed since last snapshot.".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "No changes detected.".green().bold());
    }
}

fn load_snapshot(path: &str) -> ProjectSchema {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!(
            "{} Could not read snapshot '{}': {}",
            "ERROR".red().bold(),
            path,
            e
        );
        eprintln!("Run `nest-drift snapshot` first to generate one.");
        std::process::exit(1);
    });

    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("{} Invalid snapshot file: {}", "ERROR".red().bold(), e);
        std::process::exit(1);
    })
}

/// Returns true if any changes were found.
fn diff_schemas(
    kind: &str,
    before: &[(&String, &Vec<Field>)],
    after: &[(&String, &Vec<Field>)],
) -> bool {
    let mut changes = false;

    // Added schemas
    for (name, _) in after {
        if !before.iter().any(|(n, _)| n == name) {
            println!("  {} {} {} (new)", "+".green().bold(), kind, name.green());
            changes = true;
        }
    }

    // Removed schemas
    for (name, _) in before {
        if !after.iter().any(|(n, _)| n == name) {
            println!("  {} {} {} (removed)", "-".red().bold(), kind, name.red());
            changes = true;
        }
    }

    // Modified schemas
    for (name, before_fields) in before {
        if let Some((_, after_fields)) = after.iter().find(|(n, _)| n == name) {
            let field_changes = diff_fields(before_fields, after_fields);
            if !field_changes.is_empty() {
                println!("  {} {}: {}", "~".yellow().bold(), kind, name.yellow());
                for change in &field_changes {
                    println!("    {}", change);
                }
                changes = true;
            } else {
                println!("  {} {}: {}", " ".normal(), kind, name.dimmed());
            }
        }
    }

    changes
}

fn diff_fields(before: &[Field], after: &[Field]) -> Vec<String> {
    let mut changes = Vec::new();

    for f in after {
        match before.iter().find(|b| b.name == f.name) {
            None => changes.push(format!(
                "{} field '{}': {} (added)",
                "+".green(),
                f.name.green(),
                f.field_type
            )),
            Some(b) => {
                if b.field_type != f.field_type {
                    changes.push(format!(
                        "{} field '{}': {} -> {} (type changed)",
                        "~".yellow(),
                        f.name.yellow(),
                        b.field_type.red(),
                        f.field_type.green()
                    ));
                }
                if b.optional != f.optional {
                    let before_opt = if b.optional { "optional" } else { "required" };
                    let after_opt = if f.optional { "optional" } else { "required" };
                    changes.push(format!(
                        "{} field '{}': {} -> {} (optionality changed)",
                        "~".yellow(),
                        f.name.yellow(),
                        before_opt.red(),
                        after_opt.green()
                    ));
                }
            }
        }
    }

    for f in before {
        if !after.iter().any(|a| a.name == f.name) {
            changes.push(format!(
                "{} field '{}': {} (removed)",
                "-".red(),
                f.name.red(),
                f.field_type
            ));
        }
    }

    changes
}
