import { useState, useEffect, useCallback, useRef } from 'react';
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
import { useLocalization } from '../../../contexts/LocalizationContext';

const TELEMETRY_CONFIG_KEY = 'GOOSE_TELEMETRY_ENABLED';

interface TelemetrySettingsProps {
  isWelcome: boolean;
}

export default function TelemetrySettings({ isWelcome = false }: TelemetrySettingsProps) {
  const { read, upsert } = useConfig();
  const { t } = useLocalization();
  const [telemetryEnabled, setTelemetryEnabled] = useState(true);
  const [isLoading, setIsLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const tRef = useRef(t);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  const loadTelemetryStatus = useCallback(async () => {
    try {
      const value = await read(TELEMETRY_CONFIG_KEY, false);
      setTelemetryEnabled(value === null ? true : Boolean(value));
    } catch (error) {
      console.error('Failed to load telemetry status:', error);
      toastService.error({
        title: tRef.current('telemetry.errors.configurationTitle'),
        msg: tRef.current('telemetry.errors.loadFailed'),
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
        title: t('telemetry.errors.configurationTitle'),
        msg: t('telemetry.errors.updateFailed'),
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

  const title = t('telemetry.title');
  const description = t('telemetry.description');
  const toggleLabel = t('telemetry.toggleLabel');
  const toggleDescription = t('telemetry.toggleDescription');

  const learnMoreLink = (
    <button
      onClick={() => setShowModal(true)}
      className="text-blue-600 dark:text-blue-400 hover:underline"
    >
      {t('common.actions.learnMore')}
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
