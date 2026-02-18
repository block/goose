#!/usr/bin/env node
/**
 * Generates TypeScript types + Zod validators for Goose custom extension methods.
 *
 * Usage:
 *   npm run generate              # build Rust schema, then generate TS
 *   npm run generate:skip-build   # use existing schema files
 */

import { createClient } from "@hey-api/openapi-ts";
import { execSync } from "child_process";
import * as fs from "fs/promises";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";
import * as prettier from "prettier";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const ROOT = resolve(__dirname, "../..");
const SCHEMA_PATH = resolve(ROOT, "crates/goose-acp/acp-schema.json");
const META_PATH = resolve(ROOT, "crates/goose-acp/acp-meta.json");
const OUTPUT_DIR = resolve(__dirname, "src/generated");

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

async function main() {
  // 1. Optionally rebuild the schema from Rust
  if (!process.argv.includes("--skip-build")) {
    console.log("Building Goose extension schema from Rust...");
    try {
      execSync(
        "source bin/activate-hermit && cargo run -p goose-acp --bin generate-acp-schema",
        {
          cwd: ROOT,
          stdio: "inherit",
          shell: "/bin/zsh",
        },
      );
    } catch {
      console.error(
        "Failed to build schema. Run with --skip-build to use existing files.",
      );
      process.exit(1);
    }
  }

  // 2. Read the JSON schema and metadata
  const schemaSrc = await fs.readFile(SCHEMA_PATH, "utf8");
  const jsonSchema = JSON.parse(
    // Convert JSON Schema $defs refs to OpenAPI component refs
    schemaSrc.replaceAll("#/$defs/", "#/components/schemas/"),
  );

  const metaSrc = await fs.readFile(META_PATH, "utf8");
  const meta = JSON.parse(metaSrc);

  // 3. Generate TypeScript types + Zod validators via @hey-api/openapi-ts
  await createClient({
    input: {
      openapi: "3.1.0",
      info: {
        title: "Goose Extensions",
        version: "1.0.0",
      },
      components: {
        schemas: jsonSchema.$defs,
      },
    },
    output: {
      path: OUTPUT_DIR,
    },
    plugins: ["zod", "@hey-api/typescript"],
  });

  // 4. Post-process generated files
  await postProcessTypes();
  await postProcessIndex(meta);

  console.log(`\nGenerated Goose extension schema in ${OUTPUT_DIR}`);
}

async function postProcessTypes() {
  const tsPath = resolve(OUTPUT_DIR, "types.gen.ts");
  let src = await fs.readFile(tsPath, "utf8");
  // Remove the ClientOptions type block injected by @hey-api (not part of our schema)
  src = src.replace(/\nexport type ClientOptions =[\s\S]*?^};\n/m, "\n");
  await fs.writeFile(tsPath, src);
}

async function postProcessIndex(meta: { methods: unknown[] }) {
  const indexPath = resolve(OUTPUT_DIR, "index.ts");
  let src = await fs.readFile(indexPath, "utf8");

  // Strip ClientOptions from re-exports
  src = src.replace(/,?\s*ClientOptions\s*,?/g, (match) => {
    if (match.startsWith(",") && match.endsWith(",")) return ",";
    if (match.startsWith(",")) return "";
    return "";
  });

  // Append method constants
  const methodConstants = await prettier.format(
    `
export const GOOSE_EXT_METHODS = ${JSON.stringify(meta.methods, null, 2)} as const;

export type GooseExtMethod = (typeof GOOSE_EXT_METHODS)[number];
`,
    { parser: "typescript" },
  );

  await fs.writeFile(indexPath, `${src}\n${methodConstants}`);
}
