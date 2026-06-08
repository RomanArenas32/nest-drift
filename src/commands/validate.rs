use colored::Colorize;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path};

use crate::parser;

// ── Tool definition structs ──────────────────────────────────────────────────

/// Top-level file format: either `{ "tools": [...] }` or directly `[...]`
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ToolsFile {
    Wrapped { tools: Vec<ToolDef> },
    Array(Vec<ToolDef>),
}

impl ToolsFile {
    fn into_tools(self) -> Vec<ToolDef> {
        match self {
            ToolsFile::Wrapped { tools } => tools,
            ToolsFile::Array(tools) => tools,
        }
    }
}

/// A single LLM tool definition.
/// Supports both Anthropic (`input_schema`) and OpenAI (`parameters`) formats.
#[derive(Debug, Deserialize)]
struct ToolDef {
    name: String,
    #[serde(alias = "parameters")]
    input_schema: Option<Value>,
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn run(tools_path: &str, path: &str) {
    println!("{}", "nest-drift validate".bold());
    println!("Tools:   {}", tools_path.dimmed());
    println!("Project: {}\n", path.dimmed());

    let tools = load_tools(tools_path);
    let dtos = parser::dto::parse_project(path);
    let entities = parser::entity::parse_project(path);

    if tools.is_empty() {
        println!("{}", "No tools found in file.".yellow());
        return;
    }

    println!(
        "Found {} tools, {} DTOs, {} entities\n",
        tools.len().to_string().cyan(),
        dtos.len().to_string().cyan(),
        entities.len().to_string().cyan(),
    );

    let mut issues_found = false;

    for tool in &tools {
        let schema_props = match extract_properties(&tool.input_schema) {
            Some(p) => p,
            None => {
                println!(
                    "{} {} — no input_schema/parameters found, skipping",
                    "SKIP".yellow().bold(),
                    tool.name.yellow()
                );
                continue;
            }
        };

        // Try to find a matching DTO or entity by tool name
        let matched_dto = find_matching_dto(&tool.name, &dtos);
        let matched_entity = find_matching_entity(&tool.name, &entities);

        match (matched_dto, matched_entity) {
            (None, None) => {
                println!(
                    "{} {} — no matching DTO or Entity found in codebase",
                    "WARN".yellow().bold(),
                    tool.name.yellow()
                );
                issues_found = true;
            }
            (maybe_dto, maybe_entity) => {
                // Use DTO preferentially, fall back to entity
                let (schema_name, schema_file, codebase_fields) = if let Some(dto) = maybe_dto {
                    (
                        dto.name.as_str(),
                        dto.file.as_str(),
                        dto.fields
                            .iter()
                            .map(|f| (f.name.as_str(), f.field_type.as_str(), f.optional))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    let entity = maybe_entity.unwrap();
                    (
                        entity.name.as_str(),
                        entity.file.as_str(),
                        entity
                            .fields
                            .iter()
                            .map(|f| (f.name.as_str(), f.field_type.as_str(), f.optional))
                            .collect::<Vec<_>>(),
                    )
                };

                let issues = check_tool_against_schema(&tool.name, &schema_props, &codebase_fields);

                if issues.is_empty() {
                    println!(
                        "{} {} -> {} ({})",
                        "OK  ".green().bold(),
                        tool.name.green(),
                        schema_name.green(),
                        schema_file.dimmed()
                    );
                } else {
                    println!(
                        "{} {} -> {}",
                        "FAIL".red().bold(),
                        tool.name.red(),
                        schema_name.red()
                    );
                    for issue in &issues {
                        println!("     {}", issue.red());
                    }
                    issues_found = true;
                }
            }
        }
    }

    println!();
    if issues_found {
        println!("{}", "Validation failed. Tool definitions are out of sync.".red().bold());
        std::process::exit(1);
    } else {
        println!("{}", "All tools are valid.".green().bold());
    }
}

// ── Loaders ──────────────────────────────────────────────────────────────────

fn load_tools(path: &str) -> Vec<ToolDef> {
    let content = fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("{} Could not read '{}': {}", "ERROR".red().bold(), path, e);
        std::process::exit(1);
    });

    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json");

