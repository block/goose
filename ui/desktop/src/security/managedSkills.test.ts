/**
 * @vitest-environment node
 */
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import {
  deleteManagedLocalSkill,
  importManagedSkill,
  listManagedSkillsInventory,
  restoreBundledSkill,
} from './managedSkills';

function writeSkillPackage(skillRoot: string, skillName: string, description = `${skillName} desc`) {
  fs.mkdirSync(skillRoot, { recursive: true });
  fs.writeFileSync(
    path.join(skillRoot, 'SKILL.md'),
    `---\nname: ${skillName}\ndescription: ${description}\n---\n\n# ${skillName}\n`
  );
}

describe('managedSkills', () => {
  const tempRoots: string[] = [];

  afterEach(() => {
    for (const tempRoot of tempRoots.splice(0)) {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  const makeTempRoot = (): string => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'managed-skills-'));
    tempRoots.push(tempRoot);
    return tempRoot;
  };

  it('classifies bundled, local override, local custom, invalid, and missing runtime skills', () => {
    const tempRoot = makeTempRoot();
    const distroDir = path.join(tempRoot, 'distro');
    const workingDir = path.join(tempRoot, 'workdir');
    const bundledRoot = path.join(distroDir, 'skills');
    const runtimeRoot = path.join(workingDir, '.agents', 'skills');

    writeSkillPackage(path.join(bundledRoot, 'vuln-triage'), 'vuln-triage');
    writeSkillPackage(path.join(bundledRoot, 'report-writing'), 'report-writing');
    writeSkillPackage(path.join(bundledRoot, 'ioc-analysis'), 'ioc-analysis');
    writeSkillPackage(path.join(bundledRoot, 'wooyun-legacy'), 'wooyun-legacy');

    fs.mkdirSync(runtimeRoot, { recursive: true });
    fs.cpSync(path.join(bundledRoot, 'vuln-triage'), path.join(runtimeRoot, 'vuln-triage'), {
      recursive: true,
    });
    fs.cpSync(path.join(bundledRoot, 'report-writing'), path.join(runtimeRoot, 'report-writing'), {
      recursive: true,
    });
    fs.writeFileSync(
      path.join(runtimeRoot, 'report-writing', 'SKILL.md'),
      '---\nname: report-writing\ndescription: overridden\n---\n\n# override\n'
    );
    writeSkillPackage(path.join(runtimeRoot, 'custom-investigation'), 'custom-investigation');
    fs.mkdirSync(path.join(runtimeRoot, 'broken-package'), { recursive: true });
    fs.cpSync(path.join(bundledRoot, 'wooyun-legacy'), path.join(runtimeRoot, 'wooyun-legacy'), {
      recursive: true,
    });
    fs.mkdirSync(path.join(runtimeRoot, 'wooyun-legacy', 'external', 'upstream'), {
      recursive: true,
    });
    fs.writeFileSync(
      path.join(runtimeRoot, 'wooyun-legacy', 'external', 'upstream', 'references.md'),
      '# upstream\n'
    );

    const inventory = listManagedSkillsInventory({
      bundledSkillRoot: bundledRoot,
      workingDir,
    });

    expect(inventory.bundledSkills).toEqual([
      expect.objectContaining({ id: 'ioc-analysis', status: 'missing-runtime' }),
      expect.objectContaining({ id: 'report-writing', status: 'local-override' }),
      expect.objectContaining({ id: 'vuln-triage', status: 'bundled-security' }),
      expect.objectContaining({ id: 'wooyun-legacy', status: 'bundled-security' }),
    ]);
    expect(inventory.localSkills).toEqual([
      expect.objectContaining({ id: 'broken-package', status: 'invalid' }),
      expect.objectContaining({ id: 'custom-investigation', status: 'local-custom' }),
      expect.objectContaining({
        id: 'report-writing',
        status: 'local-override',
        bundledSkillId: 'report-writing',
      }),
    ]);
  });

  it('installs a local skill folder into the current project using the frontmatter name', async () => {
    const tempRoot = makeTempRoot();
    const workingDir = path.join(tempRoot, 'workdir');
    const sourceDir = path.join(tempRoot, 'uploaded-folder');

    writeSkillPackage(sourceDir, 'local-collector', 'Local collector');

    const result = await importManagedSkill({
      workingDir,
      sourcePath: sourceDir,
    });

    expect(result).toEqual(
      expect.objectContaining({
        status: 'installed',
        skillId: 'local-collector',
        localStatus: 'local-custom',
      })
    );
    expect(
      fs.existsSync(path.join(workingDir, '.agents', 'skills', 'local-collector', 'SKILL.md'))
    ).toBe(true);
  });

  it('installs a local skill zip into the current project', async () => {
    const tempRoot = makeTempRoot();
    const workingDir = path.join(tempRoot, 'workdir');
    const extractedDir = path.join(tempRoot, 'zip-contents', 'archive-root');
    const zipPath = path.join(tempRoot, 'skill.zip');

    writeSkillPackage(extractedDir, 'zip-loader', 'Zip loader');
    fs.writeFileSync(zipPath, 'stub zip');

    const result = await importManagedSkill({
      workingDir,
      sourcePath: zipPath,
      extractZip: async (_archivePath, destinationDir) => {
        fs.mkdirSync(destinationDir, { recursive: true });
        fs.cpSync(path.join(tempRoot, 'zip-contents'), destinationDir, { recursive: true });
      },
    });

    expect(result).toEqual(
      expect.objectContaining({
        status: 'installed',
        skillId: 'zip-loader',
        localStatus: 'local-custom',
      })
    );
    expect(
      fs.existsSync(path.join(workingDir, '.agents', 'skills', 'zip-loader', 'SKILL.md'))
    ).toBe(true);
  });

  it('requires confirmation before overriding a bundled security skill and then installs the override', async () => {
    const tempRoot = makeTempRoot();
    const distroDir = path.join(tempRoot, 'distro');
    const bundledRoot = path.join(distroDir, 'skills');
    const workingDir = path.join(tempRoot, 'workdir');
    const runtimeRoot = path.join(workingDir, '.agents', 'skills');
    const sourceDir = path.join(tempRoot, 'uploaded-folder');

    writeSkillPackage(path.join(bundledRoot, 'vuln-triage'), 'vuln-triage', 'Bundled');
    fs.mkdirSync(runtimeRoot, { recursive: true });
    fs.cpSync(path.join(bundledRoot, 'vuln-triage'), path.join(runtimeRoot, 'vuln-triage'), {
      recursive: true,
    });
    writeSkillPackage(sourceDir, 'vuln-triage', 'Override');
    fs.writeFileSync(path.join(sourceDir, 'guide.md'), 'override guide\n');

    const conflict = await importManagedSkill({
      bundledSkillRoot: bundledRoot,
      workingDir,
      sourcePath: sourceDir,
    });

    expect(conflict).toEqual(
      expect.objectContaining({
        status: 'conflict',
        skillId: 'vuln-triage',
        existingStatus: 'bundled-security',
      })
    );

    const installed = await importManagedSkill({
      bundledSkillRoot: bundledRoot,
      workingDir,
      sourcePath: sourceDir,
      overwrite: true,
    });

    expect(installed).toEqual(
      expect.objectContaining({
        status: 'installed',
        skillId: 'vuln-triage',
        localStatus: 'local-override',
      })
    );
    expect(
      fs.readFileSync(path.join(runtimeRoot, 'vuln-triage', 'guide.md'), 'utf8')
    ).toBe('override guide\n');
  });

  it('deletes local custom skills and restores bundled overrides from the bundled source', async () => {
    const tempRoot = makeTempRoot();
    const distroDir = path.join(tempRoot, 'distro');
    const bundledRoot = path.join(distroDir, 'skills');
    const workingDir = path.join(tempRoot, 'workdir');
    const runtimeRoot = path.join(workingDir, '.agents', 'skills');

    writeSkillPackage(path.join(bundledRoot, 'report-writing'), 'report-writing', 'Bundled report');
    writeSkillPackage(path.join(runtimeRoot, 'custom-investigation'), 'custom-investigation');
    fs.cpSync(path.join(bundledRoot, 'report-writing'), path.join(runtimeRoot, 'report-writing'), {
      recursive: true,
    });
    fs.writeFileSync(
      path.join(runtimeRoot, 'report-writing', 'SKILL.md'),
      '---\nname: report-writing\ndescription: override\n---\n\n# override\n'
    );

    expect(deleteManagedLocalSkill({ workingDir, skillId: 'custom-investigation' })).toEqual({
      removed: true,
      removedPath: path.join(runtimeRoot, 'custom-investigation'),
    });
    expect(fs.existsSync(path.join(runtimeRoot, 'custom-investigation'))).toBe(false);

    const restored = restoreBundledSkill({
      bundledSkillRoot: bundledRoot,
      workingDir,
      skillId: 'report-writing',
    });

    expect(restored).toEqual(
      expect.objectContaining({
        restored: true,
        targetDir: path.join(runtimeRoot, 'report-writing'),
      })
    );
    expect(
      fs.readFileSync(path.join(runtimeRoot, 'report-writing', 'SKILL.md'), 'utf8')
    ).toContain('Bundled report');
    expect(
      listManagedSkillsInventory({
        bundledSkillRoot: bundledRoot,
        workingDir,
      }).localSkills
    ).toEqual([]);
  });
});
