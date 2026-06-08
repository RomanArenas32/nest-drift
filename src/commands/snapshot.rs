use colored::Colorize;
use std::fs;

use crate::parser::{self, ProjectSchema};

pub fn run(path: &str, output: &str) {
    println!("{}", "nest-drift snapshot".bold());
    println!("Scanning: {}\n", path.dimmed());

    let entities = parser::entity::parse_project(path);
    let dtos = parser::dto::parse_project(path);

    if entities.is_empty() && dtos.is_empty() {
        println!("{}", "Nothing found to snapshot.".yellow());
        return;
    }

    let schema = ProjectSchema { entities, dtos };

    let json = serde_json::to_string_pretty(&schema).expect("Failed to serialize schema");

    fs::write(output, &json).unwrap_or_else(|e| {
        eprintln!("{} Could not write snapshot: {}", "ERROR".red().bold(), e);
        std::process::exit(1);
    });

    println!(
        "{} Snapshot saved to {}",
        "OK".green().bold(),
        output.cyan()
    );
    println!(
        "   {} entities, {} DTOs captured",
        schema.entities.len().to_string().cyan(),
        schema.dtos.len().to_string().cyan()
    );
}
