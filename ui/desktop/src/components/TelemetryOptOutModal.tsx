import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { BaseModal } from './ui/BaseModal';
import { Button } from './ui/button';
import { Goose } from './icons/Goose';
import { TELEMETRY_UI_ENABLED } from '../updates';
import { toastService } from '../toasts';
import { useConfig } from './ConfigContext';
import { trackTelemetryPreference } from '../utils/analytics';

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

type TelemetryOptOutModalProps =
  | { controlled: false }
  | { controlled: true; isOpen: boolean; onClose: () => void };

export default function TelemetryOptOutModal(props: TelemetryOptOutModalProps) {
  const { read, upsert } = useConfig();
  const { t } = useTranslation();
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
          title: t('telemetry.configErrorTitle'),
          msg: t('telemetry.configErrorMessage'),
          traceback: error instanceof Error ? error.stack || '' : '',
        });
      }
    };

    checkTelemetryChoice();
  }, [isControlled, read, t]);

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
            {t('telemetry.yesShare')}
          </Button>
          <Button
            variant="ghost"
            onClick={() => handleChoice(false)}
            disabled={isLoading}
            className="w-full h-[44px] rounded-lg text-text-secondary hover:text-text-primary"
          >
            {t('telemetry.noThanks')}
          </Button>
        </div>
      }
    >
      <div className="px-2 py-3">
        <div className="flex justify-center mb-4">
          <Goose className="size-10 text-text-primary" />
        </div>
        <h2 className="text-2xl font-regular dark:text-white text-gray-900 text-center mb-3">
          {t('telemetry.title')}
        </h2>
        <p className="text-text-primary text-sm mb-3">
          {t('telemetry.description')}
        </p>
        <div className="text-text-secondary text-xs space-y-1">
          <p className="font-medium text-text-primary">{t('telemetry.whatWeCollectTitle')}</p>
          <ul className="list-disc list-inside space-y-0.5 ml-1">
            <li>{t('telemetry.collectItems.os')}</li>
            <li>{t('telemetry.collectItems.version')}</li>
            <li>{t('telemetry.collectItems.provider')}</li>
            <li>{t('telemetry.collectItems.extensions')}</li>
            <li>{t('telemetry.collectItems.sessionMetrics')}</li>
            <li>{t('telemetry.collectItems.errorTypes')}</li>
          </ul>
          <p className="mt-3 text-text-secondary">
            {t('telemetry.privacyNote')}
          </p>
        </div>
      </div>
    </BaseModal>
  );
}
