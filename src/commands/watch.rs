use colored::Colorize;

use crate::core::check as core_check;
use crate::core::watch as core_watch;

pub fn run(path: &str) {
    println!("{}", "nest-drift watch".bold());
    println!("Watching: {}", path.dimmed());
    println!("{}\n", "Press Ctrl+C to stop.".dimmed());

    // Run once immediately
    print_report(path);

    let path_owned = path.to_string();

    let _handle = core_watch::watch(path, 300, move || {
        // Clear screen
        print!("\x1b[2J\x1b[H");

        let now = chrono::Local::now().format("%H:%M:%S");
        println!("{} {}\n", "nest-drift watch".bold(), format!("[{}]", now).dimmed());

        print_report(&path_owned);
    });

    // Block the main thread — Ctrl+C exits the process
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

fn print_report(path: &str) {
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
        let dto = result.dto.as_deref().unwrap_or("—");

        if result.is_ok() {
            println!(
                "{} {} <-> {}",
                "OK  ".green().bold(),
                result.entity.green(),
                dto.dimmed()
            );
        } else {
            println!(
                "{} {} <-> {}",
                "FAIL".red().bold(),
                result.entity.red(),
                dto.dimmed()
            );
            for issue in &result.issues {
                println!("     {}", issue.message.red());
            }
        }
    }

    println!();
    if report.has_issues() {
        println!("{}", "Issues found.".red().bold());
    } else {
        println!("{}", "All checks passed.".green().bold());
    }
}
