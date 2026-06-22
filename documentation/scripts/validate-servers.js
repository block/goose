#!/usr/bin/env node
/**
 * Validates documentation/static/servers.json (the extension catalog).
 *
 * Errors fail the build (structural integrity + common submission mistakes).
 * Warnings are advisory quality signals and never fail the build, so adding a
 * stricter rule here won't retroactively break existing entries.
 *
 * Usage:
 *   node scripts/validate-servers.js [path/to/servers.json]
 *
 * Exit code 0 = no errors, 1 = one or more errors.
 */

const fs = require("fs");
const path = require("path");

const CATALOG_PATH =
  process.argv[2] ||
  path.join(__dirname, "..", "static", "servers.json");

const ALLOWED_KEYS = new Set([
  "id",
  "name",
  "description",
  "command",
  "url",
  "type",
  "link",
  "documentation",
  "installation_notes",
  "is_builtin",
  "endorsed",
  "show_install_link",
  "show_install_command",
  "environmentVariables",
  "headers",
]);

const REQUIRED_KEYS = [
  "id",
  "name",
  "description",
  "link",
  "is_builtin",
  "endorsed",
  "environmentVariables",
];

const ALLOWED_TYPES = new Set(["local", "remote", "streamable-http"]);
// Error if an id has spaces/slashes/odd characters; lowercase is enforced as a
// warning so existing mixed-case ids (which may back deep links) don't break CI.
const ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._-]*$/;
const URL_PATTERN = /^https?:\/\//;

// Launchers we know how to install/scan. Outside this set is allowed but flagged
// so a reviewer double-checks how the server actually starts.
const KNOWN_LAUNCHERS = new Set([
  "npx",
  "npm",
  "pnpm",
  "node",
  "uvx",
  "uv",
  "python",
  "python3",
  "docker",
  "go",
  "deno",
  "bunx",
]);

function isPlainObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function validateEnvVarList(entry, field, label, errors) {
  const list = entry[field];
  if (list === undefined) return;
  if (!Array.isArray(list)) {
    errors.push(`${label}: "${field}" must be an array`);
    return;
  }
  list.forEach((item, i) => {
    const where = `${label}: ${field}[${i}]`;
    if (!isPlainObject(item)) {
      errors.push(`${where} must be an object`);
      return;
    }
    if (typeof item.name !== "string" || item.name.length === 0) {
      errors.push(`${where} requires a non-empty string "name"`);
    }
    if (typeof item.description !== "string") {
      errors.push(`${where} requires a string "description"`);
    }
    if (typeof item.required !== "boolean") {
      errors.push(`${where} requires a boolean "required"`);
    }
  });
}

