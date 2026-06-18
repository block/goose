import { getSecurityTaskById, type SecurityTaskId } from './security/taskCatalog';

export interface SecurityRuntimeDiagnostics {
  workingDir?: string;
  runtimeSkillRoot?: string;
  runtimeRecipeRoot?: string;
  sourceSkillIds: string[];
  sourceRecipeIds: string[];
  missingSkillIds: string[];
  driftedSkillIds: string[];
  missingRecipeIds: string[];
  driftedRecipeIds: string[];
  skippedReason?: 'missing_distro' | 'missing_working_dir';
}

export type SecurityRuntimeIssueCode =
  | 'skill_missing'
  | 'skill_drifted'
  | 'recipe_missing'
  | 'recipe_drifted';

function asStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value.filter((entry): entry is string => typeof entry === 'string');
}

function asOptionalString(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined;
}

export function getSecurityRuntimeDiagnostics(): SecurityRuntimeDiagnostics | null {
  const rawValue = window.appConfig?.get('SECURITY_RUNTIME_DIAGNOSTICS');
  if (!rawValue || typeof rawValue !== 'object') {
    return null;
  }

  const diagnostics = rawValue as Record<string, unknown>;
  return {
    workingDir: asOptionalString(diagnostics.workingDir),
    runtimeSkillRoot: asOptionalString(diagnostics.runtimeSkillRoot),
    runtimeRecipeRoot: asOptionalString(diagnostics.runtimeRecipeRoot),
    sourceSkillIds: asStringArray(diagnostics.sourceSkillIds),
    sourceRecipeIds: asStringArray(diagnostics.sourceRecipeIds),
    missingSkillIds: asStringArray(diagnostics.missingSkillIds),
    driftedSkillIds: asStringArray(diagnostics.driftedSkillIds),
    missingRecipeIds: asStringArray(diagnostics.missingRecipeIds),
    driftedRecipeIds: asStringArray(diagnostics.driftedRecipeIds),
    skippedReason:
      diagnostics.skippedReason === 'missing_distro' || diagnostics.skippedReason === 'missing_working_dir'
        ? diagnostics.skippedReason
        : undefined,
  };
}

export function getSecurityRuntimeAvailableRecipeIds(
  diagnostics: SecurityRuntimeDiagnostics | null
): ReadonlySet<string> | undefined {
  if (!diagnostics || diagnostics.skippedReason) {
    return undefined;
  }

  return new Set(
    diagnostics.sourceRecipeIds.filter((recipeId) => !diagnostics.missingRecipeIds.includes(recipeId))
  );
}

export function getSecurityTaskRuntimeIssues(
  taskId: SecurityTaskId,
  diagnostics: SecurityRuntimeDiagnostics | null
): SecurityRuntimeIssueCode[] {
  if (!diagnostics) {
    return [];
  }

  const task = getSecurityTaskById(taskId);
  const issues: SecurityRuntimeIssueCode[] = [];

  if (diagnostics.missingSkillIds.includes(task.skillId)) {
    issues.push('skill_missing');
  } else if (diagnostics.driftedSkillIds.includes(task.skillId)) {
    issues.push('skill_drifted');
  }

  if (task.recipeId) {
    if (diagnostics.missingRecipeIds.includes(task.recipeId)) {
      issues.push('recipe_missing');
    } else if (diagnostics.driftedRecipeIds.includes(task.recipeId)) {
      issues.push('recipe_drifted');
    }
  }

  return issues;
}

export function hasSecurityRuntimeAttention(
  diagnostics: SecurityRuntimeDiagnostics | null
): boolean {
  if (!diagnostics) {
    return false;
  }

  return (
    diagnostics.missingSkillIds.length > 0 ||
    diagnostics.driftedSkillIds.length > 0 ||
    diagnostics.missingRecipeIds.length > 0 ||
    diagnostics.driftedRecipeIds.length > 0 ||
    typeof diagnostics.skippedReason === 'string'
  );
}
