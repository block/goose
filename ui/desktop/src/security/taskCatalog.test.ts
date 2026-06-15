import { describe, expect, it } from 'vitest';
import {
  collectAvailableRecipeIds,
  collectAvailableRecipeRuntimeIds,
  SECURITY_TASK_IDS,
  SECURITY_TASKS,
  getSecurityTaskById,
  resolveSecurityTaskLaunchConfig,
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
      'web-investigation',
    ]);

    expect(resolveSecurityTaskLaunchConfig('vuln-triage', 'zh-CN', availableRecipeIds)).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      recipeId: 'security-vuln-triage',
    });

    expect(
      resolveSecurityTaskLaunchConfig('alert-investigation', 'zh-CN', availableRecipeIds)
    ).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      recipeId: 'alert-investigation',
    });

    expect(resolveSecurityTaskLaunchConfig('web-investigation', 'zh-CN', availableRecipeIds)).toMatchObject({
      launchMode: 'recipe',
      availability: 'ready',
      recipeId: 'web-investigation',
    });
  });

  it('falls back to skill-guided prompt preview when no recipe is available', () => {
    const noRecipes = new Set<string>();

    expect(resolveSecurityTaskLaunchConfig('vuln-triage', 'en', noRecipes)).toMatchObject({
      launchMode: 'prompt',
      availability: 'preview',
      recipeId: undefined,
    });

    expect(resolveSecurityTaskLaunchConfig('ioc-analysis', 'zh-CN', noRecipes)).toMatchObject({
      launchMode: 'prompt',
      availability: 'preview',
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

  it('generates localized starter prompts for guided chat tasks', () => {
    const zhPrompt = resolveSecurityTaskLaunchConfig('report-writing', 'zh-CN').starterPrompt;
    const enPrompt = resolveSecurityTaskLaunchConfig('report-writing', 'en').starterPrompt;

    expect(zhPrompt).toContain('report-writing');
    expect(zhPrompt).toContain('结构化');
    expect(enPrompt).toContain('report-writing');
    expect(enPrompt).toContain('structured');
  });

  it('exposes task metadata for WooYun-style investigation', () => {
    const task = getSecurityTaskById('wooyun-legacy');
    expect(task.skillId).toBe('wooyun-legacy');
    expect(task.recipeId).toBeUndefined();
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
