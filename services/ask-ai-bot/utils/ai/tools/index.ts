import { tool } from "ai";
import { z } from "zod";
import { logger } from "../../logger";
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
};
