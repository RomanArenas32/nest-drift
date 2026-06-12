#![allow(clippy::needless_pass_by_value)]

mod core;
mod parser;

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex, OnceLock,
};

use napi::threadsafe_function::{ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode};
use napi_derive::napi;

// ── Watcher registry ──────────────────────────────────────────────────────────

fn watchers() -> &'static Mutex<HashMap<u32, Arc<AtomicBool>>> {
    static WATCHERS: OnceLock<Mutex<HashMap<u32, Arc<AtomicBool>>>> = OnceLock::new();
    WATCHERS.get_or_init(|| Mutex::new(HashMap::new()))
}

static NEXT_WATCH_ID: AtomicU32 = AtomicU32::new(1);

// ── Check ─────────────────────────────────────────────────────────────────────

#[napi(object)]
#[derive(serde::Serialize)]
pub struct CheckIssue {
    /// "no_dto" | "field_missing" | "type_mismatch"
    pub kind: String,
    pub message: String,
}

#[napi(object)]
#[derive(serde::Serialize)]
pub struct EntityCheckResult {
    pub entity: String,
    pub dto: Option<String>,
    pub ok: bool,
    pub issues: Vec<CheckIssue>,
}

#[napi(object)]
#[derive(serde::Serialize)]
pub struct CheckReport {
    pub entity_count: u32,
    pub dto_count: u32,
    pub has_issues: bool,
    pub results: Vec<EntityCheckResult>,
}

#[napi]
pub fn check(path: String) -> CheckReport {
    let report = core::check::run(&path);

    CheckReport {
        entity_count: report.entity_count as u32,
        dto_count: report.dto_count as u32,
        has_issues: report.has_issues(),
        results: report
            .results
            .into_iter()
            .map(|r| {
                let ok = r.is_ok();
                EntityCheckResult {
                    entity: r.entity,
                    dto: r.dto,
                    ok,
                    issues: r
                        .issues
                        .into_iter()
                        .map(|i| CheckIssue {
                            kind: match i.kind {
                                core::CheckIssueKind::NoDto => "no_dto".to_string(),
                                core::CheckIssueKind::FieldMissing => "field_missing".to_string(),
                                core::CheckIssueKind::TypeMismatch => "type_mismatch".to_string(),
                            },
                            message: i.message,
                        })
                        .collect(),
                }
            })
            .collect(),
    }
}

// ── Snapshot ──────────────────────────────────────────────────────────────────

#[napi(object)]
pub struct Field {
    pub name: String,
    pub field_type: String,
    pub optional: bool,
}

#[napi(object)]
pub struct EntitySchema {
    pub name: String,
    pub file: String,
    pub fields: Vec<Field>,
}

#[napi(object)]
pub struct DtoSchema {
    pub name: String,
    pub file: String,
    pub fields: Vec<Field>,
}

#[napi(object)]
pub struct ProjectSnapshot {
    pub entities: Vec<EntitySchema>,
    pub dtos: Vec<DtoSchema>,
}

#[napi]
pub fn snapshot(path: String) -> ProjectSnapshot {
    let entities = parser::entity::parse_project(&path);
    let dtos = parser::dto::parse_project(&path);

    ProjectSnapshot {
        entities: entities
            .into_iter()
            .map(|e| EntitySchema {
                name: e.name,
                file: e.file,
                fields: e
                    .fields
                    .into_iter()
                    .map(|f| Field {
                        name: f.name,
                        field_type: f.field_type,
                        optional: f.optional,
                    })
                    .collect(),
            })
            .collect(),
        dtos: dtos
            .into_iter()
            .map(|d| DtoSchema {
                name: d.name,
                file: d.file,
                fields: d
                    .fields
                    .into_iter()
                    .map(|f| Field {
                        name: f.name,
                        field_type: f.field_type,
                        optional: f.optional,
                    })
                    .collect(),
            })
            .collect(),
    }
}

// ── Diff ──────────────────────────────────────────────────────────────────────

