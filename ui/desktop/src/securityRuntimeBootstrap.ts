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

  if (directoryExists(sourceSkillRoot)) {
    const skillDirs = fs
      .readdirSync(sourceSkillRoot, { withFileTypes: true })
      .filter((entry) => entry.isDirectory())
      .map((entry) => entry.name)
      .filter((skillDir) => fileExists(path.join(sourceSkillRoot, skillDir, 'SKILL.md')));

    for (const skillDir of skillDirs) {
      if (
        copyDirectoryMissing(
          path.join(sourceSkillRoot, skillDir),
          path.join(runtimeSkillRoot, skillDir)
        )
      ) {
        seededSkillDirs.push(skillDir);
      }
    }
  }

  if (directoryExists(sourceRecipeRoot)) {
    const recipeFiles = fs
      .readdirSync(sourceRecipeRoot, { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.endsWith('.yaml.example'))
      .map((entry) => entry.name);

    for (const recipeFile of recipeFiles) {
      const runtimeRecipeFile = recipeFile.replace(/\.example$/, '');
      if (
        copyFileIfMissing(
          path.join(sourceRecipeRoot, recipeFile),
          path.join(runtimeRecipeRoot, runtimeRecipeFile)
        )
      ) {
        seededRecipeFiles.push(runtimeRecipeFile);
      }
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
