import { tool } from "ai";
import { z } from "zod";
import { logger } from "../../logger";
import { listCodebaseFiles, searchCodebase } from "./codebase-search";
import { viewCodebaseFiles } from "./codebase-viewer";
import { searchDocs } from "./docs-search";
import { viewDocs } from "./docs-viewer";

export const aiTools = {
  search_docs: tool({
    description: "Search the goose documentation for relevant information",
    inputSchema: z.object({
      query: z
        .string()
        .describe(
          "Search query for the documentation (example: 'sessions', 'tool management')",
        ),
      limit: z
        .number()
        .optional()
        .describe("Maximum number of results to return (default 15)"),
    }),
    execute: async ({ query, limit = 15 }) => {
      const results = searchDocs(query, limit);
      logger.verbose(
        `Searched docs for "${query}", found ${results.length} results`,
      );

      if (results.length === 0) {
        return "No relevant documentation found for your query. Try different keywords.";
      }

      return results
        .map(
          (r) =>
            `**${r.fileName}** (${r.filePath})\nPreview: ${r.preview}\nWeb URL: <${r.webUrl}>`,
        )
        .join("\n\n");
    },
  }),
  view_docs: tool({
    description: "View documentation file(s)",
    inputSchema: z.object({
      filePaths: z
        .union([z.string(), z.array(z.string())])
        .describe(
          "Path or array of paths to documentation files (example: 'quickstart.md' or ['guides/managing-projects.md', 'mcp/asana-mcp.md'])",
        ),
      startLine: z
        .number()
        .optional()
        .describe("Starting line number (0-indexed, default 0)"),
      lineCount: z
        .number()
        .optional()
        .describe("Number of lines to show (default 1500)"),
    }),
    execute: async ({ filePaths, startLine = 0, lineCount = 1500 }) => {
      try {
        const result = viewDocs(filePaths, startLine, lineCount);
        const count = Array.isArray(filePaths) ? filePaths.length : 1;
        logger.verbose(`Viewed ${count} documentation file(s)`);
        return result;
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : "Unknown error";
        logger.error(`Error viewing docs: ${errorMsg}`);
        return `Error viewing documentation: ${errorMsg}`;
      }
    },
  }),
  search_codebase: tool({
    description:
      "Search the goose source code (Rust crates and TypeScript UI) using regex patterns. Searches across ui/ and crates/. Use this to find function definitions, struct/type definitions, imports, error messages, or any code pattern.",
    inputSchema: z.object({
      query: z
        .string()
        .describe(
          "Regex pattern to search for in the codebase (example: 'fn create_session', 'struct Provider', 'impl.*Agent')",
        ),
      limit: z
        .number()
        .optional()
        .describe("Maximum number of results to return (default 20)"),
      scope: z
        .string()
        .optional()
        .describe(
          "Limit search to a specific area: 'ui/' for the desktop and other UIs, 'crates' for Rust backend code. Omit to search everything.",
        ),
    }),
    execute: async ({ query, limit = 20, scope }) => {
      try {
        const results = searchCodebase(query, limit, scope);

        if (results.length === 0) {
          return "No matches found in the codebase. Try a different pattern or broader search.";
        }

        return results
          .map(
            (r) => `**${r.filePath}:${r.line}**\n\`\`\`\n${r.context}\n\`\`\``,
          )
          .join("\n\n");
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : "Unknown error";
        logger.error(`Error searching codebase: ${errorMsg}`);
        return `Error searching codebase: ${errorMsg}`;
      }
    },
  }),
  view_codebase: tool({
    description:
      "View source code file(s) from the goose codebase. Paths are relative to the repository root (e.g., 'crates/goose/src/agents/agent.rs' or 'ui/desktop/src/App.tsx').",
    inputSchema: z.object({
      filePaths: z
        .union([z.string(), z.array(z.string())])
        .describe(
          "Path or array of paths to source files relative to the repo root (example: 'crates/goose/src/agents/agent.rs' or ['ui/desktop/src/main.ts', 'crates/goose-server/src/main.rs'])",
        ),
      startLine: z
        .number()
        .optional()
        .describe("Starting line number (0-indexed, default 0)"),
      lineCount: z
        .number()
        .optional()
        .describe(
          "Number of lines to show (default 200). Use smaller values for focused reading, larger for overview.",
        ),
    }),
    execute: async ({ filePaths, startLine = 0, lineCount = 200 }) => {
      try {
        const result = viewCodebaseFiles(filePaths, startLine, lineCount);
        const count = Array.isArray(filePaths) ? filePaths.length : 1;
        logger.verbose(`Viewed ${count} codebase file(s)`);
        return result;
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : "Unknown error";
        logger.error(`Error viewing codebase: ${errorMsg}`);
        return `Error viewing codebase: ${errorMsg}`;
      }
    },
  }),
  list_codebase_files: tool({
    description:
      "List files and directories in a codebase directory. Use this to explore the project structure before viewing specific files. Only works within ui/ and crates/.",
    inputSchema: z.object({
      directory: z
        .string()
        .describe(
          "Directory path relative to repo root (example: 'crates/goose/src', 'ui/desktop/src/components')",
        ),
    }),
    execute: async ({ directory }) => {
      try {
        const entries = listCodebaseFiles(directory);

        if (entries.length === 0) {
          return `Directory "${directory}" is empty.`;
        }

        return entries
          .map((e) => `${e.isDirectory ? "[dir] " : "      "}${e.filePath}`)
          .join("\n");
      } catch (error) {
        const errorMsg =
          error instanceof Error ? error.message : "Unknown error";
        logger.error(`Error listing codebase files: ${errorMsg}`);
        return `Error listing files: ${errorMsg}`;
      }
    },
  }),
};
