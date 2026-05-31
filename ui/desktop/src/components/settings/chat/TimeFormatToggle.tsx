import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { defineMessages, useIntl } from '../../../i18n';
import { setTimeFormat } from '../../../utils/timeUtils';

const i18n = defineMessages({
  title: {
    id: 'timeFormatToggle.title',
    defaultMessage: 'Use 24-Hour Clock',
  },
  description: {
    id: 'timeFormatToggle.description',
    defaultMessage: 'Display message timestamps in 24-hour format.',
  },
});

export const TimeFormatToggle = () => {
  const intl = useIntl();
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    const loadState = async () => {
      const value = await window.electron.getSetting('use24HourClock');
      setEnabled(value);
      setTimeFormat(value);
    };
    loadState();
  }, []);

  const handleToggle = async (checked: boolean) => {
    setEnabled(checked);
    setTimeFormat(checked);
    await window.electron.setSetting('use24HourClock', checked);
  };

  return (
    <div className="flex items-center justify-between py-2 px-2 hover:bg-background-secondary rounded-lg transition-all">
      <div>
        <h3 className="text-text-primary">{intl.formatMessage(i18n.title)}</h3>
        <p className="text-xs text-text-secondary max-w-md mt-[2px]">
          {intl.formatMessage(i18n.description)}
        </p>
      </div>
      <div className="flex items-center">
        <Switch checked={enabled} onCheckedChange={handleToggle} variant="mono" />
      </div>
    </div>
  );
};
