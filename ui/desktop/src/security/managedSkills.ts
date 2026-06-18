import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import YAML from 'yaml';

export type ManagedSkillInvalidCode =
  | 'invalid_archive'
  | 'invalid_frontmatter'
  | 'missing_frontmatter'
  | 'missing_name'
  | 'missing_skill_md'
  | 'name_mismatch'
  | 'unsupported_source';

export type ManagedBundledSkillStatus =
  | 'bundled-security'
  | 'invalid'
  | 'local-override'
  | 'missing-runtime';

export type ManagedLocalSkillStatus = 'invalid' | 'local-custom' | 'local-override';

export type ManagedExistingSkillStatus =
  | 'bundled-security'
  | 'invalid'
  | 'local-custom'
  | 'local-override';

export interface ManagedBundledSkillRecord {
  declaredName?: string;
  id: string;
  description: string;
  runtimeDir: string;
  sourceDir: string;
  status: ManagedBundledSkillStatus;
  invalidCode?: ManagedSkillInvalidCode;
  invalidDetail?: string;
}

export interface ManagedLocalSkillRecord {
  declaredName?: string;
  id: string;
  description: string;
  runtimeDir: string;
  status: ManagedLocalSkillStatus;
  bundledSkillId?: string;
  invalidCode?: ManagedSkillInvalidCode;
  invalidDetail?: string;
}

export interface ManagedSkillsInventory {
  bundledSkillRoot?: string;
  bundledSkills: ManagedBundledSkillRecord[];
  localSkills: ManagedLocalSkillRecord[];
  runtimeSkillRoot: string;
  workingDir: string;
}

export interface ImportManagedSkillRequest {
  overwrite?: boolean;
  sourcePath: string;
  workingDir: string;
}

export type ImportManagedSkillResult =
  | {
      existingStatus: ManagedExistingSkillStatus;
      skillId: string;
      status: 'conflict';
      targetDir: string;
    }
  | {
      code: ManagedSkillInvalidCode;
      reason: string;
      status: 'invalid';
    }
  | {
      localStatus: Exclude<ManagedLocalSkillStatus, 'invalid'>;
      skillId: string;
      status: 'installed';
      targetDir: string;
    };

export interface DeleteManagedLocalSkillResult {
  removed: boolean;
  removedPath: string;
}

export interface RestoreBundledSkillResult {
  restored: boolean;
  targetDir: string;
}

interface ParsedSkillPackage {
  declaredName?: string;
  description: string;
  dirName: string;
  id: string;
  invalidCode?: ManagedSkillInvalidCode;
  invalidDetail?: string;
  rootDir: string;
  valid: boolean;
}

interface ResolveSkillSourceResult {
  cleanup?: () => void;
  sourceDir?: string;
  invalidCode?: ManagedSkillInvalidCode;
  invalidReason?: string;
}

const allowedRuntimeSkillExtraPrefixes = new Map<string, string[]>([
  ['wooyun-legacy', ['external/upstream']],
]);

function directoryExists(directoryPath: string): boolean {
  try {
    return fs.statSync(directoryPath).isDirectory();
  } catch {
    return false;
  }
}

