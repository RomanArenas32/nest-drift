use colored::Colorize;

use crate::core::validate as core_validate;

pub fn run(tools_path: &str, path: &str) {
    println!("{}", "nest-drift validate".bold());
    println!("Tools:   {}", tools_path.dimmed());
    println!("Project: {}\n", path.dimmed());

    let report = match core_validate::run(tools_path, path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{} {}", "ERROR".red().bold(), e);
            std::process::exit(1);
        }
    };

    if report.tool_count == 0 {
        println!("{}", "No tools found in file.".yellow());
        return;
    }

    println!("Found {} tools\n", report.tool_count.to_string().cyan());

    for result in &report.results {
        if result.issues.is_empty() {
            if let (Some(schema), Some(file)) = (&result.matched_schema, &result.matched_file) {
                println!(
                    "{} {} -> {} ({})",
                    "OK  ".green().bold(),
                    result.tool.green(),
                    schema.green(),
                    file.dimmed()
                );
            }
        } else {
            let first_issue = &result.issues[0];
            // Check if it's a skip (no schema) vs a real failure
            if matches!(first_issue.kind, crate::core::ValidateIssueKind::NoSchema) {
                println!(
                    "{} {} — no input_schema/parameters found, skipping",
                    "SKIP".yellow().bold(),
                    result.tool.yellow()
                );
            } else if matches!(first_issue.kind, crate::core::ValidateIssueKind::NoMatch) {
                println!(
                    "{} {} — no matching DTO or Entity found in codebase",
                    "WARN".yellow().bold(),
                    result.tool.yellow()
                );
            } else {
                println!(
                    "{} {} -> {}",
                    "FAIL".red().bold(),
                    result.tool.red(),
                    result.matched_schema.as_deref().unwrap_or("?").red()
                );
                for issue in &result.issues {
                    println!("     {}", issue.message.red());
                }
            }
        }
    }

    println!();
    if report.has_issues() {
        println!(
            "{}",
            "Validation failed. Tool definitions are out of sync."
                .red()
                .bold()
        );
        std::process::exit(1);
    } else {
        println!("{}", "All tools are valid.".green().bold());
    }
}
