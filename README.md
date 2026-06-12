# nest-drift

Detects schema drift in NestJS projects. Validates consistency between TypeORM entities and DTOs, snapshots your schema, diffs changes over time, and cross-checks LLM tool definitions against your actual codebase.

Available as both a **CLI tool** and a **Node.js library**.

---

## The problem

Someone renames a field in a TypeORM entity. The DTO doesn't get updated. The LLM tool definition still references the old field name. Everything breaks silently — in production, or worse, inside an AI agent's tool call.

`nest-drift` catches this at the source.

---

## Installation

```bash
npm install -D nest-drift
```

---

## Usage: CLI

Run commands directly from the terminal or npm scripts. Ideal for CI pipelines and pre-commit hooks.

### `check` — validate entities vs DTOs

Scans for `@Entity` classes and their related DTOs, then reports missing fields and type mismatches.

```bash
npx nest-drift check .
npx nest-drift check ./my-nestjs-project
```

```
nest-drift check
Scanning: .

Found 3 entities, 7 DTOs

OK   User <-> CreateUserDto
FAIL User <-> UpdateUserDto
     Field 'email': type mismatch — entity has 'string', UpdateUserDto has 'number'
     Field 'role' exists in User but is missing from UpdateUserDto

OK   Product <-> CreateProductDto

Issues found. Review the above.
```

Exits with code `1` if issues are found.

---

### `snapshot` — capture the current schema

Serializes all entities and DTOs to a JSON file you can commit as a baseline.

```bash
npx nest-drift snapshot .
npx nest-drift snapshot . --output custom-snap.json
```

---

### `diff` — compare against a snapshot

Compares the current state of your project against a previously generated snapshot.

```bash
npx nest-drift diff nest-drift.snapshot.json .
```

```
Entities
  ~ Entity: User
    ~ field 'email': string -> number (type changed)
    - field 'name': string (removed)
    + field 'phone': string (added)

DTOs
  + DTO: CreateUserDto (new)
  - DTO: UpdateUserDto (removed)

Schema has changed since last snapshot.
```

Exits with code `1` if changes are detected.

---

### `validate` — cross-check LLM tool definitions

Reads your LLM tool definitions (JSON or YAML) and verifies that every property maps to a real field in your codebase with a compatible type.

```bash
npx nest-drift validate tools.json .
npx nest-drift validate tools.yaml .
```

Supports both **Anthropic** (`input_schema`) and **OpenAI** (`parameters`) tool formats.

```json
{
  "tools": [
    {
      "name": "create_user",
      "input_schema": {
        "type": "object",
        "properties": {
          "email": { "type": "string" },
          "age":   { "type": "integer" }
        }
      }
    }
  ]
}
```

```
Found 2 tools

OK   create_user -> CreateUserDto (src/user/user.dto.ts)
FAIL update_user -> UpdateUserDto
     Tool 'update_user': property 'username' not found in codebase

Validation failed. Tool definitions are out of sync.
```

---

### `watch` — re-run check on file changes

Watches the project for `.ts` file changes and re-runs `check` automatically. Clears the terminal and shows a fresh report on every save, with a 300ms debounce to avoid noise on rapid edits.

```bash
npx nest-drift watch .
```

```
nest-drift watch [10:42:31]

Found 3 entities, 7 DTOs

OK   User <-> CreateUserDto
FAIL User <-> UpdateUserDto
     Field 'role' exists in User but is missing from UpdateUserDto

OK   Product <-> CreateProductDto

Issues found.
```

Press `Ctrl+C` to stop.

---

### CLI in package.json scripts

```json
{
  "scripts": {
    "drift:check":    "nest-drift check .",
    "drift:snapshot": "nest-drift snapshot .",
    "drift:diff":     "nest-drift diff nest-drift.snapshot.json .",
    "drift:watch":    "nest-drift watch ."
  }
}
```

### Pre-commit hook (Husky)

```bash
npx husky add .husky/pre-commit "nest-drift check ."
```

### CI pipeline (GitHub Actions)

```yaml
- name: Check schema drift
  run: npx nest-drift diff nest-drift.snapshot.json .
```

---

## Usage: Library

Import the functions directly in your TypeScript/JavaScript code and work with the results programmatically. Ideal when you need custom reporting, alerting, or integration with other tools.

### Installation

```bash
npm install nest-drift
```

### `check(path)`

Returns an object describing the consistency state between entities and DTOs.

```ts
import { check } from 'nest-drift'

const report = check('./src')

console.log(`${report.entityCount} entities, ${report.dtoCount} DTOs`)

if (report.hasIssues) {
  for (const result of report.results) {
    if (!result.ok) {
      console.log(`[${result.entity}] → ${result.dto ?? 'no DTO found'}`)
      for (const issue of result.issues) {
        // issue.kind: "no_dto" | "field_missing" | "type_mismatch"
        console.log(`  ${issue.kind}: ${issue.message}`)
      }
    }
  }
}
```

---

### `snapshot(path)`

Returns the parsed schema as a plain object — no file is written.

