use std::{collections::HashMap, fs, path::Path};

use serde::Deserialize;
use serde_json::Value;

use crate::parser;

use super::{ToolValidateResult, ValidateIssue, ValidateIssueKind, ValidateReport};

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

#[derive(Debug, Deserialize)]
struct ToolDef {
    name: String,
    #[serde(alias = "parameters")]
    input_schema: Option<Value>,
}

pub fn run(tools_path: &str, path: &str) -> Result<ValidateReport, String> {
    let tools = load_tools(tools_path)?;
    let dtos = parser::dto::parse_project(path);
    let entities = parser::entity::parse_project(path);

    let tool_count = tools.len();
    let mut results = Vec::new();

    for tool in &tools {
        let schema_props = match extract_properties(&tool.input_schema) {
            Some(p) => p,
            None => {
                results.push(ToolValidateResult {
                    tool: tool.name.clone(),
                    matched_schema: None,
                    matched_file: None,
                    issues: vec![ValidateIssue {
                        tool: tool.name.clone(),
                        kind: ValidateIssueKind::NoSchema,
                        message: format!(
                            "Tool '{}' has no input_schema/parameters — skipping",
                            tool.name
                        ),
                    }],
                });
                continue;
            }
        };

        let matched_dto = find_matching_dto(&tool.name, &dtos);
        let matched_entity = find_matching_entity(&tool.name, &entities);

        match (matched_dto, matched_entity) {
            (None, None) => {
                results.push(ToolValidateResult {
                    tool: tool.name.clone(),
                    matched_schema: None,
                    matched_file: None,
                    issues: vec![ValidateIssue {
                        tool: tool.name.clone(),
                        kind: ValidateIssueKind::NoMatch,
                        message: format!(
                            "No matching DTO or Entity found for tool '{}'",
                            tool.name
                        ),
                    }],
                });
            }
            (maybe_dto, maybe_entity) => {
                let (schema_name, schema_file, codebase_fields) = if let Some(dto) = maybe_dto {
                    (
                        dto.name.clone(),
                        dto.file.clone(),
                        dto.fields
                            .iter()
                            .map(|f| (f.name.as_str(), f.field_type.as_str(), f.optional))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    let entity = maybe_entity.unwrap();
                    (
                        entity.name.clone(),
                        entity.file.clone(),
                        entity
                            .fields
                            .iter()
                            .map(|f| (f.name.as_str(), f.field_type.as_str(), f.optional))
                            .collect::<Vec<_>>(),
                    )
                };

                let issues = check_tool_against_schema(&tool.name, &schema_props, &codebase_fields);

                results.push(ToolValidateResult {
                    tool: tool.name.clone(),
                    matched_schema: Some(schema_name),
                    matched_file: Some(schema_file),
                    issues,
                });
            }
        }
    }

    Ok(ValidateReport { tool_count, results })
}

fn load_tools(path: &str) -> Result<Vec<ToolDef>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Could not read '{}': {}", path, e))?;

    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json");

    let file: ToolsFile = match ext {
        "yaml" | "yml" => serde_yaml::from_str(&content)
            .map_err(|e| format!("Invalid YAML: {}", e))?,
        _ => serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON: {}", e))?,
    };

    Ok(file.into_tools())
}

fn extract_properties(schema: &Option<Value>) -> Option<HashMap<String, String>> {
    let schema = schema.as_ref()?;
    let props = schema.get("properties")?;
    let obj = props.as_object()?;

    Some(
        obj.iter()
            .map(|(key, val)| {
                let json_type = val
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                (key.clone(), json_type)
            })
            .collect(),
    )
}

fn tool_name_to_base(tool_name: &str) -> String {
    let skip_prefixes = ["create", "update", "delete", "get", "find", "list", "fetch"];
    let parts: Vec<&str> = tool_name.split('_').collect();

    parts
        .iter()
        .filter(|p| !skip_prefixes.contains(*p))
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
    dtos.iter().find(|dto| dto.name.to_lowercase().contains(&base))
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

fn check_tool_against_schema(
    tool_name: &str,
    tool_props: &HashMap<String, String>,
    codebase_fields: &[(&str, &str, bool)],
) -> Vec<ValidateIssue> {
    let mut issues = Vec::new();

    for (prop_name, json_type) in tool_props {
        match codebase_fields.iter().find(|(name, _, _)| name == prop_name) {
            None => issues.push(ValidateIssue {
                tool: tool_name.to_string(),
                kind: ValidateIssueKind::PropertyMissing,
                message: format!(
                    "Tool '{}': property '{}' not found in codebase",
                    tool_name, prop_name
                ),
            }),
            Some((_, ts_type, _)) => {
                if !types_compatible(json_type, ts_type) {
                    issues.push(ValidateIssue {
                        tool: tool_name.to_string(),
                        kind: ValidateIssueKind::TypeMismatch,
                        message: format!(
                            "Tool '{}': property '{}' type mismatch — tool has '{}', codebase has '{}'",
                            tool_name, prop_name, json_type, ts_type
                        ),
                    });
                }
            }
        }
    }

    issues
}

fn types_compatible(json_type: &str, ts_type: &str) -> bool {
    let ts = ts_type.to_lowercase();
    match json_type {
        "string" => ts.contains("string"),
        "number" => ts.contains("number") || ts.contains("float") || ts.contains("decimal"),
        "integer" => ts.contains("number") || ts.contains("int"),
        "boolean" => ts.contains("boolean") || ts.contains("bool"),
        "array" => ts.contains("[]") || ts.contains("array"),
        "object" => ts.contains("object") || ts.contains("record") || ts == "any",
        _ => true,
    }
}