function validateEntry(entry, index, seenIds, errors, warnings) {
  const label = isPlainObject(entry) && typeof entry.id === "string"
    ? `entry "${entry.id}"`
    : `entry #${index}`;

  if (!isPlainObject(entry)) {
    errors.push(`${label} must be an object`);
    return;
  }

  for (const key of Object.keys(entry)) {
    if (!ALLOWED_KEYS.has(key)) {
      errors.push(`${label}: unknown field "${key}" (typo or unsupported key?)`);
    }
  }

  for (const key of REQUIRED_KEYS) {
    if (!(key in entry)) {
      errors.push(`${label}: missing required field "${key}"`);
    }
  }

  if (typeof entry.id === "string") {
    if (!ID_PATTERN.test(entry.id)) {
      errors.push(`${label}: "id" must match ${ID_PATTERN} (no spaces or slashes)`);
    } else if (entry.id !== entry.id.toLowerCase()) {
      warnings.push(`${label}: "id" should be lowercase by convention`);
    }
    if (seenIds.has(entry.id)) {
      errors.push(`${label}: duplicate "id" (ids must be unique)`);
    }
    seenIds.add(entry.id);
  } else if ("id" in entry) {
    errors.push(`${label}: "id" must be a string`);
  }

  if ("name" in entry && (typeof entry.name !== "string" || !entry.name.trim())) {
    errors.push(`${label}: "name" must be a non-empty string`);
  }
  if (
    "description" in entry &&
    (typeof entry.description !== "string" || !entry.description.trim())
  ) {
    errors.push(`${label}: "description" must be a non-empty string`);
  }

  for (const boolField of ["is_builtin", "endorsed", "show_install_link", "show_install_command"]) {
    if (boolField in entry && typeof entry[boolField] !== "boolean") {
      errors.push(`${label}: "${boolField}" must be a boolean`);
    }
  }

  for (const strField of ["command", "url", "installation_notes"]) {
    if (strField in entry && typeof entry[strField] !== "string") {
      errors.push(`${label}: "${strField}" must be a string`);
    }
  }

  for (const linkField of ["link", "documentation"]) {
    if (linkField in entry) {
      if (typeof entry[linkField] !== "string") {
        errors.push(`${label}: "${linkField}" must be a string`);
      } else if (entry[linkField].trim() === "") {
        // Empty link is a provenance gap, but several legacy entries ship one.
        warnings.push(`${label}: "${linkField}" is empty; add a source/repo URL`);
      } else if (!URL_PATTERN.test(entry[linkField])) {
        errors.push(`${label}: "${linkField}" must start with http:// or https://`);
      }
    }
  }

  if ("type" in entry && !ALLOWED_TYPES.has(entry.type)) {
    errors.push(
      `${label}: "type" must be one of ${[...ALLOWED_TYPES].join(", ")}`
    );
  }

  validateEnvVarList(entry, "environmentVariables", label, errors);
  validateEnvVarList(entry, "headers", label, errors);

  // Cross-field + quality rules below only run when core fields are well-formed.
  const isBuiltin = entry.is_builtin === true;
  const command = typeof entry.command === "string" ? entry.command.trim() : "";
  const url = typeof entry.url === "string" ? entry.url.trim() : "";

  if (!isBuiltin) {
    const hasInstall = command.length > 0 || url.length > 0;
    const manualInstall = entry.show_install_command === false;
    if (!hasInstall && !manualInstall) {
      errors.push(
        `${label}: non-builtin entries need a non-empty "command" or "url" ` +
          `(or set "show_install_command": false for manual-only setup)`
      );
    }
    if (command && url) {
      warnings.push(`${label}: has both "command" and "url"; usually only one applies`);
    }
    if (
      typeof entry.installation_notes !== "string" ||
      !entry.installation_notes.trim()
    ) {
      warnings.push(`${label}: missing "installation_notes"`);
    }
  }

  if (url && !("type" in entry)) {
    warnings.push(`${label}: has "url" but no "type" (expected remote/streamable-http)`);
  }

  if (command) {
    const launcher = command.split(/\s+/)[0];
    if (!KNOWN_LAUNCHERS.has(launcher)) {
      warnings.push(
        `${label}: command launcher "${launcher}" is not in the known set; ` +
          `verify how this server starts`
      );
    }
  }
}

/**
 * Validates a parsed catalog array. Returns { errors, warnings } (arrays of
 * strings). Does not read files or exit; callers decide what to do.
 */
function validateCatalog(catalog) {
  const errors = [];
  const warnings = [];

  if (!Array.isArray(catalog)) {
    errors.push("catalog must be a top-level JSON array");
    return { errors, warnings };
  }

  const seenIds = new Set();
  catalog.forEach((entry, index) => {
    validateEntry(entry, index, seenIds, errors, warnings);
  });

  return { errors, warnings };
}

function main() {
  let raw;
  try {
    raw = fs.readFileSync(CATALOG_PATH, "utf8");
  } catch (e) {
    console.error(`Could not read catalog at ${CATALOG_PATH}: ${e.message}`);
    process.exit(1);
  }

  let catalog;
  try {
    catalog = JSON.parse(raw);
  } catch (e) {
    console.error(`${CATALOG_PATH} is not valid JSON: ${e.message}`);
    process.exit(1);
  }

  if (!Array.isArray(catalog)) {
    console.error(`${CATALOG_PATH} must contain a top-level JSON array`);
    process.exit(1);
  }

  const { errors, warnings } = validateCatalog(catalog);

  if (warnings.length > 0) {
    console.warn(`\n${warnings.length} warning(s):`);
    for (const w of warnings) console.warn(`  - ${w}`);
  }

  if (errors.length > 0) {
    console.error(`\n${errors.length} error(s) in ${path.basename(CATALOG_PATH)}:`);
    for (const e of errors) console.error(`  - ${e}`);
    console.error(
      `\nValidated ${catalog.length} entries. Fix the errors above and re-run.`
    );
    process.exit(1);
  }

  console.log(
    `${path.basename(CATALOG_PATH)}: ${catalog.length} entries valid` +
      (warnings.length ? ` (${warnings.length} warning(s))` : "")
  );
}

module.exports = { validateCatalog, validateEntry, CATALOG_PATH };

if (require.main === module) {
  main();
}
