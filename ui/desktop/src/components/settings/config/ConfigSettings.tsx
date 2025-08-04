import { useState, useEffect } from 'react';
import { Input } from '../../ui/input';
import { Button } from '../../ui/button';
import { useConfig } from '../../ConfigContext';
import { cn } from '../../../utils';
import { Save, RotateCcw, FileText, Settings } from 'lucide-react';
import { toastSuccess, toastError } from '../../../toasts';
import { getUiNames, providerPrefixes } from '../../../utils/configUtils';
import type { ConfigData, ConfigValue } from '../../../types/config';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '../../ui/dialog';

export default function ConfigSettings() {
  const { config, upsert } = useConfig();
  const typedConfig = config as ConfigData;
  const [configValues, setConfigValues] = useState<ConfigData>({});
  const [modified, setModified] = useState(false);
  const [saving, setSaving] = useState<string | null>(null);
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [originalKeyOrder, setOriginalKeyOrder] = useState<string[]>([]);

  useEffect(() => {
    setConfigValues(typedConfig);

    // Capture the original key order only on first load or when keys change significantly
    if (
      originalKeyOrder.length === 0 ||
      Object.keys(typedConfig).length !== originalKeyOrder.length
    ) {
      setOriginalKeyOrder(Object.keys(typedConfig));
    }
  }, [typedConfig, originalKeyOrder.length]);

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

  // Preserve the original order of configuration keys
  const configEntries: [string, ConfigValue][] = originalKeyOrder
    .filter((key) => {
      // skip secrets
      if (key === 'extensions' || key.includes('_KEY') || key.includes('_TOKEN')) {
        return false;
      }

      // Only show provider-specific entries for the current provider
      const providerSpecific = allProviderPrefixes.some((prefix: string) => key.startsWith(prefix));
      if (providerSpecific) {
        return currentProviderPrefixes.some((prefix: string) => key.startsWith(prefix));
      }

      return true;
    })
    .map((key) => [key, configValues[key]]);

  return (
    <Card className="rounded-lg">
      <CardHeader className="pb-0">
        <CardTitle className="flex items-center gap-2">
          <FileText className="text-iconStandard" size={20} />
          Configuration
        </CardTitle>
        <CardDescription>
          Edit your goose configuration settings
          {currentProvider && ` (current settings for ${currentProvider})`}
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-4 px-4">
        <Dialog open={isModalOpen} onOpenChange={setIsModalOpen}>
          <DialogTrigger asChild>
            <Button className="flex items-center gap-2" variant="secondary" size="sm">
              <Settings className="h-4 w-4" />
              Edit Configuration
            </Button>
          </DialogTrigger>
          <DialogContent className="max-w-4xl max-h-[80vh]">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <FileText className="text-iconStandard" size={20} />
                Configuration Editor
              </DialogTitle>
              <DialogDescription>
                Edit your goose configuration settings
                {currentProvider && ` (current settings for ${currentProvider})`}
              </DialogDescription>
            </DialogHeader>

            <div className="flex-1 max-h-[60vh] overflow-auto pr-4">
              <div className="space-y-4">
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

            <DialogFooter className="gap-2">
              {modified && (
                <Button onClick={handleReset} variant="outline">
                  <RotateCcw className="h-4 w-4 mr-2" />
                  Reset Changes
                </Button>
              )}
              <Button onClick={() => setIsModalOpen(false)} variant="default">
                Done
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </CardContent>
    </Card>
  );
}
