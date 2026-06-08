use colored::Colorize;

use crate::parser::{self, EntitySchema, DtoSchema};

pub fn run(path: &str) {
    println!("{}", "nest-drift check".bold());
    println!("Scanning: {}\n", path.dimmed());

    let entities = parser::entity::parse_project(path);
    let dtos = parser::dto::parse_project(path);

    if entities.is_empty() {
        println!("{}", "No @Entity classes found.".yellow());
        return;
    }

    println!(
        "Found {} entities, {} DTOs\n",
        entities.len().to_string().cyan(),
        dtos.len().to_string().cyan()
    );

    let mut issues_found = false;

    for entity in &entities {
        let related_dtos = find_related_dtos(entity, &dtos);

        if related_dtos.is_empty() {
            println!(
                "{} {} — no matching DTO found",
                "WARN".yellow().bold(),
                entity.name.yellow()
            );
            issues_found = true;
            continue;
        }

        for dto in related_dtos {
            let issues = compare(entity, dto);
            if issues.is_empty() {
                println!(
                    "{} {} <-> {}",
                    "OK  ".green().bold(),
                    entity.name.green(),
                    dto.name.dimmed()
                );
            } else {
                println!(
                    "{} {} <-> {}",
                    "FAIL".red().bold(),
                    entity.name.red(),
                    dto.name.dimmed()
                );
                for issue in &issues {
                    println!("     {}", issue.0.red());
                }
                issues_found = true;
            }
        }
    }

    println!();
    if issues_found {
        println!("{}", "Issues found. Review the above.".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "All checks passed.".green().bold());
    }
}

/// Find DTOs whose name contains the entity name (e.g. UserEntity -> CreateUserDto, UpdateUserDto)
fn find_related_dtos<'a>(entity: &EntitySchema, dtos: &'a [DtoSchema]) -> Vec<&'a DtoSchema> {
    let base = entity
        .name
        .trim_end_matches("Entity")
        .trim_end_matches("Model")
        .to_lowercase();

    dtos.iter()
        .filter(|dto| dto.name.to_lowercase().contains(&base))
        .collect()
}

#[derive(Debug)]
struct Issue(String);

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn compare(entity: &EntitySchema, dto: &DtoSchema) -> Vec<Issue> {
    let mut issues = Vec::new();

    for entity_field in &entity.fields {
        let dto_field = dto.fields.iter().find(|f| f.name == entity_field.name);

        match dto_field {
            None => {
                issues.push(Issue(format!(
                    "Field '{}' exists in {} but is missing from {}",
                    entity_field.name, entity.name, dto.name
                )));
            }
            Some(df) => {
                // Normalize types for comparison (strip nullability hints)
                let entity_type = normalize_type(&entity_field.field_type);
                let dto_type = normalize_type(&df.field_type);

                if entity_type != dto_type {
                    issues.push(Issue(format!(
                        "Field '{}': type mismatch — entity has '{}', {} has '{}'",
                        entity_field.name, entity_field.field_type, dto.name, df.field_type
                    )));
                }
            }
        }
    }

    issues
}

fn normalize_type(t: &str) -> String {
    t.replace("| null", "")
        .replace("| undefined", "")
        .replace("Nullable<", "")
        .replace('>', "")
        .trim()
        .to_lowercase()
}
