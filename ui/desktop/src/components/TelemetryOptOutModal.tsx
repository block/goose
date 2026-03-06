import { useState, useEffect, useRef } from 'react';
import { BaseModal } from './ui/BaseModal';
import { Button } from './ui/button';
import { Goose } from './icons/Goose';
import { TELEMETRY_UI_ENABLED } from '../updates';
import { toastService } from '../toasts';
import { useConfig } from './ConfigContext';
import { trackTelemetryPreference } from '../utils/analytics';
import { useLocalization } from '../contexts/LocalizationContext';

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

type TelemetryOptOutModalProps =
  | { controlled: false }
  | { controlled: true; isOpen: boolean; onClose: () => void };

export default function TelemetryOptOutModal(props: TelemetryOptOutModalProps) {
  const { read, upsert } = useConfig();
  const { t } = useLocalization();
  const isControlled = props.controlled;
  const controlledIsOpen = isControlled ? props.isOpen : undefined;
  const onClose = isControlled ? props.onClose : undefined;
  const [showModal, setShowModal] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const tRef = useRef(t);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

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
          title: tRef.current('telemetry.errors.configurationTitle'),
          msg: tRef.current('telemetry.errors.checkFailed'),
          traceback: error instanceof Error ? error.stack || '' : '',
        });
      }
    };

    checkTelemetryChoice();
  }, [isControlled, read]);

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
            {t('telemetry.modal.shareButton')}
          </Button>
          <Button
            variant="ghost"
            onClick={() => handleChoice(false)}
            disabled={isLoading}
            className="w-full h-[44px] rounded-lg text-text-secondary hover:text-text-primary"
          >
            {t('telemetry.modal.declineButton')}
          </Button>
        </div>
      }
    >
      <div className="px-2 py-3">
        <div className="flex justify-center mb-4">
          <Goose className="size-10 text-text-primary" />
        </div>
        <h2 className="text-2xl font-regular dark:text-white text-gray-900 text-center mb-3">
          {t('telemetry.modal.title')}
        </h2>
        <p className="text-text-primary text-sm mb-3">
          {t('telemetry.modal.body')}
        </p>
        <div className="text-text-secondary text-xs space-y-1">
          <p className="font-medium text-text-primary">{t('telemetry.modal.collectTitle')}</p>
          <ul className="list-disc list-inside space-y-0.5 ml-1">
            <li>{t('telemetry.modal.operatingSystem')}</li>
            <li>{t('telemetry.modal.gooseVersion')}</li>
            <li>{t('telemetry.modal.providerAndModel')}</li>
            <li>{t('telemetry.modal.extensionsUsage')}</li>
            <li>{t('telemetry.modal.sessionMetrics')}</li>
            <li>{t('telemetry.modal.errorTypes')}</li>
          </ul>
          <p className="mt-3 text-text-secondary">
            {t('telemetry.modal.privacyNotice')}
          </p>
        </div>
      </div>
    </BaseModal>
  );
}
