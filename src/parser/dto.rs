use regex::Regex;
use std::fs;
use walkdir::WalkDir;

use super::{DtoSchema, Field};


pub fn parse_project(root: &str) -> Vec<DtoSchema> {
    let re_dto_class = Regex::new(r"class\s+\w*[Dd][Tt][Oo]\w*").unwrap();
    let mut dtos = Vec::new();

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

        if !re_dto_class.is_match(&content) {
            continue;
        }

        let schemas = parse_dto_file(&content, path.to_string_lossy().to_string());
        dtos.extend(schemas);
    }

    dtos
}

fn parse_dto_file(content: &str, file: String) -> Vec<DtoSchema> {
    let re_class = Regex::new(r"class\s+(\w*[Dd][Tt][Oo]\w*)").unwrap();
    let re_decorator = Regex::new(r"@(\w+)\s*[\(\n]").unwrap();
    let re_property =
        Regex::new(r"^\s{2,}(\w+)(\?|!?):\s*([\w<>\[\]|&\s]+?)(?:\s*[;=]|$)").unwrap();

    let mut dtos = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        if let Some(cap) = re_class.captures(lines[i]) {
            let name = cap[1].to_string();
            let fields = extract_fields(&lines, i, &re_decorator, &re_property);
            dtos.push(DtoSchema {
                name,
                file: file.clone(),
                fields,
            });
        }
        i += 1;
    }

    dtos
}

fn extract_fields(
    lines: &[&str],
    class_line: usize,
    _re_decorator: &Regex,
    re_property: &Regex,
) -> Vec<Field> {
    let mut fields = Vec::new();
    let mut depth = 0i32;
    let mut in_class = false;

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

        if depth != 1 {
            continue;
        }

        // Skip pure decorator lines
        let is_decorator_only = line.trim_start().starts_with('@')
            && !re_property.is_match(line);
        if is_decorator_only {
            continue;
        }

        // Match property declaration
        if let Some(cap) = re_property.captures(line) {
            let name = cap[1].to_string();
            let optional = &cap[2] == "?";
            let field_type = cap[3].trim().to_string();

            fields.push(Field {
                name,
                field_type,
                optional,
            });
        }
    }
    fields
}
