pub mod check;
pub mod diff;
pub mod validate;

// --- Check ---

#[derive(Debug, Clone)]
pub enum CheckIssueKind {
    NoDto,
    FieldMissing,
    TypeMismatch,
}

#[derive(Debug, Clone)]
pub struct CheckIssue {
    pub kind: CheckIssueKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct EntityCheckResult {
    pub entity: String,
    pub dto: Option<String>,
    pub issues: Vec<CheckIssue>,
}

impl EntityCheckResult {
    pub fn is_ok(&self) -> bool {
        self.dto.is_some() && self.issues.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct CheckReport {
    pub entity_count: usize,
    pub dto_count: usize,
    pub results: Vec<EntityCheckResult>,
}

impl CheckReport {
    pub fn has_issues(&self) -> bool {
        self.results.iter().any(|r| !r.is_ok())
    }
}

// --- Diff ---

#[derive(Debug, Clone)]
pub enum FieldChangeKind {
    Added,
    Removed,
    TypeChanged,
    OptionalityChanged,
}

#[derive(Debug, Clone)]
pub struct FieldChange {
    pub kind: FieldChangeKind,
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SchemaChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone)]
pub enum SchemaType {
    Entity,
    Dto,
}

#[derive(Debug, Clone)]
pub struct SchemaChange {
    pub kind: SchemaChangeKind,
    pub schema_type: SchemaType,
    pub name: String,
    pub field_changes: Vec<FieldChange>,
}

#[derive(Debug, Clone)]
pub struct DiffReport {
    pub changes: Vec<SchemaChange>,
}

impl DiffReport {
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }
}

// --- Validate ---

#[derive(Debug, Clone)]
pub enum ValidateIssueKind {
    NoMatch,
    PropertyMissing,
    TypeMismatch,
    NoSchema,
}

#[derive(Debug, Clone)]
pub struct ValidateIssue {
    pub tool: String,
    pub kind: ValidateIssueKind,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ToolValidateResult {
    pub tool: String,
    pub matched_schema: Option<String>,
    pub matched_file: Option<String>,
    pub issues: Vec<ValidateIssue>,
}

impl ToolValidateResult {
    pub fn is_ok(&self) -> bool {
        self.matched_schema.is_some() && self.issues.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ValidateReport {
    pub tool_count: usize,
    pub results: Vec<ToolValidateResult>,
}

impl ValidateReport {
    pub fn has_issues(&self) -> bool {
        self.results.iter().any(|r| !r.is_ok())
    }
}
