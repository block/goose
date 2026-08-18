import { useState, useEffect } from 'react';
import kebabCase from 'lodash/kebabCase';
import { Switch } from '../../../ui/switch';
import { Gear } from '../../../icons';
import { FixedExtensionEntry } from '../../../ConfigContext';
import { getSubtitle, getFriendlyTitle } from './ExtensionList';
import { Card, CardHeader, CardTitle, CardContent, CardAction } from '../../../ui/card';
import { defineMessages, useIntl } from '../../../../i18n';
import { toastService } from '../../../../toasts';
import { buildExtensionAuthenticateLink } from '../extensionAuthDeeplink';
import { nameToKey } from '../utils';

const i18n = defineMessages({
  configureExtension: {
    id: 'extensionItem.configureExtension',
    defaultMessage: 'Configure {name} Extension',
  },
  toggleExtension: {
    id: 'extensionItem.toggleExtension',
    defaultMessage: 'Toggle {name} extension On or Off',
  },
  toggleOAuthExtension: {
    id: 'extensionItem.toggleOAuthExtension',
    defaultMessage: 'Turn {name} on to sign in, or off to sign out',
  },
  signedIn: {
    id: 'extensionItem.signedIn',
    defaultMessage: 'Signed in',
  },
  signedOut: {
    id: 'extensionItem.signedOut',
    defaultMessage: 'Not signed in',
  },
  signingIn: {
    id: 'extensionItem.signingIn',
    defaultMessage: 'Signing in…',
  },
  signingOut: {
    id: 'extensionItem.signingOut',
    defaultMessage: 'Signing out…',
  },
  copyAuthLink: {
    id: 'extensionItem.copyAuthLink',
    defaultMessage: 'Copy sign-in link',
  },
  authLinkCopied: {
    id: 'extensionItem.authLinkCopied',
    defaultMessage: 'Sign-in link copied',
  },
});

interface ExtensionItemProps {
  extension: FixedExtensionEntry;
  onToggle: (extension: FixedExtensionEntry) => Promise<boolean | void> | void;
  onConfigure?: (extension: FixedExtensionEntry) => void;
  isStatic?: boolean;
}

export default function ExtensionItem({
  extension,
  onToggle,
  onConfigure,
  isStatic,
}: ExtensionItemProps) {
  const intl = useIntl();
  const [visuallyEnabled, setVisuallyEnabled] = useState(extension.enabled);
  const [isToggling, setIsToggling] = useState(false);

  const isStreamableHttp = extension.type === 'streamable_http';

  const handleToggle = async (ext: FixedExtensionEntry) => {
    if (isToggling) return;

    setIsToggling(true);
    const newState = !ext.enabled;
    setVisuallyEnabled(newState);

    try {
      await onToggle(ext);
    } catch {
      setVisuallyEnabled(!newState);
    } finally {
      setIsToggling(false);
    }
  };

  const handleCopyAuthLink = async () => {
    const configKey = extension.configKey ?? nameToKey(extension.name);
    const link = buildExtensionAuthenticateLink(configKey);
    await navigator.clipboard.writeText(link);
    toastService.success({
      title: getFriendlyTitle(extension),
      msg: intl.formatMessage(i18n.authLinkCopied),
    });
  };

  useEffect(() => {
    if (!isToggling) {
      setVisuallyEnabled(extension.enabled);
    }
  }, [extension.enabled, isToggling]);

  const renderSubtitle = () => {
    const { description, command } = getSubtitle(extension);
    return (
      <>
        {description && <span>{description}</span>}
        {description && command && <br />}
        {command && <span className="font-mono text-xs">{command}</span>}
      </>
    );
  };

  const editable =
    !(extension.type === 'builtin' || ('bundled' in extension && extension.bundled)) && !isStatic;

  const authStatusMessage = isToggling
    ? visuallyEnabled
      ? i18n.signingIn
      : i18n.signingOut
    : extension.authenticated
      ? i18n.signedIn
      : i18n.signedOut;

  return (
    <Card
      id={`extension-${kebabCase(extension.name)}`}
      className="transition-all duration-200 min-h-[120px] overflow-hidden"
    >
      <CardHeader>
        <CardTitle>{getFriendlyTitle(extension)}</CardTitle>

        <CardAction>
          <div className="flex items-center justify-end gap-2">
            {editable && (
              <button
                className="text-text-secondary hover:text-text-primary"
                aria-label={intl.formatMessage(i18n.configureExtension, {
                  name: getFriendlyTitle(extension),
                })}
                onClick={() => onConfigure?.(extension)}
              >
                <Gear className="w-4 h-4" />
              </button>
            )}
            <Switch
              checked={visuallyEnabled}
              onCheckedChange={() => handleToggle(extension)}
              disabled={isToggling}
              variant="mono"
              aria-label={
                isStreamableHttp
                  ? intl.formatMessage(i18n.toggleOAuthExtension, {
                      name: getFriendlyTitle(extension),
                    })
                  : intl.formatMessage(i18n.toggleExtension, {
                      name: getFriendlyTitle(extension),
                    })
              }
            />
          </div>
        </CardAction>
      </CardHeader>
      <CardContent className="px-4 overflow-hidden text-sm break-words text-text-secondary">
        {renderSubtitle()}
        {isStreamableHttp && (
          <div className="mt-2 flex items-center gap-3 text-xs">
            <span className={extension.authenticated ? 'text-text-primary' : 'text-text-secondary'}>
              {intl.formatMessage(authStatusMessage)}
            </span>
            <button
              type="button"
              className="text-text-primary underline hover:no-underline"
              onClick={handleCopyAuthLink}
            >
              {intl.formatMessage(i18n.copyAuthLink)}
            </button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
