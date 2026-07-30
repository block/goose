import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { defineMessages, useIntl } from '../../../i18n';

const i18n = defineMessages({
  title: {
    id: 'enterInsertsNewlineToggle.title',
    defaultMessage: 'Enter Inserts Newline',
  },
  description: {
    id: 'enterInsertsNewlineToggle.description',
    defaultMessage:
      'Pressing Enter starts a new line instead of sending. Send with Shift+Enter or {sendShortcut}.',
  },
});

export const EnterInsertsNewlineToggle = () => {
  const intl = useIntl();
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    const loadState = async () => {
      const state = await window.electron.getSetting('enterInsertsNewline');
      setEnabled(state === true);
    };
    loadState();
  }, []);

  const handleToggle = async (checked: boolean) => {
    setEnabled(checked);
    await window.electron.setSetting('enterInsertsNewline', checked);
    window.dispatchEvent(new CustomEvent('enterInsertsNewlineChanged'));
  };

  const sendShortcut = window.electron?.platform === 'darwin' ? '⌘+Enter' : 'Ctrl+Enter';

  return (
    <div className="flex items-center justify-between py-2 px-2 hover:bg-background-secondary rounded-lg transition-all">
      <div>
        <h3 className="text-text-primary">{intl.formatMessage(i18n.title)}</h3>
        <p className="text-xs text-text-secondary max-w-md mt-[2px]">
          {intl.formatMessage(i18n.description, { sendShortcut })}
        </p>
      </div>
      <div className="flex items-center">
        <Switch checked={enabled} onCheckedChange={handleToggle} variant="mono" />
      </div>
    </div>
  );
};