#[napi(object)]
pub struct FieldChange {
    /// "added" | "removed" | "type_changed" | "optionality_changed"
    pub kind: String,
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[napi(object)]
pub struct SchemaChange {
    /// "added" | "removed" | "modified"
    pub kind: String,
    /// "entity" | "dto"
    pub schema_type: String,
    pub name: String,
    pub field_changes: Vec<FieldChange>,
}

#[napi(object)]
pub struct DiffReport {
    pub has_changes: bool,
    pub changes: Vec<SchemaChange>,
}

#[napi]
pub fn diff(snapshot_path: String, path: String) -> napi::Result<DiffReport> {
    let report = core::diff::run(&snapshot_path, &path)
        .map_err(|e| napi::Error::from_reason(e))?;

    Ok(DiffReport {
        has_changes: report.has_changes(),
        changes: report
            .changes
            .into_iter()
            .map(|c| SchemaChange {
                kind: match c.kind {
                    core::SchemaChangeKind::Added => "added".to_string(),
                    core::SchemaChangeKind::Removed => "removed".to_string(),
                    core::SchemaChangeKind::Modified => "modified".to_string(),
                },
                schema_type: match c.schema_type {
                    core::SchemaType::Entity => "entity".to_string(),
                    core::SchemaType::Dto => "dto".to_string(),
                },
                name: c.name,
                field_changes: c
                    .field_changes
                    .into_iter()
                    .map(|fc| FieldChange {
                        kind: match fc.kind {
                            core::FieldChangeKind::Added => "added".to_string(),
                            core::FieldChangeKind::Removed => "removed".to_string(),
                            core::FieldChangeKind::TypeChanged => "type_changed".to_string(),
                            core::FieldChangeKind::OptionalityChanged => {
                                "optionality_changed".to_string()
                            }
                        },
                        field: fc.field,
                        before: fc.before,
                        after: fc.after,
                    })
                    .collect(),
            })
            .collect(),
    })
}

// ── Validate ──────────────────────────────────────────────────────────────────

#[napi(object)]
pub struct ValidateIssue {
    /// "no_match" | "property_missing" | "type_mismatch" | "no_schema"
    pub kind: String,
    pub message: String,
}

#[napi(object)]
pub struct ToolValidateResult {
    pub tool: String,
    pub matched_schema: Option<String>,
    pub matched_file: Option<String>,
    pub ok: bool,
    pub issues: Vec<ValidateIssue>,
}

#[napi(object)]
pub struct ValidateReport {
    pub tool_count: u32,
    pub has_issues: bool,
    pub results: Vec<ToolValidateResult>,
}

#[napi]
pub fn validate(tools_path: String, path: String) -> napi::Result<ValidateReport> {
    let report = core::validate::run(&tools_path, &path)
        .map_err(|e| napi::Error::from_reason(e))?;

    Ok(ValidateReport {
        tool_count: report.tool_count as u32,
        has_issues: report.has_issues(),
        results: report
            .results
            .into_iter()
            .map(|r| {
                let ok = r.is_ok();
                ToolValidateResult {
                    tool: r.tool,
                    matched_schema: r.matched_schema,
                    matched_file: r.matched_file,
                    ok,
                    issues: r
                        .issues
                        .into_iter()
                        .map(|i| ValidateIssue {
                            kind: match i.kind {
                                core::ValidateIssueKind::NoMatch => "no_match".to_string(),
                                core::ValidateIssueKind::PropertyMissing => {
                                    "property_missing".to_string()
                                }
                                core::ValidateIssueKind::TypeMismatch => {
                                    "type_mismatch".to_string()
                                }
                                core::ValidateIssueKind::NoSchema => "no_schema".to_string(),
                            },
                            message: i.message,
                        })
                        .collect(),
                }
            })
            .collect(),
    })
}

// ── Watch ─────────────────────────────────────────────────────────────────────

/// Start watching `path` for `.ts` file changes.
/// `callback` receives a JSON-serialized `CheckReport` on every change.
/// Returns a watch ID — pass it to `watchStop` to stop watching.
#[napi]
pub fn watch_start(path: String, callback: napi::JsFunction) -> napi::Result<u32> {
    let tsfn: ThreadsafeFunction<String, ErrorStrategy::Fatal> =
        callback.create_threadsafe_function(0, |ctx: ThreadSafeCallContext<String>| {
            ctx.env
                .create_string(&ctx.value)
                .map(|s| vec![s.into_unknown()])
        })?;

    let path_clone = path.clone();
    let handle = core::watch::watch(&path, 300, move || {
        let report = core::check::run(&path_clone);

        let napi_report = CheckReport {
            entity_count: report.entity_count as u32,
            dto_count: report.dto_count as u32,
            has_issues: report.has_issues(),
            results: report
                .results
                .into_iter()
                .map(|r| {
                    let ok = r.is_ok();
                    EntityCheckResult {
                        entity: r.entity,
                        dto: r.dto,
                        ok,
                        issues: r
                            .issues
                            .into_iter()
                            .map(|i| CheckIssue {
                                kind: match i.kind {
                                    core::CheckIssueKind::NoDto => "no_dto".to_string(),
                                    core::CheckIssueKind::FieldMissing => {
                                        "field_missing".to_string()
                                    }
                                    core::CheckIssueKind::TypeMismatch => {
                                        "type_mismatch".to_string()
                                    }
                                },
                                message: i.message,
                            })
                            .collect(),
                    }
                })
                .collect(),
        };

        if let Ok(json) = serde_json::to_string(&napi_report) {
            tsfn.call(json, ThreadsafeFunctionCallMode::NonBlocking);
        }
    });

    let id = NEXT_WATCH_ID.fetch_add(1, Ordering::Relaxed);
    watchers()
        .lock()
        .unwrap()
        .insert(id, handle.into_stop_flag());

    Ok(id)
}

/// Stop a running watcher by its ID.
#[napi]
pub fn watch_stop(id: u32) {
    if let Some(flag) = watchers().lock().unwrap().remove(&id) {
        flag.store(true, Ordering::Relaxed);
    }
}
