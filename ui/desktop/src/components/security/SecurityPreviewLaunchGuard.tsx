import { AlertTriangle } from 'lucide-react';

import { getConfiguredProductName } from '../../branding/productText';
import { defineMessages, useIntl } from '../../i18n';
import { getSecurityPreviewLaunchInfo } from '../../securityPreviewRuntime';
import { cn } from '../../utils';

const i18n = defineMessages({
  title: {
    id: 'securityPreviewLaunchGuard.title',
    defaultMessage: 'This local preview was opened outside the supported launcher.',
  },
  description: {
    id: 'securityPreviewLaunchGuard.description',
    defaultMessage:
      '{appName} is using fallback local state, so the visible chats, model selection, or working directory may differ from the isolated preview session.',
  },
  commandsLabel: {
    id: 'securityPreviewLaunchGuard.commandsLabel',
    defaultMessage: 'Reopen it from the repo root with one of these supported commands:',
  },
});

export function SecurityPreviewLaunchGuard({ className }: { className?: string }) {
  const intl = useIntl();
  const appName = getConfiguredProductName();
  const previewLaunchInfo = getSecurityPreviewLaunchInfo();

  if (!previewLaunchInfo.isFallbackSession) {
    return null;
  }

  return (
    <div
      data-testid="security-preview-launch-guard"
      className={cn(
        'rounded-xl border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-amber-950 dark:text-amber-100',
        className
      )}
    >
      <div className="flex items-start gap-3">
        <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-700 dark:text-amber-300" />
        <div className="min-w-0 space-y-2">
          <p className="text-sm font-semibold">{intl.formatMessage(i18n.title)}</p>
          <p className="text-sm text-amber-900/85 dark:text-amber-100/85">
            {intl.formatMessage(i18n.description, { appName })}
          </p>
          <div className="space-y-2">
            <p className="text-xs font-medium uppercase tracking-wide text-amber-800/80 dark:text-amber-200/80">
              {intl.formatMessage(i18n.commandsLabel)}
            </p>
            <div className="flex flex-wrap gap-2">
              <code className="rounded-md bg-black/10 px-2 py-1 font-mono text-xs dark:bg-white/10">
                {previewLaunchInfo.supportedScriptCommand}
              </code>
              <code className="rounded-md bg-black/10 px-2 py-1 font-mono text-xs dark:bg-white/10">
                {previewLaunchInfo.supportedPnpmCommand}
              </code>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
