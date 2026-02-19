import { useState, useEffect, useCallback } from 'react';
import { Switch } from '../../ui/atoms/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/molecules/card';
import { useConfig } from '../../../contexts/ConfigContext';
import { TELEMETRY_UI_ENABLED } from '../../../updates';
import TelemetryOptOutModal from '../../modals/TelemetryOptOutModal';
import { toastService } from '../../../toasts';
import {
  setTelemetryEnabled as setAnalyticsTelemetryEnabled,
  trackTelemetryPreference,
} from '../../../utils/analytics';

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

interface TelemetrySettingsProps {
  isWelcome: boolean;
}

export default function TelemetrySettings({ isWelcome = false }: TelemetrySettingsProps) {
  const { read, upsert } = useConfig();
  const [telemetryEnabled, setTelemetryEnabled] = useState(true);
  const [isLoading, setIsLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);

  const loadTelemetryStatus = useCallback(async () => {
    try {
      const value = await read(TELEMETRY_CONFIG_KEY, false);
      setTelemetryEnabled(value === null ? true : Boolean(value));
    } catch (error) {
      console.error('Failed to load telemetry status:', error);
      toastService.error({
        title: 'Configuration Error',
        msg: 'Failed to load telemetry settings.',
        traceback: error instanceof Error ? error.stack || '' : '',
      });
    } finally {
      setIsLoading(false);
    }
  }, [read]);

  useEffect(() => {
    loadTelemetryStatus();
  }, [loadTelemetryStatus]);

  const handleTelemetryToggle = async (checked: boolean) => {
    try {
      await upsert(TELEMETRY_CONFIG_KEY, checked, false);
      setTelemetryEnabled(checked);
      setAnalyticsTelemetryEnabled(checked);
      trackTelemetryPreference(checked, isWelcome ? 'onboarding' : 'settings');
    } catch (error) {
      console.error('Failed to update telemetry status:', error);
      toastService.error({
        title: 'Configuration Error',
        msg: 'Failed to update telemetry settings.',
        traceback: error instanceof Error ? error.stack || '' : '',
      });
    }
  };

  const handleModalClose = () => {
    setShowModal(false);
    loadTelemetryStatus();
  };

  if (!TELEMETRY_UI_ENABLED) {
    return null;
  }

  const title = 'Privacy';
  const description = 'Control how your data is used';
  const toggleLabel = 'Anonymous usage data';
  const toggleDescription = 'Help improve goose by sharing anonymous usage statistics.';

  const learnMoreLink = (
    <button
      onClick={() => setShowModal(true)}
      className="text-blue-600 dark:text-blue-400 hover:underline"
    >
      Learn more
    </button>
  );

  const toggle = (
    <Switch
      checked={telemetryEnabled}
      onCheckedChange={handleTelemetryToggle}
      disabled={isLoading}
      variant="mono"
    />
  );

  const modal = <TelemetryOptOutModal controlled isOpen={showModal} onClose={handleModalClose} />;

  const toggleRow = (
    <div className="flex items-center justify-between">
      <div>
        <h4 className={isWelcome ? 'text-text-default text-sm' : 'text-text-default text-xs'}>
          {toggleLabel}
        </h4>
        <p className={`${isWelcome ? 'text-sm' : 'text-xs'} text-text-muted max-w-md mt-[2px]`}>
          {toggleDescription} {learnMoreLink}
        </p>
      </div>
      <div className="flex items-center">{toggle}</div>
    </div>
  );

  if (isWelcome) {
    return (
      <>
        <div className="w-full p-4 sm:p-6 bg-transparent border rounded-xl">
          <h3 className="font-medium text-text-default text-sm sm:text-base mb-1">{title}</h3>
          <p className="text-text-muted text-sm sm:text-base mb-4">{description}</p>
          {toggleRow}
        </div>
        {modal}
      </>
    );
  }

  return (
    <>
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">{title}</CardTitle>
          <CardDescription>{description}</CardDescription>
        </CardHeader>
        <CardContent className="pt-4 space-y-4 px-4">{toggleRow}</CardContent>
      </Card>
      {modal}
    </>
  );
}
