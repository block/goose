#!/usr/bin/env node
/**
 * Generates TypeScript types + Zod validators for Goose custom extension methods.
 *
 * Usage:
 *   npm run generate              # build Rust schema, then generate TS
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
const SCHEMA_PATH = resolve(ROOT, "crates/goose/acp-schema.json");
const META_PATH = resolve(ROOT, "crates/goose/acp-meta.json");
const OUTPUT_DIR = resolve(__dirname, "src/generated");

// Export the main function so it can be imported by build-schema.ts
export default async function main() {
  const schemaSrc = await fs.readFile(SCHEMA_PATH, "utf8");
  const jsonSchema = JSON.parse(
    schemaSrc.replaceAll("#/$defs/", "#/components/schemas/"),
  );

  const metaSrc = await fs.readFile(META_PATH, "utf8");
  const meta = JSON.parse(metaSrc);

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
    plugins: [
      {
        case: "preserve",
        name: "zod",
      },
      {
        case: "preserve",
        name: "@hey-api/typescript",
      },
    ],
  });

  await postProcessTypes();
  await postProcessIndex(meta);

  await generateClient(meta);

  console.log(`\nGenerated Goose extension schema in ${OUTPUT_DIR}`);
}

async function postProcessTypes() {
  const tsPath = resolve(OUTPUT_DIR, "types.gen.ts");
  let src = await fs.readFile(tsPath, "utf8");
  src = src.replace(/\nexport type ClientOptions =[\s\S]*?^};\n/m, "\n");
  await fs.writeFile(tsPath, src);
}

async function postProcessIndex(meta: {
  methods: unknown[];
  notifications?: unknown[];
}) {
  const indexPath = resolve(OUTPUT_DIR, "index.ts");
  let src = await fs.readFile(indexPath, "utf8");

  src = src.replace(/,?\s*ClientOptions\s*,?/g, (match) => {
    if (match.startsWith(",") && match.endsWith(",")) return ",";
    if (match.startsWith(",")) return "";
    return "";
  });

  src = fixRelativeImports(src);

  const methodConstants = await prettier.format(
    `
export const GOOSE_EXT_METHODS = ${JSON.stringify(meta.methods, null, 2)} as const;

export type GooseExtMethod = (typeof GOOSE_EXT_METHODS)[number];

export const GOOSE_EXT_NOTIFICATIONS = ${JSON.stringify(meta.notifications ?? [], null, 2)} as const;

export type GooseExtNotification = (typeof GOOSE_EXT_NOTIFICATIONS)[number];
`,
    { parser: "typescript" },
  );

  await fs.writeFile(indexPath, `${src}\n${methodConstants}`);

  for (const file of ["zod.gen.ts", "types.gen.ts"]) {
    const filePath = resolve(OUTPUT_DIR, file);
    try {
      const content = await fs.readFile(filePath, "utf8");
      const fixed = fixRelativeImports(content);
      if (fixed !== content) {
        await fs.writeFile(filePath, fixed);
      }
    } catch {
      // File may not exist
    }
  }
}

function fixRelativeImports(src: string): string {
  return src.replace(
    /from\s+['"](\.[^'"]+)['"]/g,
    (_match, importPath: string) => {
      if (importPath.endsWith(".js") || importPath.endsWith(".json")) {
        return `from '${importPath}'`;
      }
      return `from '${importPath}.js'`;
    },
  );
}

interface MethodMeta {
  method: string;
  requestType: string | null;
  responseType: string | null;
}

interface NotificationMeta {
  method: string;
  paramsType: string | null;
}

function methodToHandlerName(method: string): string {
  let methodParts = method.split(/[/_]/).filter((part) => part.length > 0);
  let prefix = "";
  if (methodParts[0] == "goose" && methodParts[1] == "unstable") {
    methodParts.shift();
    methodParts.shift();
    prefix = "unstable_";
  } else if (methodParts[0] == "goose") {
    methodParts.shift();
  }
  const body = methodParts
    .map((part) =>
      part.replace(/[^a-zA-Z0-9]+(.)/g, (_, chr: string) => chr.toUpperCase()),
    )
    .map((part, i) =>
      i === 0 ? part : part.charAt(0).toUpperCase() + part.slice(1),
    )
    .join("");
  return `${prefix}${body}`;
}

function methodToCamelCase(method: string): string {
  let methodParts = method.split(/[/_]/).filter((part) => part.length > 0);

  let suffix: string;
  if (methodParts[0] == "goose" && methodParts[1] == "unstable") {
    methodParts.shift();
    methodParts.shift();
    suffix = "_unstable";
  } else {
    suffix = "";
  }

  let prefix = methodParts
    .map((part) =>
      part.replace(/[^a-zA-Z0-9]+(.)/g, (_, chr: string) => chr.toUpperCase()),
    )
    .map((part, i) =>
      i === 0 ? part : part.charAt(0).toUpperCase() + part.slice(1),
    )
    .join("");

  return `${prefix}${suffix}`;
}

async function generateClient(meta: {
  methods: MethodMeta[];
  notifications?: NotificationMeta[];
}) {
  const typeImports = new Set<string>();
  const zodImports = new Set<string>();
  const upstreamTypeImports = new Set<string>(["Client"]);

  const methodDefs: string[] = [];

  for (const m of meta.methods) {
    const fnName = methodToCamelCase(m.method);
    const fullMethod = m.method;

    let paramType = "";
    let paramArg = "";
    let callParams = "{}";
    if (m.requestType) {
      typeImports.add(m.requestType);
      paramType = m.requestType;
      paramArg = `params: ${paramType}`;
      callParams = "params";
    }

    let returnType: string;
    let bodyLines: string[];

    if (m.responseType && m.responseType !== "EmptyResponse") {
      typeImports.add(m.responseType);
      const zodName = `z${m.responseType}`;
      zodImports.add(zodName);
      returnType = m.responseType;
      bodyLines = [
        `const raw = await this.conn.extMethod("${fullMethod}", ${callParams});`,
        `return ${zodName}.parse(raw) as ${returnType};`,
      ];
    } else if (m.responseType === "EmptyResponse") {
      returnType = "void";
      bodyLines = [
        `await this.conn.extMethod("${fullMethod}", ${callParams});`,
      ];
    } else {
      returnType = "Record<string, unknown>";
      bodyLines = [
        `return await this.conn.extMethod("${fullMethod}", ${callParams ? callParams : "{}"});`,
      ];
    }

    methodDefs.push(`
  async ${fnName}(${paramArg}): Promise<${returnType}> {
    ${bodyLines.join("\n    ")}
  }`);
  }

  const handlerFields: string[] = [];
  const dispatchCases: string[] = [];

  for (const n of meta.notifications ?? []) {
    const handlerName = methodToHandlerName(n.method);
    if (!n.paramsType) {
      handlerFields.push(
        `  ${handlerName}?: (params: Record<string, unknown>) => Promise<void>;`,
      );
      dispatchCases.push(
        `      case "${n.method}": {
        await callbacks.${handlerName}?.(params);
        return;
      }`,
      );
      continue;
    }
    typeImports.add(n.paramsType);
    const zodName = `z${n.paramsType}`;
    zodImports.add(zodName);
    handlerFields.push(
      `  ${handlerName}?: (notification: ${n.paramsType}) => Promise<void>;`,
    );
    dispatchCases.push(
      `      case "${n.method}": {
        const parsed = ${zodName}.parse(params) as ${n.paramsType};
        await callbacks.${handlerName}?.(parsed);
        return;
      }`,
    );
  }

  const handlersInterface = `export interface GooseExtNotifications {
${handlerFields.join("\n")}
}`;

  const dispatcherFn = `export function installGooseExtNotificationDispatcher(
  callbacks: GooseClientCallbacks,
): Client {
  const dispatcher: Pick<Client, "extNotification"> = {
    extNotification: async (method, params) => {
      switch (method) {
${dispatchCases.join("\n")}
        default:
          await callbacks.extNotification?.(method, params);
          return;
      }
    },
  };
  return new Proxy(callbacks, {
    get(target, property) {
      if (property === "extNotification") {
        return dispatcher.extNotification;
      }

      const value = Reflect.get(target, property, target);
      return typeof value === "function" ? value.bind(target) : value;
    },
  }) as Client;
}`;

  const upstreamImportLine = `import type { ${[...upstreamTypeImports].sort().join(", ")} } from "@agentclientprotocol/sdk";`;
  const typeImportLine = typeImports.size
    ? `import type { ${[...typeImports].sort().join(", ")} } from "./types.gen.js";`
    : "";
  const zodImportLine = zodImports.size
    ? `import { ${[...zodImports].sort().join(", ")} } from "./zod.gen.js";`
    : "";

  let src = `// This file is auto-generated — do not edit manually.

export interface ExtMethodProvider {
  extMethod(method: string, params: Record<string, unknown>): Promise<Record<string, unknown>>;
}

${upstreamImportLine}
${typeImportLine}
${zodImportLine}

export class GooseExtClient {
  constructor(private conn: ExtMethodProvider) {}
${methodDefs.join("\n")}
}

${handlersInterface}

export type GooseClientCallbacks =
  Omit<Client, "extNotification"> &
  Partial<Pick<Client, "extNotification">> &
  GooseExtNotifications;

${dispatcherFn}
`;

  src = await prettier.format(src, { parser: "typescript" });
  src = fixRelativeImports(src);

  const clientPath = resolve(OUTPUT_DIR, "client.gen.ts");
  await fs.writeFile(clientPath, src);
}

// Run main if this file is executed directly
if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((err) => {
    console.error(err);
    process.exit(1);
  });
}