function fileExists(filePath: string): boolean {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function isAllowedRuntimeSkillExtra(skillId: string, relativePath: string): boolean {
  const prefixes = allowedRuntimeSkillExtraPrefixes.get(skillId) ?? [];
  return prefixes.some(
    (prefix) => relativePath === prefix || relativePath.startsWith(`${prefix}${path.sep}`)
  );
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

function parseSkillPackage(skillDir: string, requireDirectoryNameMatch = true): ParsedSkillPackage {
  const dirName = path.basename(skillDir);
  const skillMdPath = path.join(skillDir, 'SKILL.md');

  if (!fileExists(skillMdPath)) {
    return {
      description: '',
      dirName,
      id: dirName,
      invalidCode: 'missing_skill_md',
      invalidDetail: 'SKILL.md is missing from the selected skill package.',
      rootDir: skillDir,
      valid: false,
    };
  }

  const rawContents = fs.readFileSync(skillMdPath, 'utf8').replace(/\r\n/g, '\n');
  const frontmatterMatch = rawContents.match(/^---\n([\s\S]*?)\n---(?:\n|$)/);

  if (!frontmatterMatch) {
    return {
      description: '',
      dirName,
      id: dirName,
      invalidCode: 'missing_frontmatter',
      invalidDetail: 'SKILL.md is missing YAML frontmatter.',
      rootDir: skillDir,
      valid: false,
    };
  }

  let parsedFrontmatter: unknown;
  try {
    parsedFrontmatter = YAML.parse(frontmatterMatch[1]);
  } catch (error) {
    return {
      description: '',
      dirName,
      id: dirName,
      invalidCode: 'invalid_frontmatter',
      invalidDetail: error instanceof Error ? error.message : String(error),
      rootDir: skillDir,
      valid: false,
    };
  }

  const frontmatter =
    parsedFrontmatter && typeof parsedFrontmatter === 'object'
      ? (parsedFrontmatter as Record<string, unknown>)
      : {};
  const declaredName = typeof frontmatter.name === 'string' ? frontmatter.name.trim() : '';
  const description =
    typeof frontmatter.description === 'string' ? frontmatter.description.trim() : '';

  if (!declaredName) {
    return {
      description,
      dirName,
      id: dirName,
      invalidCode: 'missing_name',
      invalidDetail: 'SKILL.md frontmatter must include a non-empty name field.',
      rootDir: skillDir,
      valid: false,
    };
  }

  if (requireDirectoryNameMatch && declaredName !== dirName) {
    return {
      declaredName,
      description,
      dirName,
      id: dirName,
      invalidCode: 'name_mismatch',
      invalidDetail: `Directory name "${dirName}" does not match SKILL.md name "${declaredName}".`,
      rootDir: skillDir,
      valid: false,
    };
  }

  return {
    declaredName,
    description,
    dirName,
    id: declaredName,
    rootDir: skillDir,
    valid: true,
  };
}

function collectSkillDirectories(rootDir: string): string[] {
  if (!directoryExists(rootDir)) {
    return [];
  }

  return fs
    .readdirSync(rootDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(rootDir, entry.name))
    .sort((left, right) => path.basename(left).localeCompare(path.basename(right)));
}

function directoriesMatch(sourceDir: string, runtimeDir: string, skillId: string): boolean {
  const sourceFiles = listRelativeFiles(sourceDir);
  const runtimeFiles = listRelativeFiles(runtimeDir).filter(
    (relativePath) => !isAllowedRuntimeSkillExtra(skillId, relativePath)
  );

  if (JSON.stringify(sourceFiles) !== JSON.stringify(runtimeFiles)) {
    return false;
  }

  return sourceFiles.every((relativePath) =>
    fs
      .readFileSync(path.join(sourceDir, relativePath))
      .equals(fs.readFileSync(path.join(runtimeDir, relativePath)))
  );
}

function collectCandidateSkillRoots(rootDir: string): string[] {
  if (fileExists(path.join(rootDir, 'SKILL.md'))) {
    return [rootDir];
  }

  const candidates: string[] = [];
  for (const entry of fs.readdirSync(rootDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }

    const childDir = path.join(rootDir, entry.name);
    candidates.push(...collectCandidateSkillRoots(childDir));
  }

  return candidates;
}

function extractZipArchive(zipPath: string, destinationDir: string): void {
  fs.mkdirSync(destinationDir, { recursive: true });

  try {
    execFileSync('ditto', ['-x', '-k', zipPath, destinationDir], { stdio: 'ignore' });
    return;
  } catch {
    execFileSync('unzip', ['-q', zipPath, '-d', destinationDir], { stdio: 'ignore' });
  }
}

async function resolveSkillSourceDirectory(
  sourcePath: string,
  extractZip: ((zipPath: string, destinationDir: string) => Promise<void> | void) | undefined
): Promise<ResolveSkillSourceResult> {
  let stats: fs.Stats;
  try {
    stats = fs.statSync(sourcePath);
  } catch {
    return {
      invalidCode: 'unsupported_source',
      invalidReason: `Selected path does not exist: ${sourcePath}`,
    };
  }

  if (stats.isDirectory()) {
    const candidates = collectCandidateSkillRoots(sourcePath);
    if (candidates.length === 1) {
      return { sourceDir: candidates[0] };
    }

    return {
      invalidCode: 'invalid_archive',
      invalidReason:
        candidates.length === 0
          ? 'No SKILL.md package was found in the selected folder.'
          : 'The selected folder contains multiple SKILL.md packages.',
    };
  }

  if (!stats.isFile() || path.extname(sourcePath).toLowerCase() !== '.zip') {
    return {
      invalidCode: 'unsupported_source',
      invalidReason: 'Only skill folders or .zip archives can be imported.',
    };
  }

  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'security-goose-skill-import-'));
  const destinationDir = path.join(tempRoot, 'unzipped');

  try {
    await (extractZip ?? extractZipArchive)(sourcePath, destinationDir);
    const candidates = collectCandidateSkillRoots(destinationDir);
    if (candidates.length !== 1) {
      fs.rmSync(tempRoot, { recursive: true, force: true });
      return {
        invalidCode: 'invalid_archive',
        invalidReason:
          candidates.length === 0
            ? 'The selected zip does not contain a valid SKILL.md package.'
            : 'The selected zip contains multiple SKILL.md packages.',
      };
    }

    return {
      cleanup: () => {
        fs.rmSync(tempRoot, { recursive: true, force: true });
      },
      sourceDir: candidates[0],
    };
  } catch (error) {
    fs.rmSync(tempRoot, { recursive: true, force: true });
    return {
      invalidCode: 'invalid_archive',
      invalidReason: error instanceof Error ? error.message : String(error),
    };
  }
}

