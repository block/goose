import { useEffect, useState } from 'react';
import { LayoutPanelTop, RefreshCw } from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../components/ui/card';
import { Button } from '../components/ui/button';
import { useClientExtensions } from '../client-extensions/ClientExtensionsContext';
import { defineMessages, useIntl } from '../i18n';

const i18n = defineMessages({
  title: {
    id: 'settings.clientExtensions.title',
    defaultMessage: 'Client extensions',
  },
  description: {
    id: 'settings.clientExtensions.description',
    defaultMessage: 'Installed UI extensions that add navigation and chat actions to goose Desktop.',
  },
  empty: {
    id: 'settings.clientExtensions.empty',
    defaultMessage: 'No client extensions installed.',
  },
  installHint: {
    id: 'settings.clientExtensions.installHint',
    defaultMessage: 'Install extensions into:',
  },
  reload: {
    id: 'settings.clientExtensions.reload',
    defaultMessage: 'Reload extensions',
  },
  version: {
    id: 'settings.clientExtensions.version',
    defaultMessage: 'Version {version}',
  },
});

export default function ClientExtensionsSettingsSection() {
  const intl = useIntl();
  const { extensions, loading, reloadExtensions } = useClientExtensions();
  const [installDir, setInstallDir] = useState<string | null>(null);

  useEffect(() => {
    void window.electron.getClientExtensionsInstallDir().then(setInstallDir);
  }, []);

  return (
    <Card>
      <CardHeader className="flex flex-row items-start justify-between gap-4">
        <div>
          <CardTitle>{intl.formatMessage(i18n.title)}</CardTitle>
          <CardDescription>{intl.formatMessage(i18n.description)}</CardDescription>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => void reloadExtensions()}
          disabled={loading}
        >
          <RefreshCw className="w-4 h-4" />
          {intl.formatMessage(i18n.reload)}
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        {installDir && (
          <p className="text-xs text-text-secondary font-mono break-all">
            {intl.formatMessage(i18n.installHint)} {installDir}
          </p>
        )}

        {extensions.length === 0 ? (
          <p className="text-sm text-text-secondary">{intl.formatMessage(i18n.empty)}</p>
        ) : (
          <ul className="space-y-2">
            {extensions.map((extension) => (
              <li
                key={extension.id}
                className="flex items-start gap-3 rounded-lg border border-border-primary px-3 py-2"
              >
                <LayoutPanelTop className="mt-0.5 h-4 w-4 flex-shrink-0 text-text-secondary" />
                <div className="min-w-0 flex-1">
                  <div className="text-sm font-medium text-text-primary">{extension.id}</div>
                  <div className="text-xs text-text-secondary">
                    {intl.formatMessage(i18n.version, { version: extension.manifest.version })}
                  </div>
                  <div className="mt-1 flex flex-wrap gap-2 text-xs text-text-secondary">
                    {(extension.manifest.contributes?.rootLinks ?? []).map((link) => (
                      <span key={link.id} className="rounded bg-background-secondary px-2 py-0.5">
                        rootLink:{link.id}
                      </span>
                    ))}
                    {(extension.manifest.contributes?.chatActions ?? []).map((action) => (
                      <span key={action.id} className="rounded bg-background-secondary px-2 py-0.5">
                        chatAction:{action.id}
                      </span>
                    ))}
                    {(extension.manifest.contributes?.contentSuffixes ?? []).map((suffix) => (
                      <span key={suffix.id} className="rounded bg-background-secondary px-2 py-0.5">
                        contentSuffix:{suffix.id}
                      </span>
                    ))}
                    {(extension.manifest.contributes?.customRenders ?? []).map((render) => (
                      <span key={render.id} className="rounded bg-background-secondary px-2 py-0.5">
                        customRender:{render.id}
                      </span>
                    ))}
                    {(extension.manifest.contributes?.sidecars ?? []).map((sidecar) => (
                      <span key={sidecar.id} className="rounded bg-background-secondary px-2 py-0.5">
                        sidecar:{sidecar.id}
                      </span>
                    ))}
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
    </Card>
  );
}
