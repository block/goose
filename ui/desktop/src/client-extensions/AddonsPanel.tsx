import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { Layers, Plus, RefreshCw, Trash2 } from 'lucide-react';
import { Button } from '../components/ui/button';
import { Switch } from '../components/ui/switch';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '../components/ui/card';
import { useClientExtensions } from './ClientExtensionsContext';
import type { ClientExtensionManifest, DiscoveredClientExtension } from './types';
import { defineMessages, useIntl } from '../i18n';
import { cn } from '../utils';
import { toastService } from '../toasts';

const i18n = defineMessages({
  emptyTitle: {
    id: 'addonsView.emptyTitle',
    defaultMessage: 'No add-ons installed yet',
  },
  empty: {
    id: 'addonsView.empty',
    defaultMessage:
      'Install UI add-ons with goose client-extension install <path>, or enable dev examples from your local checkout.',
  },
  installHint: {
    id: 'addonsView.installHint',
    defaultMessage: 'Install directory:',
  },
  cliHint: {
    id: 'addonsView.cliHint',
    defaultMessage: 'goose client-extension install <path>',
  },
  reload: {
    id: 'addonsView.reload',
    defaultMessage: 'Reload',
  },
  install: {
    id: 'addonsView.install',
    defaultMessage: 'Install add-on',
  },
  installSuccess: {
    id: 'addonsView.installSuccess',
    defaultMessage: 'Installed add-on',
  },
  installFailed: {
    id: 'addonsView.installFailed',
    defaultMessage: 'Failed to install add-on',
  },
  uninstallSuccess: {
    id: 'addonsView.uninstallSuccess',
    defaultMessage: 'Uninstalled add-on',
  },
  uninstallFailed: {
    id: 'addonsView.uninstallFailed',
    defaultMessage: 'Failed to uninstall add-on',
  },
  confirmUninstall: {
    id: 'addonsView.confirmUninstall',
    defaultMessage: 'Uninstall "{name}"? This removes it from your install directory.',
  },
  devUninstallHint: {
    id: 'addonsView.devUninstallHint',
    defaultMessage: 'Dev examples live in your repo — disable them here instead of uninstalling.',
  },
  version: {
    id: 'addonsView.version',
    defaultMessage: 'Version {version}',
  },
  devSource: {
    id: 'addonsView.devSource',
    defaultMessage: 'Dev example',
  },
  installedSource: {
    id: 'addonsView.installedSource',
    defaultMessage: 'Installed',
  },
  disabledSource: {
    id: 'addonsView.disabledSource',
    defaultMessage: 'Disabled',
  },
  noContributions: {
    id: 'addonsView.noContributions',
    defaultMessage: 'No UI contributions declared',
  },
  contributionPage: {
    id: 'addonsView.contribution.page',
    defaultMessage: 'page',
  },
  contributionChatAction: {
    id: 'addonsView.contribution.chatAction',
    defaultMessage: 'chat action',
  },
  contributionMessageSuffix: {
    id: 'addonsView.contribution.messageSuffix',
    defaultMessage: 'message decoration',
  },
  contributionCustomRender: {
    id: 'addonsView.contribution.customRender',
    defaultMessage: 'custom render',
  },
  contributionSidecar: {
    id: 'addonsView.contribution.sidecar',
    defaultMessage: 'side panel',
  },
  toggleAddon: {
    id: 'addonsView.toggleAddon',
    defaultMessage: 'Toggle {name}',
  },
  uninstall: {
    id: 'addonsView.uninstall',
    defaultMessage: 'Uninstall',
  },
  uninstallAddon: {
    id: 'addonsView.uninstallAddon',
    defaultMessage: 'Uninstall {name}',
  },
  uninstallHint: {
    id: 'addonsView.uninstallHint',
    defaultMessage: 'goose client-extension uninstall <id>',
  },
});

function contributionTags(manifest: ClientExtensionManifest, intl: ReturnType<typeof useIntl>) {
  const tags: string[] = [];
  for (const link of manifest.contributes?.rootLinks ?? []) {
    tags.push(`${intl.formatMessage(i18n.contributionPage)}: ${link.id}`);
  }
  for (const action of manifest.contributes?.chatActions ?? []) {
    tags.push(`${intl.formatMessage(i18n.contributionChatAction)}: ${action.id}`);
  }
  for (const suffix of manifest.contributes?.contentSuffixes ?? []) {
    tags.push(`${intl.formatMessage(i18n.contributionMessageSuffix)}: ${suffix.id}`);
  }
  for (const render of manifest.contributes?.customRenders ?? []) {
    tags.push(`${intl.formatMessage(i18n.contributionCustomRender)}: ${render.id}`);
  }
  for (const sidecar of manifest.contributes?.sidecars ?? []) {
    tags.push(`${intl.formatMessage(i18n.contributionSidecar)}: ${sidecar.id}`);
  }
  return tags;
}

