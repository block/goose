import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { getTelemetryStatus, setTelemetryStatus } from '../../../api';
import { TELEMETRY_UI_ENABLED } from '../../../updates';
import TelemetryOptOutModal from '../../TelemetryOptOutModal';

interface TelemetrySettingsProps {
  isWelcome?: boolean;
}

export default function TelemetrySettings({ isWelcome = false }: TelemetrySettingsProps) {
  const [telemetryEnabled, setTelemetryEnabled] = useState(true);
  const [isLoading, setIsLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);

  const loadTelemetryStatus = async () => {
    try {
      const response = await getTelemetryStatus();
      if (response.data) {
        setTelemetryEnabled(response.data.enabled);
      }
    } catch (error) {
      console.error('Failed to load telemetry status:', error);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadTelemetryStatus();
  }, []);

  const handleTelemetryToggle = async (checked: boolean) => {
    try {
      const response = await setTelemetryStatus({ body: { enabled: checked } });
      if (response.data) {
        setTelemetryEnabled(response.data.enabled);
      }
    } catch (error) {
      console.error('Failed to update telemetry status:', error);
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

  const modal = (
    <TelemetryOptOutModal isOpen={showModal} onClose={handleModalClose} showOnFirstLaunch={false} />
  );

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
        <div className="w-full p-4 sm:p-6 bg-transparent border border-background-hover rounded-xl">
          <h3 className="font-medium text-text-standard text-sm sm:text-base mb-1">{title}</h3>
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
