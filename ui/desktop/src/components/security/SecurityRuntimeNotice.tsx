import { AlertTriangle } from 'lucide-react';

import type { SecurityTaskId } from '../../security/taskCatalog';
import {
  getSecurityRuntimeDiagnostics,
  getSecurityTaskRuntimeIssues,
  hasSecurityRuntimeAttention,
} from '../../securityRuntimeDiagnostics';
import { getSecurityPreviewLaunchInfo } from '../../securityPreviewRuntime';
import { defineMessages, useIntl } from '../../i18n';
import { cn } from '../../utils';

const i18n = defineMessages({
  overviewTitle: {
    id: 'securityRuntimeNotice.overviewTitle',
    defaultMessage: 'Security runtime needs attention',
  },
  overviewDescription: {
    id: 'securityRuntimeNotice.overviewDescription',
    defaultMessage:
      'The current working directory is missing or has drifted from the bundled security skills or task templates. Some task starters may fall back to guided chat or run with older instructions.',
  },
  missingSkills: {
    id: 'securityRuntimeNotice.missingSkills',
    defaultMessage: 'Missing skills: {ids}',
  },
  driftedSkills: {
    id: 'securityRuntimeNotice.driftedSkills',
    defaultMessage: 'Drifted skills: {ids}',
  },
  missingRecipes: {
    id: 'securityRuntimeNotice.missingRecipes',
    defaultMessage: 'Missing task templates: {ids}',
  },
  driftedRecipes: {
    id: 'securityRuntimeNotice.driftedRecipes',
    defaultMessage: 'Drifted task templates: {ids}',
  },
  skippedMissingWorkingDir: {
    id: 'securityRuntimeNotice.skippedMissingWorkingDir',
    defaultMessage:
      'The working directory is unavailable, so the app could not verify its local skill and task template assets.',
  },
  skippedMissingDistro: {
    id: 'securityRuntimeNotice.skippedMissingDistro',
    defaultMessage:
      'The bundled security distro source is unavailable, so the app could not verify its local skill and task template assets.',
  },
  repoResyncHint: {
    id: 'securityRuntimeNotice.repoResyncHint',
    defaultMessage:
      'Repo preview can resync the mirrored runtime assets with `node scripts/sync-security-runtime-assets.mjs`.',
  },
  packagedReseedHint: {
    id: 'securityRuntimeNotice.packagedReseedHint',
    defaultMessage:
      'Packaged local preview should be reopened through the official launcher so missing assets can be seeded into the current working directory again.',
  },
  taskSkillMissing: {
    id: 'securityRuntimeNotice.taskSkillMissing',
    defaultMessage: 'Skill asset missing in the current working directory',
  },
  taskSkillDrifted: {
    id: 'securityRuntimeNotice.taskSkillDrifted',
    defaultMessage: 'Skill asset differs from the bundled source',
  },
  taskRecipeMissing: {
    id: 'securityRuntimeNotice.taskRecipeMissing',
    defaultMessage: 'Task template runtime file missing in the current working directory',
  },
  taskRecipeDrifted: {
    id: 'securityRuntimeNotice.taskRecipeDrifted',
    defaultMessage: 'Task template runtime file differs from the bundled source',
  },
});

function formatTaskIssue(issue: ReturnType<typeof getSecurityTaskRuntimeIssues>[number], intl: ReturnType<typeof useIntl>): string {
  switch (issue) {
    case 'skill_missing':
      return intl.formatMessage(i18n.taskSkillMissing);
    case 'skill_drifted':
      return intl.formatMessage(i18n.taskSkillDrifted);
    case 'recipe_missing':
      return intl.formatMessage(i18n.taskRecipeMissing);
    case 'recipe_drifted':
      return intl.formatMessage(i18n.taskRecipeDrifted);
  }
}

export function SecurityRuntimeOverviewNotice({ className }: { className?: string }) {
  const intl = useIntl();
  const diagnostics = getSecurityRuntimeDiagnostics();
  const previewLaunchInfo = getSecurityPreviewLaunchInfo();

  if (!hasSecurityRuntimeAttention(diagnostics)) {
    return null;
  }

  const detailLines: string[] = [];

  if (diagnostics?.missingSkillIds.length) {
    detailLines.push(
      intl.formatMessage(i18n.missingSkills, { ids: diagnostics.missingSkillIds.join(', ') })
    );
  }

  if (diagnostics?.driftedSkillIds.length) {
    detailLines.push(
      intl.formatMessage(i18n.driftedSkills, { ids: diagnostics.driftedSkillIds.join(', ') })
    );
  }

  if (diagnostics?.missingRecipeIds.length) {
    detailLines.push(
      intl.formatMessage(i18n.missingRecipes, { ids: diagnostics.missingRecipeIds.join(', ') })
    );
  }

  if (diagnostics?.driftedRecipeIds.length) {
    detailLines.push(
      intl.formatMessage(i18n.driftedRecipes, { ids: diagnostics.driftedRecipeIds.join(', ') })
    );
  }

  const skippedDescription =
    diagnostics?.skippedReason === 'missing_working_dir'
      ? intl.formatMessage(i18n.skippedMissingWorkingDir)
      : diagnostics?.skippedReason === 'missing_distro'
        ? intl.formatMessage(i18n.skippedMissingDistro)
        : intl.formatMessage(i18n.overviewDescription);

  const actionHint = previewLaunchInfo.isPackagedLocalPreview
    ? intl.formatMessage(i18n.packagedReseedHint)
    : intl.formatMessage(i18n.repoResyncHint);

  return (
    <div
      data-testid="security-runtime-overview-notice"
      className={cn(
        'rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-amber-950 dark:text-amber-100',
        className
      )}
    >
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-700 dark:text-amber-300" />
        <div className="min-w-0 space-y-2">
          <p className="text-sm font-semibold">{intl.formatMessage(i18n.overviewTitle)}</p>
          <p className="text-sm text-amber-900/85 dark:text-amber-100/85">{skippedDescription}</p>
          {detailLines.length > 0 ? (
            <ul className="space-y-1 text-xs text-amber-900/85 dark:text-amber-100/85">
              {detailLines.map((line) => (
                <li key={line}>{line}</li>
              ))}
            </ul>
          ) : null}
          <p className="text-xs text-amber-900/85 dark:text-amber-100/85">{actionHint}</p>
        </div>
      </div>
    </div>
  );
}

export function SecurityTaskRuntimeHint({
  taskId,
  className,
}: {
  taskId: SecurityTaskId;
  className?: string;
}) {
  const intl = useIntl();
  const issues = getSecurityTaskRuntimeIssues(taskId, getSecurityRuntimeDiagnostics());

  if (issues.length === 0) {
    return null;
  }

  return (
    <p
      data-testid={`security-task-runtime-hint-${taskId}`}
      className={cn('text-xs leading-5 text-amber-700 dark:text-amber-300', className)}
    >
      {issues.map((issue) => formatTaskIssue(issue, intl)).join(' · ')}
    </p>
  );
}