function AddonCard({
  extension,
  loading,
  onToggle,
  onUninstall,
}: {
  extension: DiscoveredClientExtension;
  loading: boolean;
  onToggle: (enabled: boolean) => Promise<void>;
  onUninstall?: (extensionId: string) => Promise<void>;
}) {
  const intl = useIntl();
  const [visuallyEnabled, setVisuallyEnabled] = useState(extension.enabled);
  const [isToggling, setIsToggling] = useState(false);
  const [isUninstalling, setIsUninstalling] = useState(false);

  useEffect(() => {
    if (!isToggling) {
      setVisuallyEnabled(extension.enabled);
    }
  }, [extension.enabled, isToggling]);

  const tags = useMemo(
    () => contributionTags(extension.manifest, intl),
    [extension.manifest, intl]
  );

  const handleToggle = async () => {
    if (isToggling || loading) {
      return;
    }

    const nextState = !visuallyEnabled;
    setIsToggling(true);
    setVisuallyEnabled(nextState);
    try {
      await onToggle(nextState);
    } catch {
      setVisuallyEnabled(!nextState);
    } finally {
      setIsToggling(false);
    }
  };

  const handleUninstall = async () => {
    if (!onUninstall || isUninstalling || loading) {
      return;
    }

    if (
      !window.confirm(intl.formatMessage(i18n.confirmUninstall, { name: extension.id }))
    ) {
      return;
    }

    setIsUninstalling(true);
    try {
      await onUninstall(extension.id);
      toastService.success({
        title: intl.formatMessage(i18n.uninstallSuccess),
        msg: extension.id,
      });
    } catch (error) {
      toastService.error({
        title: intl.formatMessage(i18n.uninstallFailed),
        msg: error instanceof Error ? error.message : extension.id,
      });
    } finally {
      setIsUninstalling(false);
    }
  };

  const canUninstall = extension.source === 'installed' && onUninstall;

  return (
    <Card
      id={`addon-${extension.id}`}
      className={cn(
        'min-h-[160px] transition-all duration-200 hover:border-border-primary',
        !extension.enabled && 'opacity-75'
      )}
    >
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Layers className="h-4 w-4 text-text-secondary" />
          <span className="truncate">{extension.id}</span>
        </CardTitle>
        <CardAction>
          <div className="flex items-center gap-2">
            {canUninstall && (
              <Button
                type="button"
                variant="outline"
                size="xs"
                disabled={loading || isUninstalling}
                onClick={() => void handleUninstall()}
                className="text-text-secondary hover:text-destructive hover:border-destructive"
                aria-label={intl.formatMessage(i18n.uninstallAddon, { name: extension.id })}
              >
                <Trash2 className="h-3.5 w-3.5" />
                {intl.formatMessage(i18n.uninstall)}
              </Button>
            )}
            <Switch
              checked={visuallyEnabled}
              onCheckedChange={() => void handleToggle()}
              disabled={loading || isToggling}
              variant="mono"
              aria-label={intl.formatMessage(i18n.toggleAddon, { name: extension.id })}
            />
          </div>
        </CardAction>
        <CardDescription className="flex flex-wrap items-center gap-2 pt-1">
          <span>{intl.formatMessage(i18n.version, { version: extension.manifest.version })}</span>
          <span className="inline-block rounded bg-background-secondary px-2 py-0.5 text-xs">
            {extension.source === 'dev'
              ? intl.formatMessage(i18n.devSource)
              : intl.formatMessage(i18n.installedSource)}
          </span>
          {!extension.enabled && (
            <span className="inline-block rounded bg-background-secondary px-2 py-0.5 text-xs">
              {intl.formatMessage(i18n.disabledSource)}
            </span>
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="px-4 pt-0">
        {tags.length === 0 ? (
          <p className="text-sm text-text-secondary">{intl.formatMessage(i18n.noContributions)}</p>
        ) : (
          <div className="flex flex-wrap gap-2">
            {tags.map((tag) => (
              <span
                key={tag}
                className="inline-block rounded bg-background-secondary px-2 py-1 text-xs text-text-secondary"
              >
                {tag}
              </span>
            ))}
          </div>
        )}
        {extension.source === 'dev' && (
          <p className="mt-3 text-xs text-text-secondary">
            {intl.formatMessage(i18n.devUninstallHint)}
          </p>
        )}
      </CardContent>
    </Card>
  );
}

function AddonsGrid({ children }: { children: ReactNode }) {
  return (
    <div
      className="grid gap-4 p-1"
      style={{
        gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))',
        justifyContent: 'center',
      }}
    >
      {children}
    </div>
  );
}

