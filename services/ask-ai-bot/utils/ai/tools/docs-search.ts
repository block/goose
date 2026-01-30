import fs from "fs";
import MiniSearch from "minisearch";
import path from "path";
import { logger } from "../../logger";

export interface SearchResult {
  filePath: string;
  fileName: string;
  score: number;
  preview: string;
  lineCount: number;
  webUrl: string;
}

interface DocFile {
  id: string;
  path: string;
  fileName: string;
  content: string;
  lineCount: number;
}

let miniSearch: MiniSearch<DocFile> | null = null;

function getDocsDir(): string {
  return process.env.DOCS_PATH || path.join(process.cwd(), "docs");
}

function initializeSearch(): MiniSearch<DocFile> {
  if (miniSearch) {
    return miniSearch;
  }

  const docsDir = getDocsDir();

  if (!fs.existsSync(docsDir)) {
    logger.warn(`Docs directory not found at ${docsDir}`);
    miniSearch = new MiniSearch({
      fields: ["content", "fileName", "path"],
      storeFields: ["path", "fileName", "content", "lineCount"],
    });
    return miniSearch;
  }

  const docs: DocFile[] = [];

  function walkDir(dir: string) {
    try {
      const files = fs.readdirSync(dir);

      for (const file of files) {
        const filePath = path.join(dir, file);
        const stat = fs.statSync(filePath);

        if (stat.isDirectory()) {
          if (file === "assets" || file === "docker") {
            continue;
          }
          walkDir(filePath);
        } else {
          try {
            const content = fs.readFileSync(filePath, "utf-8");
            const relativePath = path.relative(docsDir, filePath);
            const docFile: DocFile = {
              id: relativePath,
              path: relativePath,
              fileName: file,
              content,
              lineCount: content.split("\n").length,
            };
            docs.push(docFile);
          } catch (error) {
            logger.error(`Error reading file ${filePath}:`, error);
          }
        }
      }
    } catch (error) {
      logger.error(`Error walking directory ${dir}:`, error);
    }
  }

  walkDir(docsDir);

  miniSearch = new MiniSearch({
    fields: ["content", "fileName", "path"],
    storeFields: ["path", "fileName", "content", "lineCount"],
  });

  miniSearch.addAll(docs);
  logger.verbose(`Loaded ${docs.length} documentation files`);

  return miniSearch;
}

function generateWebUrl(filePath: string): string {
  const baseUrl = "https://block.github.io/goose/docs";
  // Remove file extension for the URL path
  const urlPath = filePath.replace(/\.[^/.]+$/, "");
  return `${baseUrl}/${urlPath}`;
}

function getPreview(content: string, maxLength: number = 200): string {
  const withoutFrontmatter = content.replace(/^---[\s\S]*?---\n/, "");
  const lines = withoutFrontmatter.split("\n");
  let preview = "";

  for (const line of lines) {
    const cleanLine = line
      .replace(/^#+\s+/, "")
      .replace(/\[([^\]]+)\]\([^\)]+\)/g, "$1")
      .replace(/[*_]/g, "")
      .trim();

    if (
      cleanLine &&
      !cleanLine.startsWith("import") &&
      !cleanLine.startsWith("export")
    ) {
      preview = cleanLine;
      break;
    }
  }

  if (preview.length > maxLength) {
    preview = preview.substring(0, maxLength) + "...";
  }

  return preview || "(No preview available)";
}

export function searchDocs(query: string, limit: number = 5): SearchResult[] {
  const search = initializeSearch();
  const results = search.search(query).slice(0, limit);

  if (results.length === 0) {
    logger.verbose(`Search for "${query}" returned no results`);
    return [];
  }

  const searchResults: SearchResult[] = results.map((result) => ({
    filePath: result.path,
    fileName: result.fileName,
    score: result.score,
    preview: getPreview(result.content),
    lineCount: result.lineCount,
    webUrl: generateWebUrl(result.path),
  }));

  logger.verbose(
    `Search for "${query}" returned ${searchResults.length} results`,
  );
  return searchResults;
}

export function reloadDocsCache(): void {
  miniSearch = null;
  initializeSearch();
}
