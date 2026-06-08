use colored::Colorize;

use crate::core::check as core_check;

pub fn run(path: &str) {
    println!("{}", "nest-drift check".bold());
    println!("Scanning: {}\n", path.dimmed());

    let report = core_check::run(path);

    if report.entity_count == 0 {
        println!("{}", "No @Entity classes found.".yellow());
        return;
    }

    println!(
        "Found {} entities, {} DTOs\n",
        report.entity_count.to_string().cyan(),
        report.dto_count.to_string().cyan()
    );

    for result in &report.results {
        let dto_name = result.dto.as_deref().unwrap_or("—");

        if result.is_ok() {
            println!(
                "{} {} <-> {}",
                "OK  ".green().bold(),
                result.entity.green(),
                dto_name.dimmed()
            );
        } else {
            println!(
                "{} {} <-> {}",
                "FAIL".red().bold(),
                result.entity.red(),
                dto_name.dimmed()
            );
            for issue in &result.issues {
                println!("     {}", issue.message.red());
            }
        }
    }

    println!();
    if report.has_issues() {
        println!("{}", "Issues found. Review the above.".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "All checks passed.".green().bold());
    }
}