    let file: ToolsFile = match ext {
        "yaml" | "yml" => serde_yaml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("{} Invalid YAML: {}", "ERROR".red().bold(), e);
            std::process::exit(1);
        }),
        _ => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("{} Invalid JSON: {}", "ERROR".red().bold(), e);
            std::process::exit(1);
        }),
    };

    file.into_tools()
}

// ── Schema helpers ────────────────────────────────────────────────────────────

/// Extracts `properties` map from a JSON Schema object.
fn extract_properties(schema: &Option<Value>) -> Option<HashMap<String, String>> {
    let schema = schema.as_ref()?;
    let props = schema.get("properties")?;
    let obj = props.as_object()?;

    let map = obj
        .iter()
        .map(|(key, val)| {
            let json_type = val
                .get("type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown")
                .to_string();
            (key.clone(), json_type)
        })
        .collect();

    Some(map)
}

// ── Matching logic ────────────────────────────────────────────────────────────

/// Converts a snake_case tool name to a PascalCase base for matching.
/// e.g. `create_user` -> `User`, `get_product_by_id` -> `Product`
fn tool_name_to_base(tool_name: &str) -> String {
    let skip_prefixes = ["create", "update", "delete", "get", "find", "list", "fetch"];
    let parts: Vec<&str> = tool_name.split('_').collect();

    let meaningful: Vec<&str> = parts
        .iter()
        .filter(|p| !skip_prefixes.contains(*p))
        .copied()
        .collect();

    meaningful
        .iter()
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<String>()
        .to_lowercase()
}

fn find_matching_dto<'a>(
    tool_name: &str,
    dtos: &'a [parser::DtoSchema],
) -> Option<&'a parser::DtoSchema> {
    let base = tool_name_to_base(tool_name);
    dtos.iter()
        .find(|dto| dto.name.to_lowercase().contains(&base))
}

fn find_matching_entity<'a>(
    tool_name: &str,
    entities: &'a [parser::EntitySchema],
) -> Option<&'a parser::EntitySchema> {
    let base = tool_name_to_base(tool_name);
    entities.iter().find(|e| {
        e.name
            .trim_end_matches("Entity")
            .to_lowercase()
            .contains(&base)
    })
}

// ── Comparison ────────────────────────────────────────────────────────────────

fn check_tool_against_schema(
    tool_name: &str,
    tool_props: &HashMap<String, String>,
    codebase_fields: &[(&str, &str, bool)],
) -> Vec<String> {
    let mut issues = Vec::new();

    for (prop_name, json_type) in tool_props {
        match codebase_fields.iter().find(|(name, _, _)| name == prop_name) {
            None => issues.push(format!(
                "Tool '{}': property '{}' not found in codebase",
                tool_name, prop_name
            )),
            Some((_, ts_type, _)) => {
                if !types_compatible(json_type, ts_type) {
                    issues.push(format!(
                        "Tool '{}': property '{}' type mismatch — tool has '{}', codebase has '{}'",
                        tool_name, prop_name, json_type, ts_type
                    ));
                }
            }
        }
    }

    issues
}

/// Checks if a JSON Schema type is compatible with a TypeScript type.
fn types_compatible(json_type: &str, ts_type: &str) -> bool {
    let ts = ts_type.to_lowercase();
    match json_type {
        "string" => ts.contains("string"),
        "number" => ts.contains("number") || ts.contains("float") || ts.contains("decimal"),
        "integer" => ts.contains("number") || ts.contains("int"),
        "boolean" => ts.contains("boolean") || ts.contains("bool"),
        "array" => ts.contains("[]") || ts.contains("array"),
        "object" => ts.contains("object") || ts.contains("record") || ts == "any",
        _ => true, // unknown types pass through
    }
}
