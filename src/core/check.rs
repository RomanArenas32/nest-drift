use crate::parser::{self, DtoSchema, EntitySchema};

use super::{CheckIssue, CheckIssueKind, CheckReport, EntityCheckResult};

pub fn run(path: &str) -> CheckReport {
    let entities = parser::entity::parse_project(path);
    let dtos = parser::dto::parse_project(path);

    let entity_count = entities.len();
    let dto_count = dtos.len();
    let mut results = Vec::new();

    for entity in &entities {
        let related_dtos = find_related_dtos(entity, &dtos);

        if related_dtos.is_empty() {
            results.push(EntityCheckResult {
                entity: entity.name.clone(),
                dto: None,
                issues: vec![CheckIssue {
                    kind: CheckIssueKind::NoDto,
                    message: format!("No matching DTO found for entity '{}'", entity.name),
                }],
            });
            continue;
        }

        for dto in related_dtos {
            let issues = compare_fields(entity, dto);
            results.push(EntityCheckResult {
                entity: entity.name.clone(),
                dto: Some(dto.name.clone()),
                issues,
            });
        }
    }

    CheckReport {
        entity_count,
        dto_count,
        results,
    }
}

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

fn compare_fields(entity: &EntitySchema, dto: &DtoSchema) -> Vec<CheckIssue> {
    let mut issues = Vec::new();

    for entity_field in &entity.fields {
        match dto.fields.iter().find(|f| f.name == entity_field.name) {
            None => issues.push(CheckIssue {
                kind: CheckIssueKind::FieldMissing,
                message: format!(
                    "Field '{}' exists in {} but is missing from {}",
                    entity_field.name, entity.name, dto.name
                ),
            }),
            Some(df) => {
                let entity_type = normalize_type(&entity_field.field_type);
                let dto_type = normalize_type(&df.field_type);

                if entity_type != dto_type {
                    issues.push(CheckIssue {
                        kind: CheckIssueKind::TypeMismatch,
                        message: format!(
                            "Field '{}': type mismatch — entity has '{}', {} has '{}'",
                            entity_field.name, entity_field.field_type, dto.name, df.field_type
                        ),
                    });
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
