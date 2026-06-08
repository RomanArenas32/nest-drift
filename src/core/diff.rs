use std::fs;

use crate::parser::{self, Field, ProjectSchema};

use super::{
    DiffReport, FieldChange, FieldChangeKind, SchemaChange, SchemaChangeKind, SchemaType,
};

pub fn run(snapshot_path: &str, path: &str) -> Result<DiffReport, String> {
    let snapshot = load_snapshot(snapshot_path)?;

    let current = ProjectSchema {
        entities: parser::entity::parse_project(path),
        dtos: parser::dto::parse_project(path),
    };

    let mut changes = Vec::new();

    changes.extend(diff_schemas(
        SchemaType::Entity,
        &snapshot.entities.iter().map(|e| (&e.name, &e.fields)).collect::<Vec<_>>(),
        &current.entities.iter().map(|e| (&e.name, &e.fields)).collect::<Vec<_>>(),
    ));

    changes.extend(diff_schemas(
        SchemaType::Dto,
        &snapshot.dtos.iter().map(|d| (&d.name, &d.fields)).collect::<Vec<_>>(),
        &current.dtos.iter().map(|d| (&d.name, &d.fields)).collect::<Vec<_>>(),
    ));

    Ok(DiffReport { changes })
}

fn load_snapshot(path: &str) -> Result<ProjectSchema, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Could not read snapshot '{}': {}", path, e))?;

    serde_json::from_str(&content).map_err(|e| format!("Invalid snapshot file: {}", e))
}

fn diff_schemas(
    schema_type: SchemaType,
    before: &[(&String, &Vec<Field>)],
    after: &[(&String, &Vec<Field>)],
) -> Vec<SchemaChange> {
    let mut changes = Vec::new();

    for (name, _) in after {
        if !before.iter().any(|(n, _)| n == name) {
            changes.push(SchemaChange {
                kind: SchemaChangeKind::Added,
                schema_type: schema_type.clone(),
                name: name.to_string(),
                field_changes: vec![],
            });
        }
    }

    for (name, _) in before {
        if !after.iter().any(|(n, _)| n == name) {
            changes.push(SchemaChange {
                kind: SchemaChangeKind::Removed,
                schema_type: schema_type.clone(),
                name: name.to_string(),
                field_changes: vec![],
            });
        }
    }

    for (name, before_fields) in before {
        if let Some((_, after_fields)) = after.iter().find(|(n, _)| n == name) {
            let field_changes = diff_fields(before_fields, after_fields);
            if !field_changes.is_empty() {
                changes.push(SchemaChange {
                    kind: SchemaChangeKind::Modified,
                    schema_type: schema_type.clone(),
                    name: name.to_string(),
                    field_changes,
                });
            }
        }
    }

    changes
}

fn diff_fields(before: &[Field], after: &[Field]) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    for f in after {
        match before.iter().find(|b| b.name == f.name) {
            None => changes.push(FieldChange {
                kind: FieldChangeKind::Added,
                field: f.name.clone(),
                before: None,
                after: Some(f.field_type.clone()),
            }),
            Some(b) => {
                if b.field_type != f.field_type {
                    changes.push(FieldChange {
                        kind: FieldChangeKind::TypeChanged,
                        field: f.name.clone(),
                        before: Some(b.field_type.clone()),
                        after: Some(f.field_type.clone()),
                    });
                }
                if b.optional != f.optional {
                    changes.push(FieldChange {
                        kind: FieldChangeKind::OptionalityChanged,
                        field: f.name.clone(),
                        before: Some(if b.optional { "optional" } else { "required" }.to_string()),
                        after: Some(if f.optional { "optional" } else { "required" }.to_string()),
                    });
                }
            }
        }
    }

    for f in before {
        if !after.iter().any(|a| a.name == f.name) {
            changes.push(FieldChange {
                kind: FieldChangeKind::Removed,
                field: f.name.clone(),
                before: Some(f.field_type.clone()),
                after: None,
            });
        }
    }

    changes
}
