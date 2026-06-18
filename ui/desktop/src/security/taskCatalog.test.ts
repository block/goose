import { describe, expect, it } from 'vitest';
import {
  collectAvailableRecipeIds,
  collectAvailableRecipeRuntimeIds,
  getSecurityTaskIdForRecipeManifest,
  SECURITY_TASK_IDS,
  SECURITY_TASKS,
  getSecurityTaskById,
  resolveSecurityTaskLaunchConfig,
  resolveSecurityTaskLaunchConfigForRecipeManifest,
} from './taskCatalog';

describe('security task catalog', () => {
  it('defines the six curated Goal 6 security task entry points', () => {
    expect(SECURITY_TASK_IDS).toEqual([
      'vuln-triage',
      'alert-investigation',
      'ioc-analysis',
      'web-investigation',
      'report-writing',
      'wooyun-legacy',
    ]);

    expect(SECURITY_TASKS).toHaveLength(6);
  });

  it('maps recipe-backed tasks to existing Goose recipe ids when available', () => {
    const availableRecipeIds = new Set([
      'security-vuln-triage',
      'alert-investigation',
      'ioc-analysis',
      'report-writing',
      'web-investigation',
      'wooyun-legacy',
    ]);

    expect(resolveSecurityTaskLaunchConfig('vuln-triage', 'zh-CN', availableRecipeIds)).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      primaryPath: 'recipe',
      preferredRecipeId: 'security-vuln-triage',
      recipeId: 'security-vuln-triage',
    });

    expect(
      resolveSecurityTaskLaunchConfig('alert-investigation', 'zh-CN', availableRecipeIds)
    ).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      primaryPath: 'recipe',
      preferredRecipeId: 'alert-investigation',
      recipeId: 'alert-investigation',
    });

    expect(resolveSecurityTaskLaunchConfig('web-investigation', 'zh-CN', availableRecipeIds)).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      primaryPath: 'recipe',
      preferredRecipeId: 'web-investigation',
      recipeId: 'web-investigation',
    });

    expect(resolveSecurityTaskLaunchConfig('ioc-analysis', 'zh-CN', availableRecipeIds)).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      primaryPath: 'recipe',
      preferredRecipeId: 'ioc-analysis',
      recipeId: 'ioc-analysis',
    });

    expect(resolveSecurityTaskLaunchConfig('report-writing', 'zh-CN', availableRecipeIds)).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      primaryPath: 'recipe',
      preferredRecipeId: 'report-writing',
      recipeId: 'report-writing',
    });

    expect(resolveSecurityTaskLaunchConfig('wooyun-legacy', 'zh-CN', availableRecipeIds)).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      primaryPath: 'recipe',
      preferredRecipeId: 'wooyun-legacy',
      recipeId: 'wooyun-legacy',
    });
  });

  it('falls back to skill-guided prompt preview when no recipe is available', () => {
    const noRecipes = new Set<string>();

    expect(resolveSecurityTaskLaunchConfig('vuln-triage', 'en', noRecipes)).toMatchObject({
      launchMode: 'prompt',
      availability: 'preview',
      primaryPath: 'recipe',
      preferredRecipeId: 'security-vuln-triage',
      recipeId: undefined,
    });

    expect(resolveSecurityTaskLaunchConfig('ioc-analysis', 'zh-CN', noRecipes)).toMatchObject({
      launchMode: 'prompt',
      availability: 'preview',
      primaryPath: 'recipe',
      preferredRecipeId: 'ioc-analysis',
      recipeId: undefined,
      skillId: 'ioc-analysis',
    });
  });

  it('treats recipe file stems as available runtime ids for security task mapping', () => {
    const availableRecipeIds = collectAvailableRecipeIds([
      {
        id: 'd698463cf7ebcf7f',
        file_path: '/tmp/security-goose/.goose/recipes/security-vuln-triage.yaml',
      },
      {
        id: '50f71ea65c57f100',
        file_path: '/tmp/security-goose/.goose/recipes/alert-investigation.yaml',
      },
    ]);

    expect(availableRecipeIds.has('d698463cf7ebcf7f')).toBe(true);
    expect(availableRecipeIds.has('security-vuln-triage')).toBe(true);
    expect(availableRecipeIds.has('alert-investigation')).toBe(true);

    const runtimeRecipeIds = collectAvailableRecipeRuntimeIds([
      {
        id: 'd698463cf7ebcf7f',
        file_path: '/tmp/security-goose/.goose/recipes/security-vuln-triage.yaml',
      },
    ]);

    expect(runtimeRecipeIds.get('security-vuln-triage')).toBe('d698463cf7ebcf7f');

    expect(
      resolveSecurityTaskLaunchConfig('vuln-triage', 'zh-CN', availableRecipeIds)
    ).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      recipeId: 'security-vuln-triage',
    });
  });

  it('maps saved recipe manifests back to the curated security task catalog', () => {
    const runtimeManifest = {
      id: 'd698463cf7ebcf7f',
      file_path: '/tmp/security-goose/.goose/recipes/security-vuln-triage.yaml',
    };

    expect(getSecurityTaskIdForRecipeManifest(runtimeManifest)).toBe('vuln-triage');

    expect(
      resolveSecurityTaskLaunchConfigForRecipeManifest(
        runtimeManifest,
        'en',
        new Set(['security-vuln-triage'])
      )
    ).toMatchObject({
      recipeId: 'security-vuln-triage',
      preferredRecipeId: 'security-vuln-triage',
      launchMode: 'recipe',
      availability: 'ready',
    });
  });

  it('generates localized starter prompts for guided chat tasks', () => {
    const availableRecipeIds = new Set(['report-writing']);
    const zhPrompt = resolveSecurityTaskLaunchConfig('report-writing', 'zh-CN', availableRecipeIds)
      .starterPrompt;
    const enPrompt = resolveSecurityTaskLaunchConfig('report-writing', 'en', availableRecipeIds)
      .starterPrompt;

    expect(zhPrompt).toContain('report-writing recipe');
    expect(zhPrompt).toContain('核心结论');
    expect(zhPrompt).toContain('待确认项');
    expect(enPrompt).toContain('report-writing recipe');
    expect(enPrompt).toContain('Core conclusions');
    expect(enPrompt).toContain('Open questions');
  });

  it('uses recipe-primary starter prompts when the mapped recipe is active', () => {
    const availableRecipeIds = new Set(['security-vuln-triage', 'alert-investigation', 'web-investigation']);

    const vulnPrompt = resolveSecurityTaskLaunchConfig(
      'vuln-triage',
      'en',
      availableRecipeIds
    ).starterPrompt;
    const alertPrompt = resolveSecurityTaskLaunchConfig(
      'alert-investigation',
      'en',
      availableRecipeIds
    ).starterPrompt;

    expect(vulnPrompt).toContain('This task already runs on the security-vuln-triage recipe');
    expect(vulnPrompt).not.toContain('if it is available in this session');
    expect(alertPrompt).toContain('Missing data');
    expect(alertPrompt).toContain('alert-investigation recipe as the primary workflow');
  });

  it('uses fallback prompt wording when a recipe-backed task loses its runtime recipe', () => {
    const prompt = resolveSecurityTaskLaunchConfig('web-investigation', 'en', new Set()).starterPrompt;

    expect(prompt).toContain('recipe runtime is unavailable in this workspace');
    expect(prompt).toContain('primary methodology');
  });

  it('keeps recipe-backed prompts aligned with the former skill output sections', () => {
    const availableRecipeIds = new Set(['ioc-analysis', 'report-writing', 'wooyun-legacy']);
    const iocPrompt = resolveSecurityTaskLaunchConfig('ioc-analysis', 'en', availableRecipeIds)
      .starterPrompt;
    const reportPrompt = resolveSecurityTaskLaunchConfig('report-writing', 'en', availableRecipeIds)
      .starterPrompt;
    const wooyunPrompt = resolveSecurityTaskLaunchConfig('wooyun-legacy', 'en', availableRecipeIds)
      .starterPrompt;

    expect(iocPrompt).toContain('IOC summary');
    expect(iocPrompt).toContain('Key findings');
    expect(iocPrompt).toContain('Linked entities');
    expect(reportPrompt).toContain('Background');
    expect(reportPrompt).toContain('Action items');
    expect(wooyunPrompt).toContain('Execution mode');
    expect(wooyunPrompt).toContain('Key evidence');
    expect(wooyunPrompt).toContain('distinguish direct evidence from historical analogy');
  });

  it('exposes task metadata for WooYun-style investigation', () => {
    const task = getSecurityTaskById('wooyun-legacy');
    expect(task.skillId).toBe('wooyun-legacy');
    expect(task.recipeId).toBe('wooyun-legacy');
  });

  it('tracks recommended security extensions without creating a parallel task layer', () => {
    expect(getSecurityTaskById('vuln-triage').recommendedExtensionIds).toEqual(['aiseesec-mcp']);
    expect(getSecurityTaskById('alert-investigation').recommendedExtensionIds).toEqual([
      'threat-intel-mcp',
      'local-security-gateway-mcp',
    ]);
    expect(getSecurityTaskById('ioc-analysis').recommendedExtensionIds).toEqual([
      'threat-intel-mcp',
    ]);
    expect(getSecurityTaskById('web-investigation').recommendedExtensionIds).toEqual([
      'browser-assist-mcp',
      'threat-intel-mcp',
    ]);
  });
});
