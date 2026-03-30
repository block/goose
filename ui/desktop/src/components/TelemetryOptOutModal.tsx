import { useState, useEffect } from 'react';
import { BaseModal } from './ui/BaseModal';
import { Button } from './ui/button';
import { Goose } from './icons/Goose';
import { TELEMETRY_UI_ENABLED } from '../updates';
import { toastService } from '../toasts';
import { useConfig } from './ConfigContext';
import { trackTelemetryPreference } from '../utils/analytics';
import { defineMessages, useIntl } from '../i18n';

const i18n = defineMessages({
  configError: {
    id: 'telemetryOptOutModal.configError',
    defaultMessage: 'Configuration Error',
  },
  configErrorMessage: {
    id: 'telemetryOptOutModal.configErrorMessage',
    defaultMessage: 'Failed to check telemetry configuration.',
  },
  optIn: {
    id: 'telemetryOptOutModal.optIn',
    defaultMessage: 'Yes, share anonymous usage data',
  },
  optOut: {
    id: 'telemetryOptOutModal.optOut',
    defaultMessage: 'No thanks',
  },
  heading: {
    id: 'telemetryOptOutModal.heading',
    defaultMessage: 'Help improve goose',
  },
  description: {
    id: 'telemetryOptOutModal.description',
    defaultMessage:
      'Would you like to help improve goose by sharing anonymous usage data? This helps us understand how goose is used and identify areas for improvement.',
  },
  whatWeCollect: {
    id: 'telemetryOptOutModal.whatWeCollect',
    defaultMessage: 'What we collect:',
  },
  collectOs: {
    id: 'telemetryOptOutModal.collectOs',
    defaultMessage: 'Operating system, version, and architecture',
  },
  collectVersion: {
    id: 'telemetryOptOutModal.collectVersion',
    defaultMessage: 'goose version and install method',
  },
  collectProvider: {
    id: 'telemetryOptOutModal.collectProvider',
    defaultMessage: 'Provider and model used',
  },
  collectExtensions: {
    id: 'telemetryOptOutModal.collectExtensions',
    defaultMessage: 'Extensions and tool usage counts (names only)',
  },
  collectSession: {
    id: 'telemetryOptOutModal.collectSession',
    defaultMessage: 'Session metrics (duration, interaction count, token usage)',
  },
  collectErrors: {
    id: 'telemetryOptOutModal.collectErrors',
    defaultMessage: 'Error types (e.g., "rate_limit", "auth" - no details)',
  },
  privacyNote: {
    id: 'telemetryOptOutModal.privacyNote',
    defaultMessage:
      'We never collect your conversations, code, tool arguments, error messages, or any personal data. You can change this setting anytime in Settings → App.',
  },
});

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

type TelemetryOptOutModalProps =
  | { controlled: false }
  | { controlled: true; isOpen: boolean; onClose: () => void };

export default function TelemetryOptOutModal(props: TelemetryOptOutModalProps) {
  const intl = useIntl();
  const { read, upsert } = useConfig();
  const isControlled = props.controlled;
  const controlledIsOpen = isControlled ? props.isOpen : undefined;
  const onClose = isControlled ? props.onClose : undefined;
  const [showModal, setShowModal] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  // Only check telemetry choice on first launch in uncontrolled mode
  useEffect(() => {
    if (isControlled) return;

    const checkTelemetryChoice = async () => {
      try {
        const provider = await read('GOOSE_PROVIDER', false);

        if (!provider || provider === '') {
          return;
        }

        const telemetryEnabled = await read(TELEMETRY_CONFIG_KEY, false);

        if (telemetryEnabled === null) {
          setShowModal(true);
        }
      } catch (error) {
        console.error('Failed to check telemetry config:', error);
        toastService.error({
          title: intl.formatMessage(i18n.configError),
          msg: intl.formatMessage(i18n.configErrorMessage),
          traceback: error instanceof Error ? error.stack || '' : '',
        });
      }
    };

    checkTelemetryChoice();
  }, [isControlled, read, intl]);

  const handleChoice = async (enabled: boolean) => {
    setIsLoading(true);
    try {
      await upsert(TELEMETRY_CONFIG_KEY, enabled, false);
      trackTelemetryPreference(enabled, 'modal');
      setShowModal(false);
      onClose?.();
    } catch (error) {
      console.error('Failed to set telemetry preference:', error);
      setShowModal(false);
      onClose?.();
    } finally {
      setIsLoading(false);
    }
  };

  if (!TELEMETRY_UI_ENABLED) {
    return null;
  }

  const isModalOpen = controlledIsOpen !== undefined ? controlledIsOpen : showModal;

  if (!isModalOpen) {
    return null;
  }

  return (
    <BaseModal
      isOpen={isModalOpen}
      actions={
        <div className="flex flex-col gap-2 pb-3 px-3">
          <Button
            variant="default"
            onClick={() => handleChoice(true)}
            disabled={isLoading}
            className="w-full h-[44px] rounded-lg"
          >
            {intl.formatMessage(i18n.optIn)}
          </Button>
          <Button
            variant="ghost"
            onClick={() => handleChoice(false)}
            disabled={isLoading}
            className="w-full h-[44px] rounded-lg text-text-secondary hover:text-text-primary"
          >
            {intl.formatMessage(i18n.optOut)}
          </Button>
        </div>
      }
    >
      <div className="px-2 py-3">
        <div className="flex justify-center mb-4">
          <Goose className="size-10 text-text-primary" />
        </div>
        <h2 className="text-2xl font-regular dark:text-white text-gray-900 text-center mb-3">
          {intl.formatMessage(i18n.heading)}
        </h2>
        <p className="text-text-primary text-sm mb-3">
          {intl.formatMessage(i18n.description)}
        </p>
        <div className="text-text-secondary text-xs space-y-1">
          <p className="font-medium text-text-primary">{intl.formatMessage(i18n.whatWeCollect)}</p>
          <ul className="list-disc list-inside space-y-0.5 ml-1">
            <li>{intl.formatMessage(i18n.collectOs)}</li>
            <li>{intl.formatMessage(i18n.collectVersion)}</li>
            <li>{intl.formatMessage(i18n.collectProvider)}</li>
            <li>{intl.formatMessage(i18n.collectExtensions)}</li>
            <li>{intl.formatMessage(i18n.collectSession)}</li>
            <li>{intl.formatMessage(i18n.collectErrors)}</li>
          </ul>
          <p className="mt-3 text-text-secondary">
            {intl.formatMessage(i18n.privacyNote)}
          </p>
        </div>
      </div>
    </BaseModal>
  );
}
