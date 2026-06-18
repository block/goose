import fs from 'node:fs';
import path from 'node:path';

export interface SeedBundledSecurityRuntimeAssetsOptions {
  isPackaged: boolean;
  distroDir?: string;
  workingDir: string;
}

export interface SeedBundledSecurityRuntimeAssetsResult {
  distroDir?: string;
  runtimeSkillRoot?: string;
  runtimeRecipeRoot?: string;
  seededSkillDirs: string[];
  seededRecipeFiles: string[];
  skippedReason?: 'not_packaged' | 'missing_distro' | 'missing_working_dir';
}

export interface InspectBundledSecurityRuntimeAssetsOptions {
  distroDir?: string;
  workingDir?: string;
}

export interface BundledSecurityRuntimeDiagnostics {
  distroDir?: string;
  workingDir?: string;
  runtimeSkillRoot?: string;
  runtimeRecipeRoot?: string;
  sourceSkillRoot?: string;
  sourceRecipeRoot?: string;
  sourceSkillIds: string[];
  sourceRecipeIds: string[];
  missingSkillIds: string[];
  driftedSkillIds: string[];
  missingRecipeIds: string[];
  driftedRecipeIds: string[];
  skippedReason?: 'missing_distro' | 'missing_working_dir';
}

function fileExists(filePath: string): boolean {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function directoryExists(directoryPath: string): boolean {
  try {
    return fs.statSync(directoryPath).isDirectory();
  } catch {
    return false;
  }
}

function normalizeLineEndings(value: string): string {
  return value.replace(/\r\n/g, '\n');
}

function copyFileIfMissing(sourcePath: string, targetPath: string): boolean {
  if (fs.existsSync(targetPath)) {
    return false;
  }

  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  fs.copyFileSync(sourcePath, targetPath);
  return true;
}

function copyDirectoryMissing(sourceDir: string, targetDir: string): boolean {
  let copiedAny = false;

  fs.mkdirSync(targetDir, { recursive: true });

  for (const entry of fs.readdirSync(sourceDir, { withFileTypes: true })) {
    const sourcePath = path.join(sourceDir, entry.name);
    const targetPath = path.join(targetDir, entry.name);

    if (entry.isDirectory()) {
      copiedAny = copyDirectoryMissing(sourcePath, targetPath) || copiedAny;
      continue;
    }

    if (entry.isFile()) {
      copiedAny = copyFileIfMissing(sourcePath, targetPath) || copiedAny;
    }
  }

  return copiedAny;
}

function collectSourceSkillDirs(sourceSkillRoot: string): string[] {
  if (!directoryExists(sourceSkillRoot)) {
    return [];
  }

  return fs
    .readdirSync(sourceSkillRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .filter((skillDir) => fileExists(path.join(sourceSkillRoot, skillDir, 'SKILL.md')))
    .sort();
}

function collectSourceRecipeIds(sourceRecipeRoot: string): string[] {
  if (!directoryExists(sourceRecipeRoot)) {
    return [];
  }

  return fs
    .readdirSync(sourceRecipeRoot, { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.yaml.example'))
    .map((entry) => entry.name.replace(/\.yaml\.example$/, ''))
    .sort();
}

function listRelativeFiles(rootDir: string, currentDir = rootDir): string[] {
  const relativeFiles: string[] = [];

  for (const entry of fs.readdirSync(currentDir, { withFileTypes: true })) {
    const entryPath = path.join(currentDir, entry.name);

    if (entry.isDirectory()) {
      relativeFiles.push(...listRelativeFiles(rootDir, entryPath));
      continue;
    }

    if (entry.isFile()) {
      relativeFiles.push(path.relative(rootDir, entryPath));
    }
  }

  return relativeFiles.sort();
}

export function seedBundledSecurityRuntimeAssets(
  options: SeedBundledSecurityRuntimeAssetsOptions
): SeedBundledSecurityRuntimeAssetsResult {
  const { isPackaged, distroDir, workingDir } = options;

  if (!isPackaged) {
    return {
      seededSkillDirs: [],
      seededRecipeFiles: [],
      skippedReason: 'not_packaged',
    };
  }

  if (!distroDir || !fileExists(path.join(distroDir, 'branding', 'product-metadata.json'))) {
    return {
      seededSkillDirs: [],
      seededRecipeFiles: [],
      skippedReason: 'missing_distro',
    };
  }

  if (!workingDir || !directoryExists(workingDir)) {
    return {
      distroDir,
      seededSkillDirs: [],
      seededRecipeFiles: [],
      skippedReason: 'missing_working_dir',
    };
  }

  const runtimeSkillRoot = path.join(workingDir, '.agents', 'skills');
  const runtimeRecipeRoot = path.join(workingDir, '.goose', 'recipes');
  const sourceSkillRoot = path.join(distroDir, 'skills');
  const sourceRecipeRoot = path.join(distroDir, 'recipes');
  const seededSkillDirs: string[] = [];
  const seededRecipeFiles: string[] = [];

  for (const skillDir of collectSourceSkillDirs(sourceSkillRoot)) {
    if (
      copyDirectoryMissing(path.join(sourceSkillRoot, skillDir), path.join(runtimeSkillRoot, skillDir))
    ) {
      seededSkillDirs.push(skillDir);
    }
  }

  for (const recipeId of collectSourceRecipeIds(sourceRecipeRoot)) {
    const runtimeRecipeFile = `${recipeId}.yaml`;
    if (
      copyFileIfMissing(
        path.join(sourceRecipeRoot, `${recipeId}.yaml.example`),
        path.join(runtimeRecipeRoot, runtimeRecipeFile)
      )
    ) {
      seededRecipeFiles.push(runtimeRecipeFile);
    }
  }

  return {
    distroDir,
    runtimeSkillRoot,
    runtimeRecipeRoot,
    seededSkillDirs,
    seededRecipeFiles,
  };
}

export function inspectBundledSecurityRuntimeAssets(
  options: InspectBundledSecurityRuntimeAssetsOptions
): BundledSecurityRuntimeDiagnostics {
  const { distroDir, workingDir } = options;

  if (!distroDir || !fileExists(path.join(distroDir, 'branding', 'product-metadata.json'))) {
    return {
      distroDir,
      workingDir,
      sourceSkillIds: [],
      sourceRecipeIds: [],
      missingSkillIds: [],
      driftedSkillIds: [],
      missingRecipeIds: [],
      driftedRecipeIds: [],
      skippedReason: 'missing_distro',
    };
  }

  if (!workingDir || !directoryExists(workingDir)) {
    return {
      distroDir,
      workingDir,
      sourceSkillIds: [],
      sourceRecipeIds: [],
      missingSkillIds: [],
      driftedSkillIds: [],
      missingRecipeIds: [],
      driftedRecipeIds: [],
      skippedReason: 'missing_working_dir',
    };
  }

  const runtimeSkillRoot = path.join(workingDir, '.agents', 'skills');
  const runtimeRecipeRoot = path.join(workingDir, '.goose', 'recipes');
  const sourceSkillRoot = path.join(distroDir, 'skills');
  const sourceRecipeRoot = path.join(distroDir, 'recipes');
  const sourceSkillIds = collectSourceSkillDirs(sourceSkillRoot);
  const sourceRecipeIds = collectSourceRecipeIds(sourceRecipeRoot);
  const missingSkillIds: string[] = [];
  const driftedSkillIds: string[] = [];
  const missingRecipeIds: string[] = [];
  const driftedRecipeIds: string[] = [];

  for (const skillId of sourceSkillIds) {
    const sourceDir = path.join(sourceSkillRoot, skillId);
    const runtimeDir = path.join(runtimeSkillRoot, skillId);

    if (!directoryExists(runtimeDir)) {
      missingSkillIds.push(skillId);
      continue;
    }

    let missingFile = false;
    let driftedFile = false;

    for (const relativePath of listRelativeFiles(sourceDir)) {
      const sourceFilePath = path.join(sourceDir, relativePath);
      const runtimeFilePath = path.join(runtimeDir, relativePath);

      if (!fileExists(runtimeFilePath)) {
        missingFile = true;
        continue;
      }

      const sourceContents = normalizeLineEndings(fs.readFileSync(sourceFilePath, 'utf8'));
      const runtimeContents = normalizeLineEndings(fs.readFileSync(runtimeFilePath, 'utf8'));

      if (sourceContents !== runtimeContents) {
        driftedFile = true;
      }
    }

    if (missingFile) {
      missingSkillIds.push(skillId);
      continue;
    }

    if (driftedFile) {
      driftedSkillIds.push(skillId);
    }
  }

  for (const recipeId of sourceRecipeIds) {
    const sourceRecipePath = path.join(sourceRecipeRoot, `${recipeId}.yaml.example`);
    const runtimeRecipePath = path.join(runtimeRecipeRoot, `${recipeId}.yaml`);

    if (!fileExists(runtimeRecipePath)) {
      missingRecipeIds.push(recipeId);
      continue;
    }

    const sourceContents = normalizeLineEndings(fs.readFileSync(sourceRecipePath, 'utf8'));
    const runtimeContents = normalizeLineEndings(fs.readFileSync(runtimeRecipePath, 'utf8'));

    if (sourceContents !== runtimeContents) {
      driftedRecipeIds.push(recipeId);
    }
  }

  return {
    distroDir,
    workingDir,
    runtimeSkillRoot,
    runtimeRecipeRoot,
    sourceSkillRoot,
    sourceRecipeRoot,
    sourceSkillIds,
    sourceRecipeIds,
    missingSkillIds,
    driftedSkillIds,
    missingRecipeIds,
    driftedRecipeIds,
  };
}