function getRuntimeSkillRoot(workingDir: string): string {
  return path.join(workingDir, '.agents', 'skills');
}

function getBundledSkillDirectory(bundledSkillRoot: string | undefined, skillId: string): string | undefined {
  if (!bundledSkillRoot) {
    return undefined;
  }

  const bundledDir = path.join(bundledSkillRoot, skillId);
  return fileExists(path.join(bundledDir, 'SKILL.md')) ? bundledDir : undefined;
}

export function listManagedSkillsInventory(options: {
  bundledSkillRoot?: string;
  workingDir: string;
}): ManagedSkillsInventory {
  const { bundledSkillRoot, workingDir } = options;
  const runtimeSkillRoot = getRuntimeSkillRoot(workingDir);
  const bundledPackages = collectSkillDirectories(bundledSkillRoot ?? '')
    .map((skillDir) => parseSkillPackage(skillDir))
    .filter((skillPackage) => skillPackage.valid);
  const bundledById = new Map(bundledPackages.map((skillPackage) => [skillPackage.id, skillPackage]));
  const runtimePackages = collectSkillDirectories(runtimeSkillRoot).map((skillDir) =>
    parseSkillPackage(skillDir)
  );
  const runtimeByDirName = new Map(runtimePackages.map((skillPackage) => [skillPackage.dirName, skillPackage]));

  const bundledSkills = bundledPackages
    .map<ManagedBundledSkillRecord>((bundledSkill) => {
      const runtimeSkill = runtimeByDirName.get(bundledSkill.id);
      const runtimeDir = path.join(runtimeSkillRoot, bundledSkill.id);

      if (!runtimeSkill || !directoryExists(runtimeDir)) {
        return {
          declaredName: bundledSkill.declaredName,
          description: bundledSkill.description,
          id: bundledSkill.id,
          runtimeDir,
          sourceDir: bundledSkill.rootDir,
          status: 'missing-runtime',
        };
      }

      if (!runtimeSkill.valid) {
        return {
          declaredName: bundledSkill.declaredName,
          description: bundledSkill.description,
          id: bundledSkill.id,
          runtimeDir,
          sourceDir: bundledSkill.rootDir,
          status: 'invalid',
          invalidCode: runtimeSkill.invalidCode,
          invalidDetail: runtimeSkill.invalidDetail,
        };
      }

      return {
        declaredName: bundledSkill.declaredName,
        description: bundledSkill.description,
        id: bundledSkill.id,
        runtimeDir,
        sourceDir: bundledSkill.rootDir,
        status: directoriesMatch(bundledSkill.rootDir, runtimeDir, bundledSkill.id)
          ? 'bundled-security'
          : 'local-override',
      };
    })
    .sort((left, right) => left.id.localeCompare(right.id));

  const localSkills = runtimePackages
    .flatMap<ManagedLocalSkillRecord>((runtimeSkill) => {
      const bundledSkill = bundledById.get(runtimeSkill.id) ?? bundledById.get(runtimeSkill.dirName);

      if (!runtimeSkill.valid) {
        return [
          {
            bundledSkillId: bundledSkill?.id,
            declaredName: runtimeSkill.declaredName,
            description: runtimeSkill.description,
            id: runtimeSkill.id,
            invalidCode: runtimeSkill.invalidCode,
            invalidDetail: runtimeSkill.invalidDetail,
            runtimeDir: runtimeSkill.rootDir,
            status: 'invalid',
          },
        ];
      }

      if (!bundledSkill) {
        return [
          {
            declaredName: runtimeSkill.declaredName,
            description: runtimeSkill.description,
            id: runtimeSkill.id,
            runtimeDir: runtimeSkill.rootDir,
            status: 'local-custom',
          },
        ];
      }

      if (!directoriesMatch(bundledSkill.rootDir, runtimeSkill.rootDir, bundledSkill.id)) {
        return [
          {
            bundledSkillId: bundledSkill.id,
            declaredName: runtimeSkill.declaredName,
            description: runtimeSkill.description || bundledSkill.description,
            id: runtimeSkill.id,
            runtimeDir: runtimeSkill.rootDir,
            status: 'local-override',
          },
        ];
      }

      return [];
    })
    .sort((left, right) => left.id.localeCompare(right.id));

  return {
    bundledSkillRoot,
    bundledSkills,
    localSkills,
    runtimeSkillRoot,
    workingDir,
  };
}

