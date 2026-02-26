import { useEffect, useState } from 'react';
import { Switch } from '@/components/atoms/switch';

export const SpellcheckToggle = () => {
  const [enabled, setEnabled] = useState(true);

  useEffect(() => {
    const loadState = async () => {
      const state = await window.electron.getSpellcheckState();
      setEnabled(state);
    };
    loadState();
  }, []);

  const handleToggle = async (checked: boolean) => {
    setEnabled(checked);
    await window.electron.setSpellcheck(checked);
  };

  return (
    <div className="flex items-center justify-between py-2 px-2 hover:bg-background-muted rounded-lg transition-all">
      <div>
        <h3 className="text-text-default">Enable Spellcheck</h3>
        <p className="text-xs text-text-muted max-w-md mt-[2px]">
          Check spelling in the chat input. Requires restart to take effect.
        </p>
      </div>
      <div className="flex items-center">
        <Switch checked={enabled} onCheckedChange={handleToggle} variant="mono" />
      </div>
    </div>
  );
};
