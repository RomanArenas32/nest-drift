# nest-drift

A CLI tool to detect schema drift in NestJS projects.

Validates consistency between entities and DTOs, snapshots your schema at a point in time, diffs changes across commits, and cross-checks LLM tool definitions against your actual codebase — so broken contracts get caught before they reach production.

---

## The problem

Someone renames a field in a TypeORM entity. The DTO doesn't get updated. The LLM tool definition still references the old field name. Everything breaks silently — in production, or worse, inside an AI agent's tool call.

`nest-drift` catches this at the source.

---

## Commands

### `check` — validate entities vs DTOs

Scans your project for `@Entity` classes and their related DTOs, then reports missing fields and type mismatches.

```bash
nest-drift check ./my-nestjs-project
```

```
nest-drift check
Scanning: ./my-nestjs-project

Found 3 entities, 7 DTOs

OK   User <-> CreateUserDto
FAIL User <-> UpdateUserDto
     Field 'email': type mismatch — entity has 'string', UpdateUserDto has 'number'
     Field 'role' exists in User but is missing from UpdateUserDto

OK   Product <-> CreateProductDto

Issues found. Review the above.
```

---

### `snapshot` — capture the current schema

Serializes all entities and DTOs into a JSON file you can commit or store as a baseline.

```bash
nest-drift snapshot ./my-nestjs-project
nest-drift snapshot ./my-nestjs-project --output custom-snap.json
```

---

### `diff` — compare schema against a snapshot

Compares the current state of your project against a previously generated snapshot. Reports added, removed, and modified classes and fields.

```bash
nest-drift diff nest-drift.snapshot.json ./my-nestjs-project
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

Exits with code `1` if changes are detected — ideal for CI gates or pre-commit hooks.

---

### `validate` — cross-check LLM tool definitions

Reads your LLM tool definitions (JSON or YAML) and verifies that every property maps to a real field in your codebase with a compatible type.

```bash
nest-drift validate tools.json ./my-nestjs-project
nest-drift validate tools.yaml ./my-nestjs-project
```

Supports both **Anthropic** (`input_schema`) and **OpenAI** (`parameters`) tool formats:

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
nest-drift validate
Tools:   tools.json
Project: ./my-nestjs-project

Found 2 tools, 5 DTOs, 2 entities

OK   create_user -> CreateUserDto (src/user/user.dto.ts)
FAIL update_user -> UpdateUserDto
     Tool 'update_user': property 'username' not found in codebase
     Tool 'update_user': property 'age' type mismatch — tool has 'integer', codebase has 'string'

Validation failed. Tool definitions are out of sync.
```

---

## Use as a pre-commit hook

Add to your `package.json`:

```json
{
  "scripts": {
    "precommit": "nest-drift check ."
  }
}
```

Or use with [Husky](https://typicode.github.io/husky):

```bash
npx husky add .husky/pre-commit "nest-drift check ."
```

---

## Use in CI

```yaml
- name: Check schema drift
  run: |
    nest-drift diff nest-drift.snapshot.json .
```

---

## Installation

> npm distribution coming soon.

For now, build from source:

```bash
git clone https://github.com/RomanArenas32/nest-drift.git
cd nest-drift
cargo build --release
./target/release/nest-drift --help
```

---

## Built with

- [Rust](https://www.rust-lang.org/)
- [clap](https://docs.rs/clap) — CLI parsing
- [serde](https://serde.rs/) + [serde_json](https://docs.rs/serde_json) + [serde_yaml](https://docs.rs/serde_yaml) — serialization
- [walkdir](https://docs.rs/walkdir) — directory traversal
- [regex](https://docs.rs/regex) — TypeScript parsing
- [colored](https://docs.rs/colored) — terminal output

---

## License

MIT
