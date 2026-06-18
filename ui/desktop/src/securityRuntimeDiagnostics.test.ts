/**
 * @vitest-environment jsdom
 */
import { afterEach, describe, expect, it } from 'vitest';

import {
  getSecurityRuntimeAvailableRecipeIds,
  getSecurityRuntimeDiagnostics,
  getSecurityTaskRuntimeIssues,
  hasSecurityRuntimeAttention,
} from './securityRuntimeDiagnostics';

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

describe('securityRuntimeDiagnostics', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('parses runtime diagnostics and derives recipe availability for launcher gating', () => {
    mockAppConfig({
      SECURITY_RUNTIME_DIAGNOSTICS: {
        sourceSkillIds: ['vuln-triage'],
        sourceRecipeIds: ['security-vuln-triage', 'web-investigation'],
        missingSkillIds: [],
        driftedSkillIds: [],
        missingRecipeIds: ['web-investigation'],
        driftedRecipeIds: [],
      },
    });

    const diagnostics = getSecurityRuntimeDiagnostics();

    expect(diagnostics?.sourceRecipeIds).toEqual(['security-vuln-triage', 'web-investigation']);
    expect(Array.from(getSecurityRuntimeAvailableRecipeIds(diagnostics) ?? [])).toEqual([
      'security-vuln-triage',
    ]);
  });

  it('reports task-level skill and recipe issues without adding a parallel runtime layer', () => {
    const diagnostics = {
      sourceSkillIds: ['alert-triage', 'ioc-analysis'],
      sourceRecipeIds: ['alert-investigation'],
      missingSkillIds: ['alert-triage'],
      driftedSkillIds: ['ioc-analysis'],
      missingRecipeIds: [],
      driftedRecipeIds: ['alert-investigation'],
    };

    expect(getSecurityTaskRuntimeIssues('alert-investigation', diagnostics)).toEqual([
      'skill_missing',
      'recipe_drifted',
    ]);
    expect(getSecurityTaskRuntimeIssues('ioc-analysis', diagnostics)).toEqual(['skill_drifted']);
    expect(hasSecurityRuntimeAttention(diagnostics)).toBe(true);
  });
});
