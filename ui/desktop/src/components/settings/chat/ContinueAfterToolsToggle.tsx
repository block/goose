import { Switch } from '../../ui/switch';
import { useConfig } from '../../ConfigContext';
import { defineMessages, useIntl } from '../../../i18n';

const CONFIG_KEY = 'GOOSE_CONTINUE_AFTER_TOOLS';

const i18n = defineMessages({
  title: {
    id: 'continueAfterToolsToggle.title',
    defaultMessage: 'Continue After Tools',
  },
  description: {
    id: 'continueAfterToolsToggle.description',
    defaultMessage:
      'When Goose stops mid-task after using tools, nudge it once to keep going. Off by default.',
  },
});

export const ContinueAfterToolsToggle = () => {
  const intl = useIntl();
  const { config, upsert } = useConfig();
  const enabled = Boolean(config[CONFIG_KEY]);

  const handleToggle = async (checked: boolean) => {
    try {
      await upsert(CONFIG_KEY, checked, false);
    } catch (error) {
      console.error('Error updating continue-after-tools setting:', error);
    }
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
