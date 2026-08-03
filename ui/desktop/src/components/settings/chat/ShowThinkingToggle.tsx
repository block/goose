import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';
import { defineMessages, useIntl } from '../../../i18n';

const i18n = defineMessages({
  title: {
    id: 'showThinkingToggle.title',
    defaultMessage: 'Always Expand Thinking Traces',
  },
  description: {
    id: 'showThinkingToggle.description',
    defaultMessage: 'Keep thinking traces expanded by default instead of collapsed',
  },
});

export const ShowThinkingToggle = () => {
  const intl = useIntl();
  const { config, upsert } = useConfig();
  const [showThinking, setShowThinking] = useState(false);

  useEffect(() => {
    setShowThinking(config['GOOSE_SHOW_THINKING'] === true);
  }, [config['GOOSE_SHOW_THINKING']]);

  const handleToggle = async (checked: boolean) => {
    setShowThinking(checked);
    await upsert('GOOSE_SHOW_THINKING', checked, false);
  };

  return (
    <div className="flex items-center justify-between py-2 px-2 hover:bg-background-secondary rounded-lg transition-all">
      <div>
        <h3 className="text-text-primary">{intl.formatMessage(i18n.title)}</h3>
        <p className="text-xs text-text-secondary max-w-md mt-[2px]">
          {intl.formatMessage(i18n.description)}
        </p>
      </div>
      <Switch checked={showThinking} onCheckedChange={handleToggle} variant="mono" />
    </div>
  );
};