export function AddonsInstallButton({
  loading,
  onInstall,
}: {
  loading: boolean;
  onInstall: () => void;
}) {
  const intl = useIntl();

  return (
    <Button
      type="button"
      variant="default"
      size="sm"
      onClick={onInstall}
      disabled={loading}
      className="flex items-center gap-2"
    >
      <Plus className="h-4 w-4" />
      {intl.formatMessage(i18n.install)}
    </Button>
  );
}

export function useInstallAddonFromFolder() {
  const intl = useIntl();
  const { installExtension, loading } = useClientExtensions();

  return {
    loading,
    installFromFolder: async () => {
      const result = await window.electron.directoryChooser();
      const sourcePath = result.canceled ? null : result.filePaths[0];
      if (!sourcePath) {
        return;
      }

      try {
        await installExtension(sourcePath);
        toastService.success({
          title: intl.formatMessage(i18n.installSuccess),
          msg: sourcePath,
        });
      } catch (error) {
        toastService.error({
          title: intl.formatMessage(i18n.installFailed),
          msg: error instanceof Error ? error.message : sourcePath,
        });
      }
    },
  };
}

export function AddonsPanel() {
  const intl = useIntl();
  const { extensions, loading, setExtensionEnabled, uninstallExtension } = useClientExtensions();
  const [installDir, setInstallDir] = useState<string | null>(null);

  useEffect(() => {
    void window.electron.getClientExtensionsInstallDir().then(setInstallDir);
  }, []);

  if (extensions.length === 0) {
    return (
      <div className="flex min-h-[320px] flex-col items-center justify-center gap-4">
        <div className="max-w-md text-center">
          <h3 className="mb-2 text-lg font-medium">{intl.formatMessage(i18n.emptyTitle)}</h3>
          <p className="text-sm text-text-secondary">{intl.formatMessage(i18n.empty)}</p>
          {installDir && (
            <p className="mt-4 break-all font-mono text-xs text-text-secondary">
              {intl.formatMessage(i18n.installHint)} {installDir}
            </p>
          )}
          <p className="mt-2 font-mono text-xs text-text-secondary">
            {intl.formatMessage(i18n.cliHint)}
          </p>
          <p className="mt-1 font-mono text-xs text-text-secondary">
            {intl.formatMessage(i18n.uninstallHint)}
          </p>
        </div>
      </div>
    );
  }

  return (
    <AddonsGrid>
      {extensions.map((extension) => (
        <AddonCard
          key={extension.id}
          extension={extension}
          loading={loading}
          onToggle={(enabled) => setExtensionEnabled(extension.id, enabled)}
          onUninstall={uninstallExtension}
        />
      ))}
    </AddonsGrid>
  );
}

export function AddonsInstallHint() {
  const intl = useIntl();
  const [installDir, setInstallDir] = useState<string | null>(null);

  useEffect(() => {
    void window.electron.getClientExtensionsInstallDir().then(setInstallDir);
  }, []);

  if (!installDir) {
    return null;
  }

  return (
    <p className="mb-6 font-mono text-xs text-text-secondary">
      {intl.formatMessage(i18n.installHint)} {installDir}
      <span className="mx-2 text-border-primary">·</span>
      {intl.formatMessage(i18n.cliHint)}
    </p>
  );
}

export function AddonsReloadButton({
  loading,
  onReload,
}: {
  loading: boolean;
  onReload: () => void;
}) {
  const intl = useIntl();

  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      onClick={onReload}
      disabled={loading}
      className="flex items-center gap-2"
    >
      <RefreshCw className="h-4 w-4" />
      {intl.formatMessage(i18n.reload)}
    </Button>
  );
}
