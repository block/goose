import { useIntl } from '../../i18n';
import {
  getSecurityExtensionById,
  SECURITY_EXTENSION_IDS,
  type SecurityExtensionId,
  type SecurityExtensionStatus,
} from '../../security/extensionCatalog';
import { securityTaskUiMessages } from '../../security/taskMessages';

function getStatusMessage(status: SecurityExtensionStatus) {
  switch (status) {
    case 'local_preview':
      return securityTaskUiMessages.extensionStatusLocalPreview;
    case 'disabled_stub':
      return securityTaskUiMessages.extensionStatusDisabledStub;
    case 'blocked_external_dependency':
      return securityTaskUiMessages.extensionStatusBlocked;
  }
}

function getStatusDetailMessage(status: SecurityExtensionStatus) {
  switch (status) {
    case 'local_preview':
      return securityTaskUiMessages.extensionDetailLocalPreview;
    case 'disabled_stub':
      return securityTaskUiMessages.extensionDetailDisabledStub;
    case 'blocked_external_dependency':
      return securityTaskUiMessages.extensionDetailBlockedExternal;
  }
}

function getStatusClassName(status: SecurityExtensionStatus): string {
  switch (status) {
    case 'local_preview':
      return 'border-emerald-500/30 bg-emerald-500/10 text-emerald-200';
    case 'disabled_stub':
      return 'border-amber-500/30 bg-amber-500/10 text-amber-200';
    case 'blocked_external_dependency':
      return 'border-rose-500/30 bg-rose-500/10 text-rose-200';
  }
}

export function SecurityExtensionOverview() {
  const intl = useIntl();

  return (
    <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-4">
      {SECURITY_EXTENSION_IDS.map((extensionId) => {
        const extension = getSecurityExtensionById(extensionId);
        return (
          <div
            key={extension.id}
            data-testid={`security-extension-status-${extension.id}`}
            className="rounded-xl border border-border-primary bg-background-secondary/50 px-3 py-2"
          >
            <div className="flex items-start justify-between gap-2">
              <span className="text-sm font-medium text-text-primary">{extension.displayName}</span>
              <span
                data-testid={`security-extension-badge-${extension.id}`}
                className={`rounded-full border px-2 py-0.5 text-[10px] uppercase tracking-[0.12em] ${getStatusClassName(extension.status)}`}
              >
                {intl.formatMessage(getStatusMessage(extension.status))}
              </span>
            </div>
            <p className="mt-1 text-xs leading-5 text-text-secondary">
              {intl.formatMessage(getStatusDetailMessage(extension.status))}
            </p>
          </div>
        );
      })}
    </div>
  );
}

interface SecurityTaskExtensionHintsProps {
  extensionIds: ReadonlyArray<SecurityExtensionId>;
  compact?: boolean;
}

export function SecurityTaskExtensionHints({
  extensionIds,
  compact = false,
}: SecurityTaskExtensionHintsProps) {
  const intl = useIntl();

  if (extensionIds.length === 0) {
    return null;
  }

  return (
    <div className="space-y-1.5">
      <p className="text-[11px] uppercase tracking-[0.14em] text-text-secondary">
        {intl.formatMessage(securityTaskUiMessages.recommendedExtensions)}
      </p>
      <div className="flex flex-wrap gap-2">
        {extensionIds.map((extensionId) => {
          const extension = getSecurityExtensionById(extensionId);
          return (
            <span
              key={extension.id}
              data-testid={`security-task-extension-${extension.id}`}
              className={`rounded-full border px-2 py-1 ${compact ? 'text-[11px]' : 'text-xs'} ${getStatusClassName(extension.status)}`}
            >
              <span className="font-medium">{extension.displayName}</span>
              <span className="mx-1 text-current/70">·</span>
              <span>{intl.formatMessage(getStatusMessage(extension.status))}</span>
            </span>
          );
        })}
      </div>
    </div>
  );
}
