import { useState, useEffect } from 'react';
import { Switch } from '../../ui/switch';
import { useLocalization } from '../../../contexts/LocalizationContext';

export const SpellcheckToggle = () => {
  const { t } = useLocalization();
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
    <div className="flex items-center justify-between py-2 px-2 hover:bg-background-secondary rounded-lg transition-all">
      <div>
        <h3 className="text-text-primary">{t('spellcheck.title')}</h3>
        <p className="text-xs text-text-secondary max-w-md mt-[2px]">
          {t('spellcheck.description')}
        </p>
      </div>
      <div className="flex items-center">
        <Switch checked={enabled} onCheckedChange={handleToggle} variant="mono" />
      </div>
    </div>
  );
};
