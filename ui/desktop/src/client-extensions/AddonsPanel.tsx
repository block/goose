import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { Layers, RefreshCw } from 'lucide-react';
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
}: {
  extension: DiscoveredClientExtension;
  loading: boolean;
  onToggle: (enabled: boolean) => Promise<void>;
}) {
  const intl = useIntl();
  const [visuallyEnabled, setVisuallyEnabled] = useState(extension.enabled);
  const [isToggling, setIsToggling] = useState(false);

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
          <Switch
            checked={visuallyEnabled}
            onCheckedChange={() => void handleToggle()}
            disabled={loading || isToggling}
            variant="mono"
            aria-label={intl.formatMessage(i18n.toggleAddon, { name: extension.id })}
          />
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

export function AddonsPanel() {
  const intl = useIntl();
  const { extensions, loading, setExtensionEnabled } = useClientExtensions();
  const [installDir, setInstallDir] = useState<string | null>(null);

  useEffect(() => {
    void window.electron.getClientExtensionsInstallDir().then(setInstallDir);
  }, []);

  if (extensions.length === 0) {
    return (
      <div className="flex min-h-[320px] items-center justify-center">
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
