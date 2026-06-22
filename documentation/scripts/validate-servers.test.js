const { test, describe } = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const { validateCatalog, CATALOG_PATH } = require("./validate-servers");

function baseEntry(overrides = {}) {
  return {
    id: "example-mcp",
    name: "Example",
    description: "An example MCP server",
    command: "npx -y @example/mcp",
    link: "https://github.com/example/mcp",
    installation_notes: "Install using npx.",
    is_builtin: false,
    endorsed: false,
    environmentVariables: [],
    ...overrides,
  };
}

describe("live catalog", () => {
  test("documentation/static/servers.json has no errors", () => {
    const catalog = JSON.parse(fs.readFileSync(CATALOG_PATH, "utf8"));
    const { errors } = validateCatalog(catalog);
    assert.deepStrictEqual(
      errors,
      [],
      `servers.json has validation errors:\n${errors.join("\n")}`
    );
  });
});

describe("validateCatalog", () => {
  test("accepts a well-formed entry", () => {
    const { errors } = validateCatalog([baseEntry()]);
    assert.deepStrictEqual(errors, []);
  });

  test("rejects a non-array catalog", () => {
    const { errors } = validateCatalog({});
    assert.ok(errors.length > 0);
  });

  test("requires core fields", () => {
    const { errors } = validateCatalog([{ id: "x" }]);
    assert.ok(errors.some((e) => e.includes('missing required field "name"')));
    assert.ok(errors.some((e) => e.includes('missing required field "link"')));
  });

  test("rejects unknown fields", () => {
    const { errors } = validateCatalog([baseEntry({ commnd: "typo" })]);
    assert.ok(errors.some((e) => e.includes('unknown field "commnd"')));
  });

  test("rejects duplicate ids", () => {
    const { errors } = validateCatalog([baseEntry(), baseEntry()]);
    assert.ok(errors.some((e) => e.includes("duplicate")));
  });

  test("rejects ids with spaces", () => {
    const { errors } = validateCatalog([baseEntry({ id: "bad id" })]);
    assert.ok(errors.some((e) => e.includes('"id" must match')));
  });

  test("rejects a non-http link", () => {
    const { errors } = validateCatalog([baseEntry({ link: "github.com/x" })]);
    assert.ok(errors.some((e) => e.includes("http://")));
  });

  test("rejects an invalid type", () => {
    const { errors } = validateCatalog([baseEntry({ type: "grpc" })]);
    assert.ok(errors.some((e) => e.includes('"type" must be one of')));
  });

  test("rejects malformed environmentVariables", () => {
    const { errors } = validateCatalog([
      baseEntry({ environmentVariables: [{ name: "", description: 1 }] }),
    ]);
    assert.ok(errors.some((e) => e.includes("environmentVariables")));
  });

  test("requires an install method for non-builtin entries", () => {
    const { errors } = validateCatalog([baseEntry({ command: "" })]);
    assert.ok(errors.some((e) => e.includes("non-builtin")));
  });

  test("allows manual-only non-builtin entries", () => {
    const { errors } = validateCatalog([
      baseEntry({ command: "", show_install_command: false }),
    ]);
    assert.deepStrictEqual(errors, []);
  });

  test("allows builtin entries without a command", () => {
    const { errors } = validateCatalog([
      baseEntry({ is_builtin: true, command: "" }),
    ]);
    assert.deepStrictEqual(errors, []);
  });

  test("warns (not errors) on a missing installation_notes", () => {
    const entry = baseEntry();
    delete entry.installation_notes;
    const { errors, warnings } = validateCatalog([entry]);
    assert.deepStrictEqual(errors, []);
    assert.ok(warnings.some((w) => w.includes("installation_notes")));
  });
});
