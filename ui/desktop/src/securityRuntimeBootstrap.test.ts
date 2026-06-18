import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import {
  inspectBundledSecurityRuntimeAssets,
  seedBundledSecurityRuntimeAssets,
} from './securityRuntimeBootstrap';

describe('seedBundledSecurityRuntimeAssets', () => {
  const tempRoots: string[] = [];

  afterEach(() => {
    for (const tempRoot of tempRoots.splice(0)) {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });

  const makeTempRoot = (): string => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'security-runtime-bootstrap-'));
    tempRoots.push(tempRoot);
    return tempRoot;
  };

  it('skips outside packaged mode', () => {
    expect(
      seedBundledSecurityRuntimeAssets({
        isPackaged: false,
        distroDir: '/unused',
        workingDir: '/unused',
      })
    ).toEqual({
      seededSkillDirs: [],
      seededRecipeFiles: [],
      skippedReason: 'not_packaged',
    });
  });

  it('seeds bundled skills and recipes into Goose-native working directory roots', () => {
    const tempRoot = makeTempRoot();
    const distroDir = path.join(tempRoot, 'security-cn');
    const workingDir = path.join(tempRoot, 'workdir');

    fs.mkdirSync(path.join(distroDir, 'branding'), { recursive: true });
    fs.writeFileSync(path.join(distroDir, 'branding', 'product-metadata.json'), '{}\n');
    fs.mkdirSync(path.join(distroDir, 'skills', 'vuln-triage'), { recursive: true });
    fs.writeFileSync(path.join(distroDir, 'skills', 'vuln-triage', 'SKILL.md'), '# skill\n');
    fs.writeFileSync(path.join(distroDir, 'skills', 'vuln-triage', 'guide.md'), 'guide\n');
    fs.mkdirSync(path.join(distroDir, 'recipes'), { recursive: true });
    fs.writeFileSync(
      path.join(distroDir, 'recipes', 'security-vuln-triage.yaml.example'),
      'title: Security Vuln Triage\n'
    );
    fs.mkdirSync(workingDir, { recursive: true });

    const result = seedBundledSecurityRuntimeAssets({
      isPackaged: true,
      distroDir,
      workingDir,
    });

    expect(result.seededSkillDirs).toEqual(['vuln-triage']);
    expect(result.seededRecipeFiles).toEqual(['security-vuln-triage.yaml']);
    expect(
      fs.readFileSync(path.join(workingDir, '.agents', 'skills', 'vuln-triage', 'SKILL.md'), 'utf8')
    ).toBe('# skill\n');
    expect(
      fs.readFileSync(
        path.join(workingDir, '.goose', 'recipes', 'security-vuln-triage.yaml'),
        'utf8'
      )
    ).toBe('title: Security Vuln Triage\n');
  });

  it('preserves user-modified runtime assets on repeated packaged launches', () => {
    const tempRoot = makeTempRoot();
    const distroDir = path.join(tempRoot, 'security-cn');
    const workingDir = path.join(tempRoot, 'workdir');

    fs.mkdirSync(path.join(distroDir, 'branding'), { recursive: true });
    fs.writeFileSync(path.join(distroDir, 'branding', 'product-metadata.json'), '{}\n');
    fs.mkdirSync(path.join(distroDir, 'skills', 'report-writing'), { recursive: true });
    fs.writeFileSync(path.join(distroDir, 'skills', 'report-writing', 'SKILL.md'), '# packaged\n');
    fs.mkdirSync(path.join(distroDir, 'recipes'), { recursive: true });
    fs.writeFileSync(
      path.join(distroDir, 'recipes', 'alert-investigation.yaml.example'),
      'title: Packaged Alert Investigation\n'
    );
    fs.mkdirSync(workingDir, { recursive: true });

    seedBundledSecurityRuntimeAssets({
      isPackaged: true,
      distroDir,
      workingDir,
    });

    fs.writeFileSync(
      path.join(workingDir, '.agents', 'skills', 'report-writing', 'SKILL.md'),
      '# user override\n'
    );
    fs.writeFileSync(
      path.join(workingDir, '.goose', 'recipes', 'alert-investigation.yaml'),
      'title: User Override\n'
    );
    fs.writeFileSync(
      path.join(distroDir, 'skills', 'report-writing', 'SKILL.md'),
      '# packaged v2\n'
    );
    fs.writeFileSync(
      path.join(distroDir, 'recipes', 'alert-investigation.yaml.example'),
      'title: Packaged Alert Investigation V2\n'
    );

    const result = seedBundledSecurityRuntimeAssets({
      isPackaged: true,
      distroDir,
      workingDir,
    });

    expect(result.seededSkillDirs).toEqual([]);
    expect(result.seededRecipeFiles).toEqual([]);
    expect(
      fs.readFileSync(
        path.join(workingDir, '.agents', 'skills', 'report-writing', 'SKILL.md'),
        'utf8'
      )
    ).toBe('# user override\n');
    expect(
      fs.readFileSync(
        path.join(workingDir, '.goose', 'recipes', 'alert-investigation.yaml'),
        'utf8'
      )
    ).toBe('title: User Override\n');
  });

  it('reports missing and drifted runtime assets against the bundled security source', () => {
    const tempRoot = makeTempRoot();
    const distroDir = path.join(tempRoot, 'security-cn');
    const workingDir = path.join(tempRoot, 'workdir');

    fs.mkdirSync(path.join(distroDir, 'branding'), { recursive: true });
    fs.writeFileSync(path.join(distroDir, 'branding', 'product-metadata.json'), '{}\n');
    fs.mkdirSync(path.join(distroDir, 'skills', 'vuln-triage'), { recursive: true });
    fs.writeFileSync(path.join(distroDir, 'skills', 'vuln-triage', 'SKILL.md'), '# vuln\n');
    fs.mkdirSync(path.join(distroDir, 'skills', 'report-writing'), { recursive: true });
    fs.writeFileSync(path.join(distroDir, 'skills', 'report-writing', 'SKILL.md'), '# report\n');
    fs.mkdirSync(path.join(distroDir, 'recipes'), { recursive: true });
    fs.writeFileSync(
      path.join(distroDir, 'recipes', 'security-vuln-triage.yaml.example'),
      'title: Vuln\n'
    );
    fs.writeFileSync(path.join(distroDir, 'recipes', 'web-investigation.yaml.example'), 'title: Web\n');

    fs.mkdirSync(path.join(workingDir, '.agents', 'skills', 'report-writing'), { recursive: true });
    fs.writeFileSync(
      path.join(workingDir, '.agents', 'skills', 'report-writing', 'SKILL.md'),
      '# report override\n'
    );
    fs.mkdirSync(path.join(workingDir, '.goose', 'recipes'), { recursive: true });
    fs.writeFileSync(
      path.join(workingDir, '.goose', 'recipes', 'security-vuln-triage.yaml'),
      'title: Vuln override\n'
    );
    fs.mkdirSync(workingDir, { recursive: true });

    const diagnostics = inspectBundledSecurityRuntimeAssets({
      distroDir,
      workingDir,
    });

    expect(diagnostics.sourceSkillIds).toEqual(['report-writing', 'vuln-triage']);
    expect(diagnostics.sourceRecipeIds).toEqual(['security-vuln-triage', 'web-investigation']);
    expect(diagnostics.missingSkillIds).toEqual(['vuln-triage']);
    expect(diagnostics.driftedSkillIds).toEqual(['report-writing']);
    expect(diagnostics.missingRecipeIds).toEqual(['web-investigation']);
    expect(diagnostics.driftedRecipeIds).toEqual(['security-vuln-triage']);
  });
});
