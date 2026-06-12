'use strict'

const { existsSync } = require('fs')
const { join } = require('path')

// Try to load the pre-built native binding
const platforms = {
  'darwin-x64': 'nest_drift.darwin-x64.node',
  'darwin-arm64': 'nest_drift.darwin-arm64.node',
  'linux-x64-gnu': 'nest_drift.linux-x64-gnu.node',
  'linux-arm64-gnu': 'nest_drift.linux-arm64-gnu.node',
  'win32-x64-msvc': 'nest_drift.win32-x64-msvc.node',
}

const platform = `${process.platform}-${process.arch}`
const binding = platforms[platform]

if (!binding) {
  throw new Error(`nest-drift: unsupported platform ${platform}`)
}

const bindingPath = join(__dirname, binding)

if (!existsSync(bindingPath)) {
  throw new Error(
    `nest-drift: native binding not found at ${bindingPath}.\n` +
    `Run \`npm run build\` to compile it.`
  )
}

const native = require(bindingPath)

/**
 * Watch `path` for TypeScript file changes and call `callback` with a
 * fresh CheckReport on every change (debounced 300ms).
 *
 * @param {string} path - Path to the NestJS project
 * @param {(report: import('./index').CheckReport) => void} callback
 * @returns {() => void} stop function — call it to stop watching
 */
function watch(path, callback) {
  const id = native.watchStart(path, (_err, json) => {
    try {
      callback(JSON.parse(json))
    } catch (_) {}
  })
  return () => native.watchStop(id)
}

module.exports = { ...native, watch }
