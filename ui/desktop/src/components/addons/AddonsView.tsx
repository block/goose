import { MainPanelLayout } from '../Layout/MainPanelLayout';
import {
  AddonsInstallButton,
  AddonsInstallHint,
  AddonsPanel,
  AddonsReloadButton,
  useInstallAddonFromFolder,
} from '../../client-extensions/AddonsPanel';
import { useClientExtensions } from '../../client-extensions/ClientExtensionsContext';
import { defineMessages, useIntl } from '../../i18n';

const i18n = defineMessages({
  title: {
    id: 'addonsView.title',
    defaultMessage: 'Add-ons',
  },
  description: {
    id: 'addonsView.description',
    defaultMessage:
      'Install UI add-ons that extend goose Desktop with custom pages, chat actions, side panels, and message decorations. Distinct from MCP Extensions, which connect goose to external tools.',
  },
});

export default function AddonsView() {
  const intl = useIntl();
  const { loading, reloadExtensions } = useClientExtensions();
  const { installFromFolder } = useInstallAddonFromFolder();

  return (
    <MainPanelLayout>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto">
        <div className="bg-background-primary px-8 pb-4 pt-16">
          <div className="page-transition flex flex-col">
            <div className="mb-1 flex items-center justify-between gap-3">
              <h1 className="text-4xl font-light">{intl.formatMessage(i18n.title)}</h1>
              <div className="flex items-center gap-2">
                <AddonsInstallButton
                  loading={loading}
                  onInstall={() => void installFromFolder()}
                />
                <AddonsReloadButton loading={loading} onReload={() => void reloadExtensions()} />
              </div>
            </div>
            <p className="mb-2 max-w-3xl text-sm text-text-secondary">
              {intl.formatMessage(i18n.description)}
            </p>
            <AddonsInstallHint />
          </div>
        </div>

        <div className="flex-1 px-8 pb-16">
          <AddonsPanel />
        </div>
      </div>
    </MainPanelLayout>
  );
}
