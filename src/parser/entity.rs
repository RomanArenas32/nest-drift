use regex::Regex;
use std::fs;
use walkdir::WalkDir;

use super::{EntitySchema, Field};

// Decorators that mark a property as a database column
const COLUMN_DECORATORS: &[&str] = &[
    "Column",
    "PrimaryColumn",
    "PrimaryGeneratedColumn",
    "CreateDateColumn",
    "UpdateDateColumn",
    "DeleteDateColumn",
    "VersionColumn",
    "OneToOne",
    "ManyToOne",
    "OneToMany",
    "ManyToMany",
];

pub fn parse_project(root: &str) -> Vec<EntitySchema> {
    let re_entity_class = Regex::new(r"@Entity\b").unwrap();
    let mut entities = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map(|ext| ext == "ts").unwrap_or(false)
                && !e.path().components().any(|c| {
                    matches!(c.as_os_str().to_str(), Some("node_modules") | Some("dist"))
                })
        })
    {
        let path = entry.path();
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !re_entity_class.is_match(&content) {
            continue;
        }

        let schemas = parse_entity_file(&content, path.to_string_lossy().to_string());
        entities.extend(schemas);
    }

    entities
}

fn parse_entity_file(content: &str, file: String) -> Vec<EntitySchema> {
    let re_class = Regex::new(r"class\s+(\w+)").unwrap();
    let re_decorator = Regex::new(r"@(\w+)\s*[\(\n]").unwrap();
    let re_property =
        Regex::new(r"^\s{2,}(\w+)(\?|!?):\s*([\w<>\[\]|&\s]+?)(?:\s*[;=]|$)").unwrap();

    let column_set: std::collections::HashSet<&str> =
        COLUMN_DECORATORS.iter().copied().collect();

    let mut entities = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Look for @Entity decorator
        if lines[i].contains("@Entity") {
            // Find the class declaration after @Entity
            let class_start = find_class_line(&lines, i);
            if let Some(class_line_idx) = class_start {
                let class_name = re_class
                    .captures(lines[class_line_idx])
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string());

                if let Some(name) = class_name {
                    let fields =
                        extract_fields(&lines, class_line_idx, &re_decorator, &re_property, &column_set);
                    entities.push(EntitySchema {
                        name,
                        file: file.clone(),
                        fields,
                    });
                    i = class_line_idx;
                }
            }
        }
        i += 1;
    }

    entities
}

fn find_class_line(lines: &[&str], from: usize) -> Option<usize> {
    for i in from..std::cmp::min(from + 10, lines.len()) {
        if lines[i].contains("class ") {
            return Some(i);
        }
    }
    None
}

fn extract_fields(
    lines: &[&str],
    class_line: usize,
    re_decorator: &Regex,
    re_property: &Regex,
    column_set: &std::collections::HashSet<&str>,
) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut depth = 0;
    let mut in_class = false;
    let mut pending_column = false;

    for line in &lines[class_line..] {
        depth += line.chars().filter(|&c| c == '{').count() as i32;
        depth -= line.chars().filter(|&c| c == '}').count() as i32;

        if !in_class && depth > 0 {
            in_class = true;
            continue;
        }

        if depth <= 0 && in_class {
            break;
        }

        // Only look at class body (depth == 1 means directly inside the class)
        if depth != 1 {
            continue;
        }

        // Check for column decorators
        for cap in re_decorator.captures_iter(line) {
            if let Some(dec_name) = cap.get(1) {
                if column_set.contains(dec_name.as_str()) {
                    pending_column = true;
                }
            }
        }

        // Try to match a property declaration
        if let Some(cap) = re_property.captures(line) {
            if pending_column {
                let name = cap[1].to_string();
                let optional = &cap[2] == "?";
                let field_type = cap[3].trim().to_string();

                fields.push(Field {
                    name,
                    field_type,
                    optional,
                });
                pending_column = false;
            }
        }
    }

    fields
}
