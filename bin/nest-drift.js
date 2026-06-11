#!/usr/bin/env node
'use strict'

const native = require('../index.js')

const args = process.argv.slice(2)
const command = args[0]

function bold(s)   { return `\x1b[1m${s}\x1b[0m` }
function dim(s)    { return `\x1b[2m${s}\x1b[0m` }
function red(s)    { return `\x1b[31m${s}\x1b[0m` }
function green(s)  { return `\x1b[32m${s}\x1b[0m` }
function yellow(s) { return `\x1b[33m${s}\x1b[0m` }
function cyan(s)   { return `\x1b[36m${s}\x1b[0m` }

function die(msg) {
  console.error(bold(red('ERROR')) + ' ' + msg)
  process.exit(1)
}

// ── check ─────────────────────────────────────────────────────────────────────

if (command === 'check') {
  const path = args[1] ?? '.'
  console.log(bold('nest-drift check'))
  console.log('Scanning: ' + dim(path) + '\n')

  const report = native.check(path)

  if (report.entityCount === 0) {
    console.log(yellow('No @Entity classes found.'))
    process.exit(0)
  }

  console.log(`Found ${cyan(report.entityCount)} entities, ${cyan(report.dtoCount)} DTOs\n`)

  for (const result of report.results) {
    const dto = result.dto ?? '—'
    if (result.ok) {
      console.log(`${bold(green('OK  '))} ${green(result.entity)} <-> ${dim(dto)}`)
    } else {
      console.log(`${bold(red('FAIL'))} ${red(result.entity)} <-> ${dim(dto)}`)
      for (const issue of result.issues) {
        console.log(`     ${red(issue.message)}`)
      }
    }
  }

  console.log()
  if (report.hasIssues) {
    console.log(bold(red('Issues found. Review the above.')))
    process.exit(1)
  } else {
    console.log(bold(green('All checks passed.')))
  }

// ── snapshot ──────────────────────────────────────────────────────────────────

} else if (command === 'snapshot') {
  const path   = args[1] ?? '.'
  let output   = 'nest-drift.snapshot.json'
  const outIdx = args.indexOf('--output') !== -1 ? args.indexOf('--output') : args.indexOf('-o')
  if (outIdx !== -1 && args[outIdx + 1]) output = args[outIdx + 1]

  console.log(bold('nest-drift snapshot'))
  console.log('Scanning: ' + dim(path) + '\n')

  const schema = native.snapshot(path)

  if (schema.entities.length === 0 && schema.dtos.length === 0) {
    console.log(yellow('Nothing found to snapshot.'))
    process.exit(0)
  }

  const { writeFileSync } = require('fs')
  writeFileSync(output, JSON.stringify(schema, null, 2))

  console.log(`${bold(green('OK'))} Snapshot saved to ${cyan(output)}`)
  console.log(`   ${cyan(schema.entities.length)} entities, ${cyan(schema.dtos.length)} DTOs captured`)

// ── diff ──────────────────────────────────────────────────────────────────────

} else if (command === 'diff') {
  const snapshotPath = args[1] ?? 'nest-drift.snapshot.json'
  const path         = args[2] ?? '.'

  console.log(bold('nest-drift diff'))
  console.log('Snapshot: ' + dim(snapshotPath))
  console.log('Project:  ' + dim(path) + '\n')

  let report
  try {
    report = native.diff(snapshotPath, path)
  } catch (e) {
    die(e.message)
  }

  const entities = report.changes.filter(c => c.schemaType === 'entity')
  const dtos     = report.changes.filter(c => c.schemaType === 'dto')

  console.log(bold('\x1b[4mEntities\x1b[0m'))
  if (entities.length === 0) {
    console.log('  ' + dim('No changes.'))
  }
  for (const c of entities) printSchemaChange(c)

  console.log()
  console.log(bold('\x1b[4mDTOs\x1b[0m'))
  if (dtos.length === 0) {
    console.log('  ' + dim('No changes.'))
  }
  for (const c of dtos) printSchemaChange(c)

  console.log()
  if (report.hasChanges) {
    console.log(bold(red('Schema has changed since last snapshot.')))
    process.exit(1)
  } else {
    console.log(bold(green('No changes detected.')))
  }

// ── validate ──────────────────────────────────────────────────────────────────

} else if (command === 'validate') {
  const toolsPath = args[1]
  const path      = args[2] ?? '.'

  if (!toolsPath) die('Usage: nest-drift validate <tools.json> [path]')

  console.log(bold('nest-drift validate'))
  console.log('Tools:   ' + dim(toolsPath))
  console.log('Project: ' + dim(path) + '\n')

  let report
  try {
    report = native.validate(toolsPath, path)
  } catch (e) {
    die(e.message)
  }

  if (report.toolCount === 0) {
    console.log(yellow('No tools found in file.'))
    process.exit(0)
  }

  console.log(`Found ${cyan(report.toolCount)} tools\n`)

  for (const result of report.results) {
    if (result.ok) {
      console.log(`${bold(green('OK  '))} ${green(result.tool)} -> ${green(result.matchedSchema)} (${dim(result.matchedFile)})`)
    } else {
      const issue = result.issues[0]
      if (issue.kind === 'no_schema') {
        console.log(`${bold(yellow('SKIP'))} ${yellow(result.tool)} — no input_schema/parameters found, skipping`)
      } else if (issue.kind === 'no_match') {
        console.log(`${bold(yellow('WARN'))} ${yellow(result.tool)} — no matching DTO or Entity found in codebase`)
      } else {
        console.log(`${bold(red('FAIL'))} ${red(result.tool)} -> ${red(result.matchedSchema ?? '?')}`)
        for (const i of result.issues) {
          console.log(`     ${red(i.message)}`)
        }
      }
    }
  }

  console.log()
  if (report.hasIssues) {
    console.log(bold(red('Validation failed. Tool definitions are out of sync.')))
    process.exit(1)
  } else {
    console.log(bold(green('All tools are valid.')))
  }

// ── help ──────────────────────────────────────────────────────────────────────

} else {
  console.log(`
${bold('nest-drift')} — NestJS schema consistency validator

${bold('Commands:')}
  ${cyan('check')}    [path]                    Validate entities vs DTOs
  ${cyan('snapshot')} [path] [-o output.json]   Capture current schema
  ${cyan('diff')}     [snapshot] [path]         Compare against a snapshot
  ${cyan('validate')} <tools.json> [path]       Validate LLM tool definitions

${bold('Examples:')}
  npx nest-drift check .
  npx nest-drift snapshot . --output snap.json
  npx nest-drift diff nest-drift.snapshot.json .
  npx nest-drift validate tools.json .
`)
  if (command && command !== '--help' && command !== '-h') {
    die(`Unknown command: ${command}`)
  }
}

// ── helpers ───────────────────────────────────────────────────────────────────

function printSchemaChange(c) {
  if (c.kind === 'added') {
    console.log(`  ${bold(green('+'))} ${green(c.name)} (new)`)
  } else if (c.kind === 'removed') {
    console.log(`  ${bold(red('-'))} ${red(c.name)} (removed)`)
  } else {
    console.log(`  ${bold(yellow('~'))} ${yellow(c.name)}`)
    for (const fc of c.fieldChanges) {
      if (fc.kind === 'added') {
        console.log(`    ${green('+')} field '${green(fc.field)}': ${fc.after} (added)`)
      } else if (fc.kind === 'removed') {
        console.log(`    ${red('-')} field '${red(fc.field)}': ${fc.before} (removed)`)
      } else if (fc.kind === 'type_changed') {
        console.log(`    ${yellow('~')} field '${yellow(fc.field)}': ${red(fc.before)} -> ${green(fc.after)} (type changed)`)
      } else if (fc.kind === 'optionality_changed') {
        console.log(`    ${yellow('~')} field '${yellow(fc.field)}': ${red(fc.before)} -> ${green(fc.after)} (optionality changed)`)
      }
    }
  }
}
