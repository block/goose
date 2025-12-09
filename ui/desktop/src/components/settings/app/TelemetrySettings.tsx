import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import { getTelemetryStatus, setTelemetryStatus } from '../../../api';
import { TELEMETRY_UI_ENABLED } from '../../../updates';
import TelemetryOptOutModal from '../../TelemetryOptOutModal';

interface TelemetrySettingsProps {
  variant?: 'card' | 'inline';
}

export default function TelemetrySettings({ variant = 'card' }: TelemetrySettingsProps) {
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
    // Reload status after modal closes in case user made a choice
    loadTelemetryStatus();
  };

  if (!TELEMETRY_UI_ENABLED) {
    return null;
  }

  const content = (
    <div className="flex items-center justify-between">
      <div>
        <h3 className="text-text-default text-xs">Anonymous usage data</h3>
        <p className="text-xs text-text-muted max-w-md mt-[2px]">
          Help improve goose by sharing anonymous usage statistics.{' '}
          <button
            onClick={() => setShowModal(true)}
            className="text-blue-600 dark:text-blue-400 hover:underline"
          >
            Learn more
          </button>
        </p>
      </div>
      <div className="flex items-center">
        <Switch
          checked={telemetryEnabled}
          onCheckedChange={handleTelemetryToggle}
          disabled={isLoading}
          variant="mono"
        />
      </div>
    </div>
  );

  const modal = (
    <TelemetryOptOutModal isOpen={showModal} onClose={handleModalClose} showOnFirstLaunch={false} />
  );

  if (variant === 'inline') {
    return (
      <>
        {content}
        {modal}
      </>
    );
  }

  return (
    <>
      <Card className="rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="mb-1">Privacy</CardTitle>
          <CardDescription>Control how your data is used</CardDescription>
        </CardHeader>
        <CardContent className="pt-4 space-y-4 px-4">{content}</CardContent>
      </Card>
      {modal}
    </>
  );
}