export function getManagedLocalVisibleSkillNames(inventory: ManagedSkillsInventory): string[] {
  return inventory.localSkills
    .filter((skill) => skill.status !== 'invalid')
    .map((skill) => skill.id)
    .sort();
}

export async function importManagedSkill(
  request: ImportManagedSkillRequest & {
    bundledSkillRoot?: string;
    extractZip?: (zipPath: string, destinationDir: string) => Promise<void> | void;
  }
): Promise<ImportManagedSkillResult> {
  const { bundledSkillRoot, extractZip, overwrite = false, sourcePath, workingDir } = request;
  const runtimeSkillRoot = getRuntimeSkillRoot(workingDir);
  fs.mkdirSync(runtimeSkillRoot, { recursive: true });

  const resolved = await resolveSkillSourceDirectory(sourcePath, extractZip);
  if (!resolved.sourceDir) {
    return {
      code: resolved.invalidCode ?? 'unsupported_source',
      reason: resolved.invalidReason ?? 'Unable to resolve a valid skill package from the selection.',
      status: 'invalid',
    };
  }

  try {
    const sourceSkill = parseSkillPackage(resolved.sourceDir, false);
    if (!sourceSkill.valid) {
      return {
        code: sourceSkill.invalidCode ?? 'invalid_frontmatter',
        reason: sourceSkill.invalidDetail ?? 'Selected skill package is invalid.',
        status: 'invalid',
      };
    }

    const targetDir = path.join(runtimeSkillRoot, sourceSkill.id);
    const inventory = listManagedSkillsInventory({ bundledSkillRoot, workingDir });
    const existingLocalSkill = inventory.localSkills.find((skill) => skill.id === sourceSkill.id);
    const existingBundledSkill = inventory.bundledSkills.find((skill) => skill.id === sourceSkill.id);

    let existingStatus: ManagedExistingSkillStatus | undefined;
    if (existingLocalSkill) {
      existingStatus = existingLocalSkill.status;
    } else if (existingBundledSkill && existingBundledSkill.status !== 'missing-runtime') {
      existingStatus = existingBundledSkill.status;
    } else if (directoryExists(targetDir)) {
      existingStatus = 'invalid';
    }

    if (existingStatus && !overwrite) {
      return {
        existingStatus,
        skillId: sourceSkill.id,
        status: 'conflict',
        targetDir,
      };
    }

    if (directoryExists(targetDir)) {
      fs.rmSync(targetDir, { recursive: true, force: true });
    }

    fs.cpSync(resolved.sourceDir, targetDir, { recursive: true });

    return {
      localStatus: getBundledSkillDirectory(bundledSkillRoot, sourceSkill.id)
        ? 'local-override'
        : 'local-custom',
      skillId: sourceSkill.id,
      status: 'installed',
      targetDir,
    };
  } finally {
    resolved.cleanup?.();
  }
}

export function deleteManagedLocalSkill(options: {
  skillId: string;
  workingDir: string;
}): DeleteManagedLocalSkillResult {
  const removedPath = path.join(getRuntimeSkillRoot(options.workingDir), options.skillId);
  fs.rmSync(removedPath, { recursive: true, force: true });

  return {
    removed: true,
    removedPath,
  };
}

export function restoreBundledSkill(options: {
  bundledSkillRoot?: string;
  skillId: string;
  workingDir: string;
}): RestoreBundledSkillResult {
  const bundledSkillDir = getBundledSkillDirectory(options.bundledSkillRoot, options.skillId);
  if (!bundledSkillDir) {
    throw new Error(`Bundled skill "${options.skillId}" is unavailable.`);
  }

  const runtimeSkillRoot = getRuntimeSkillRoot(options.workingDir);
  const targetDir = path.join(runtimeSkillRoot, options.skillId);
  fs.mkdirSync(runtimeSkillRoot, { recursive: true });
  fs.rmSync(targetDir, { recursive: true, force: true });
  fs.cpSync(bundledSkillDir, targetDir, { recursive: true });

  return {
    restored: true,
    targetDir,
  };
}
