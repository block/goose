import fs from "fs";
import path from "path";
import { logger } from "../../logger";

export interface CodeSearchResult {
  filePath: string;
  line: number;
  content: string;
  context: string;
}

const SOURCE_EXTENSIONS = new Set([
  ".rs",
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".json",
  ".toml",
  ".yaml",
  ".yml",
  ".css",
  ".scss",
  ".html",
  ".md",
  ".sql",
  ".sh",
  ".mts",
]);

const IGNORED_DIRS = new Set([
  "node_modules",
  "target",
  "dist",
  "out",
  ".vite",
  ".git",
  "build",
  "coverage",
]);

function getCodebaseDir(): string {
  return process.env.CODEBASE_PATH || path.join(process.cwd(), "../..");
}

function getSearchableDirs(): { name: string; path: string }[] {
  const base = path.resolve(getCodebaseDir());
  return [
    { name: "ui", path: path.join(base, "ui") },
    { name: "crates", path: path.join(base, "crates") },
  ];
}

function shouldSkipDir(dirName: string): boolean {
  return IGNORED_DIRS.has(dirName);
}

function isSourceFile(fileName: string): boolean {
  const ext = path.extname(fileName).toLowerCase();
  return SOURCE_EXTENSIONS.has(ext);
}

function getContextLines(
  lines: string[],
  matchLine: number,
  contextSize: number = 2,
): string {
  const start = Math.max(0, matchLine - contextSize);
  const end = Math.min(lines.length - 1, matchLine + contextSize);
  const contextLines: string[] = [];

  for (let i = start; i <= end; i++) {
    const prefix = i === matchLine ? ">" : " ";
    contextLines.push(`${prefix} ${i + 1}: ${lines[i]}`);
  }

  return contextLines.join("\n");
}

function searchInFile(
  filePath: string,
  pattern: RegExp,
  baseDir: string,
): CodeSearchResult[] {
  const results: CodeSearchResult[] = [];

  try {
    const content = fs.readFileSync(filePath, "utf-8");
    const lines = content.split("\n");

    for (let i = 0; i < lines.length; i++) {
      if (pattern.test(lines[i])) {
        const relativePath = path.relative(baseDir, filePath);
        results.push({
          filePath: relativePath,
          line: i + 1,
          content: lines[i].trim(),
          context: getContextLines(lines, i),
        });
      }
    }
  } catch {
    // Skip files that can't be read (binary, permissions, etc.)
  }

  return results;
}

function walkAndSearch(
  dir: string,
  pattern: RegExp,
  baseDir: string,
  results: CodeSearchResult[],
  maxResults: number,
): void {
  if (results.length >= maxResults) return;

  try {
    const entries = fs.readdirSync(dir, { withFileTypes: true });

    for (const entry of entries) {
      if (results.length >= maxResults) return;

      if (entry.isDirectory()) {
        if (shouldSkipDir(entry.name)) continue;
        walkAndSearch(
          path.join(dir, entry.name),
          pattern,
          baseDir,
          results,
          maxResults,
        );
      } else if (isSourceFile(entry.name)) {
        const fileResults = searchInFile(
          path.join(dir, entry.name),
          pattern,
          baseDir,
        );
        for (const result of fileResults) {
          if (results.length >= maxResults) return;
          results.push(result);
        }
      }
    }
  } catch (error) {
    logger.error(`Error walking directory ${dir}:`, error);
  }
}

export function searchCodebase(
  query: string,
  limit: number = 20,
  scope?: string,
): CodeSearchResult[] {
  const searchDirs = getSearchableDirs();
  const allResults: CodeSearchResult[] = [];

  let pattern: RegExp;
  try {
    pattern = new RegExp(query, "i");
  } catch {
    pattern = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "i");
  }

  for (const dir of searchDirs) {
    if (scope && !dir.name.includes(scope)) continue;

    if (!fs.existsSync(dir.path)) {
      logger.warn(`Codebase directory not found: ${dir.path}`);
      continue;
    }

    walkAndSearch(
      dir.path,
      pattern,
      path.resolve(getCodebaseDir()),
      allResults,
      limit,
    );
  }

  logger.verbose(
    `Code search for "${query}" returned ${allResults.length} results`,
  );
  return allResults;
}

export function listCodebaseFiles(
  directory: string,
): { filePath: string; isDirectory: boolean }[] {
  const baseDir = path.resolve(getCodebaseDir());
  const targetDir = path.resolve(path.join(baseDir, directory));

  if (!targetDir.startsWith(baseDir)) {
    throw new Error("Invalid path - directory traversal not allowed");
  }

  if (!fs.existsSync(targetDir)) {
    throw new Error(`Directory not found: ${directory}`);
  }

  const stat = fs.statSync(targetDir);
  if (!stat.isDirectory()) {
    throw new Error(`Not a directory: ${directory}`);
  }

  try {
    const entries = fs.readdirSync(targetDir, { withFileTypes: true });
    return entries
      .filter((entry) => !shouldSkipDir(entry.name))
      .map((entry) => ({
        filePath: path.join(directory, entry.name),
        isDirectory: entry.isDirectory(),
      }))
      .sort((a, b) => {
        if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
        return a.filePath.localeCompare(b.filePath);
      });
  } catch (error) {
    throw new Error(`Failed to list directory: ${directory}`);
  }
}
