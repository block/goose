import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { useConfig } from '../../ConfigContext';
import { TELEMETRY_UI_ENABLED } from '../../../updates';
import TelemetryOptOutModal from '../../TelemetryOptOutModal';
import { toastService } from '../../../toasts';
import {
  setTelemetryEnabled as setAnalyticsTelemetryEnabled,
  trackTelemetryPreference,
} from '../../../utils/analytics';


interface TelemetrySettingsProps {
  isWelcome: boolean;
}

export default function TelemetrySettings({ isWelcome = false }: TelemetrySettingsProps) {
  const { config, update } = useConfig();
  const [telemetryEnabled, setTelemetryEnabled] = useState(true);
  const [isLoading] = useState(false);
  const [showModal, setShowModal] = useState(false);

  useEffect(() => {
    const value = config.GOOSE_TELEMETRY_ENABLED;
    setTelemetryEnabled(value === null || value === undefined ? true : Boolean(value));
  }, [config.GOOSE_TELEMETRY_ENABLED]);

  const handleTelemetryToggle = async (checked: boolean) => {
    try {
      await update({ GOOSE_TELEMETRY_ENABLED: checked });
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
        <h4 className={isWelcome ? 'text-text-primary text-sm' : 'text-text-primary text-xs'}>
          {toggleLabel}
        </h4>
        <p className={`${isWelcome ? 'text-sm' : 'text-xs'} text-text-secondary max-w-md mt-[2px]`}>
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
          <h3 className="font-medium text-text-primary text-sm sm:text-base mb-1">{title}</h3>
          <p className="text-text-secondary text-sm sm:text-base mb-4">{description}</p>
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
