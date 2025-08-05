import { useState, useEffect } from 'react';
import { Input } from '../../ui/input';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import { cn } from '../../../utils';
import { Save, RotateCcw, FileText } from 'lucide-react';
import { toastSuccess, toastError } from '../../../toasts';
import { getUiNames, providerPrefixes } from '../../../utils/configUtils';
import type { ConfigData, ConfigValue } from '../../../types/config';

export default function ConfigSettings() {
  const { config, upsert } = useConfig();
  const typedConfig = config as ConfigData;
  const [configValues, setConfigValues] = useState<ConfigData>({});
  const [modified, setModified] = useState(false);
  const [saving, setSaving] = useState<string | null>(null);

  useEffect(() => {
    setConfigValues(typedConfig);
  }, [typedConfig]);

  const handleChange = (key: string, value: string) => {
    setConfigValues((prev: ConfigData) => ({
      ...prev,
      [key]: value,
    }));
    setModified(true);
  };

  const handleSave = async (key: string) => {
    setSaving(key);
    try {
      await upsert(key, configValues[key], false);
      toastSuccess({
        title: 'Configuration Updated',
        msg: `Successfully saved "${getUiNames(key)}"`,
      });
      setModified(false);
    } catch (error) {
      console.error('Failed to save config:', error);
      toastError({
        title: 'Save Failed',
        msg: `Failed to save "${getUiNames(key)}"`,
        traceback: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setSaving(null);
    }
  };

  const handleReset = () => {
    setConfigValues(typedConfig);
    setModified(false);
    toastSuccess({
      title: 'Configuration Reset',
      msg: 'All changes have been reverted',
    });
  };

  const currentProvider = typedConfig.GOOSE_PROVIDER || '';

  const currentProviderPrefixes = providerPrefixes[currentProvider] || [];

  const allProviderPrefixes = Object.values(providerPrefixes).flat();

  const providerSpecificEntries: [string, ConfigValue][] = [];
  const generalEntries: [string, ConfigValue][] = [];

  Object.entries(configValues).forEach(([key, value]) => {
    // skip secrets
    if (key === 'extensions' || key.includes('_KEY') || key.includes('_TOKEN')) {
      return;
    }

    const providerSpecific = allProviderPrefixes.some((prefix: string) => key.startsWith(prefix));

    if (providerSpecific) {
      if (currentProviderPrefixes.some((prefix: string) => key.startsWith(prefix))) {
        providerSpecificEntries.push([key, value]);
      }
    } else {
      generalEntries.push([key, value]);
    }
  });

  const configEntries = [...providerSpecificEntries, ...generalEntries];

  return (
    <section id="configEditor" className="px-8">
      <div className="flex justify-between items-center mb-2">
        <div className="flex items-center gap-2">
          <FileText className="text-iconStandard" size={20} />
          <h2 className="text-xl font-medium text-textStandard">Configuration</h2>
        </div>
        {modified && (
          <Button onClick={handleReset} variant="ghost" className="text-sm">
            <RotateCcw className="h-4 w-4 mr-2" />
            Reset
          </Button>
        )}
      </div>
      <div className="pb-8">
        <p className="text-sm text-textSubtle mb-6">
          Edit your goose config
          {currentProvider && ` (current settings for ${currentProvider})`}
        </p>

        <div className="space-y-3">
          {configEntries.length === 0 ? (
            <p className="text-textSubtle">No configuration settings found.</p>
          ) : (
            configEntries.map(([key, _value]) => (
              <div key={key} className="grid grid-cols-[200px_1fr_auto] gap-3 items-center">
                <label className="text-sm font-medium text-textStandard" title={key}>
                  {getUiNames(key)}
                </label>
                <Input
                  value={String(configValues[key] || '')}
                  onChange={(e) => handleChange(key, e.target.value)}
                  className={cn(
                    'text-textStandard border-borderSubtle hover:border-borderStandard',
                    configValues[key] !== typedConfig[key] && 'border-blue-500'
                  )}
                  placeholder={`Enter ${getUiNames(key).toLowerCase()}`}
                />
                <Button
                  onClick={() => handleSave(key)}
                  disabled={configValues[key] === typedConfig[key] || saving === key}
                  variant="ghost"
                  size="sm"
                  className="min-w-[60px]"
                >
                  {saving === key ? (
                    <span className="text-xs">Saving...</span>
                  ) : (
                    <Save className="h-4 w-4" />
                  )}
                </Button>
              </div>
            ))
          )}
        </div>
      </div>
    </section>
  );
}