```ts
import { snapshot } from 'nest-drift'

const schema = snapshot('./src')

console.log(schema.entities) // EntitySchema[]
console.log(schema.dtos)     // DtoSchema[]

// Each schema has: { name, file, fields: [{ name, fieldType, optional }] }
```

If you want to save it to disk:

```ts
import { snapshot } from 'nest-drift'
import { writeFileSync } from 'fs'

const schema = snapshot('./src')
writeFileSync('nest-drift.snapshot.json', JSON.stringify(schema, null, 2))
```

---

### `diff(snapshotPath, path)`

Compares a snapshot file against the current state of the project.

```ts
import { diff } from 'nest-drift'

const report = diff('nest-drift.snapshot.json', './src')

if (report.hasChanges) {
  for (const change of report.changes) {
    // change.kind: "added" | "removed" | "modified"
    // change.schemaType: "entity" | "dto"
    console.log(`${change.kind} ${change.schemaType}: ${change.name}`)

    for (const fc of change.fieldChanges) {
      // fc.kind: "added" | "removed" | "type_changed" | "optionality_changed"
      console.log(`  ${fc.kind}: ${fc.field} (${fc.before} → ${fc.after})`)
    }
  }

  // Custom alerting
  await sendSlackAlert(report.changes)
}
```

Throws if the snapshot file doesn't exist or is invalid.

---

### `watch(path, callback)`

Watches the project for `.ts` file changes and calls `callback` with a fresh `CheckReport` on every change (debounced 300ms). Returns a stop function.

```ts
import { watch } from 'nest-drift'

const stop = watch('./src', (report) => {
  if (report.hasIssues) {
    for (const result of report.results.filter(r => !r.ok)) {
      console.log(`[${result.entity}] has issues`)
    }

    // Custom alerting — Slack, webhook, whatever you need
    await sendSlackAlert(report.results)
  }
})

// Stop watching when done
process.on('SIGINT', stop)
```

Keeps the process alive while watching. Call `stop()` to shut down the watcher and release resources.

---

### `validate(toolsPath, path)`

Validates LLM tool definitions against the codebase.

```ts
import { validate } from 'nest-drift'

const report = validate('./tools.json', './src')

for (const result of report.results) {
  if (result.ok) {
    console.log(`✓ ${result.tool} → ${result.matchedSchema}`)
  } else {
    console.log(`✗ ${result.tool}`)
    for (const issue of result.issues) {
      // issue.kind: "no_match" | "property_missing" | "type_mismatch" | "no_schema"
      console.log(`  ${issue.message}`)
    }
  }
}
```

---

### Full example: custom CI script

```ts
import { check, diff } from 'nest-drift'
import { execSync } from 'child_process'

const checkReport = check('./src')
const diffReport  = diff('nest-drift.snapshot.json', './src')

if (checkReport.hasIssues || diffReport.hasChanges) {
  const summary = {
    checkIssues:  checkReport.results.filter(r => !r.ok).map(r => r.entity),
    schemaChanges: diffReport.changes.map(c => `${c.kind} ${c.name}`),
  }

  // Post to Slack, save to DB, open a GitHub issue — whatever you need
  await notifyTeam(summary)
  process.exit(1)
}
```

---

## Return types

```ts
// check()
interface CheckReport {
  entityCount: number
  dtoCount: number
  hasIssues: boolean
  results: EntityCheckResult[]
}
interface EntityCheckResult {
  entity: string
  dto: string | null
  ok: boolean
  issues: { kind: 'no_dto' | 'field_missing' | 'type_mismatch', message: string }[]
}

// snapshot()
interface ProjectSnapshot {
  entities: EntitySchema[]
  dtos: DtoSchema[]
}
interface EntitySchema {
  name: string
  file: string
  fields: { name: string, fieldType: string, optional: boolean }[]
}

// diff()
interface DiffReport {
  hasChanges: boolean
  changes: SchemaChange[]
}
interface SchemaChange {
  kind: 'added' | 'removed' | 'modified'
  schemaType: 'entity' | 'dto'
  name: string
  fieldChanges: FieldChange[]
}
interface FieldChange {
  kind: 'added' | 'removed' | 'type_changed' | 'optionality_changed'
  field: string
  before: string | null
  after: string | null
}

// watch()
type StopFn = () => void
function watch(path: string, callback: (report: CheckReport) => void): StopFn

// validate()
interface ValidateReport {
  toolCount: number
  hasIssues: boolean
  results: ToolValidateResult[]
}
interface ToolValidateResult {
  tool: string
  matchedSchema: string | null
  matchedFile: string | null
  ok: boolean
  issues: { kind: 'no_match' | 'property_missing' | 'type_mismatch' | 'no_schema', message: string }[]
}
```

---

## Built with

- [Rust](https://www.rust-lang.org/)
- [napi-rs](https://napi.rs/) — native Node.js bindings
- [clap](https://docs.rs/clap) — CLI parsing
- [serde](https://serde.rs/) + [serde_json](https://docs.rs/serde_json) + [serde_yaml](https://docs.rs/serde_yaml) — serialization
- [walkdir](https://docs.rs/walkdir) — directory traversal
- [regex](https://docs.rs/regex) — TypeScript parsing
- [colored](https://docs.rs/colored) — terminal output

---

## License

MIT
