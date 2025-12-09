import { useState, useEffect } from 'react';
import { BaseModal } from './ui/BaseModal';
import { Button } from './ui/button';
import { readConfig, setTelemetryStatus } from '../api';
import { Goose } from './icons/Goose';
import { TELEMETRY_UI_ENABLED } from '../updates';

interface TelemetryOptOutModalProps {
  isOpen?: boolean;
  onClose?: () => void;
  showOnFirstLaunch?: boolean;
}

export default function TelemetryOptOutModal({
  isOpen: controlledIsOpen,
  onClose,
  showOnFirstLaunch = true,
}: TelemetryOptOutModalProps) {
  const [showModal, setShowModal] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    if (!showOnFirstLaunch) return;

    const checkTelemetryChoice = async () => {
      try {
        const providerResponse = await readConfig({
          body: { key: 'GOOSE_PROVIDER', is_secret: false },
        });

        if (!providerResponse.data || providerResponse.data === '') {
          return;
        }

        const telemetryResponse = await readConfig({
          body: { key: 'GOOSE_TELEMETRY_ENABLED', is_secret: false },
        });

        if (telemetryResponse.data === null || telemetryResponse.data === undefined) {
          setShowModal(true);
        }
      } catch (error) {
        console.error('Failed to check telemetry config:', error);
      }
    };

    checkTelemetryChoice();
  }, [showOnFirstLaunch]);

  const handleChoice = async (enabled: boolean) => {
    setIsLoading(true);
    try {
      await setTelemetryStatus({ body: { enabled } });
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
            Yes, share anonymous usage data
          </Button>
          <Button
            variant="ghost"
            onClick={() => handleChoice(false)}
            disabled={isLoading}
            className="w-full h-[44px] rounded-lg text-text-muted hover:text-text-default"
          >
            No thanks
          </Button>
        </div>
      }
    >
      <div className="px-2 py-3">
        <div className="flex justify-center mb-4">
          <Goose className="size-10 text-text-default" />
        </div>
        <h2 className="text-2xl font-regular dark:text-white text-gray-900 text-center mb-3">
          Help improve goose
        </h2>
        <p className="text-text-default text-sm mb-3">
          Would you like to help improve goose by sharing anonymous usage data? This helps us
          understand how goose is used and identify areas for improvement.
        </p>
        <div className="text-text-muted text-xs space-y-1">
          <p className="font-medium text-text-default">What we collect:</p>
          <ul className="list-disc list-inside space-y-0.5 ml-1">
            <li>Operating system and architecture</li>
            <li>goose version</li>
            <li>Provider and model used</li>
            <li>Number of extensions enabled</li>
            <li>Session count and token usage (aggregated)</li>
          </ul>
          <p className="mt-3 text-text-muted">
            We never collect your conversations, code, or any personal data. You can change this
            setting anytime in Settings → App.
          </p>
        </div>
      </div>
    </BaseModal>
  );
}
