import { useEffect, useState, useCallback, useRef } from 'react';
import { View } from '../../../utils/navigationUtils';
import ModelSettingsButtons from './subcomponents/ModelSettingsButtons';
import { useConfig } from '../../ConfigContext';
import {
  UNKNOWN_PROVIDER_MSG,
  UNKNOWN_PROVIDER_TITLE,
  useModelAndProvider,
} from '../../ModelAndProviderContext';
import { toastError } from '../../../toasts';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../ui/card';
import ResetProviderSection from '../reset_provider/ResetProviderSection';
import { Switch } from '../../ui/switch';
import { trackSettingToggled } from '../../../utils/analytics';

const MODEL_LOCK_USER_PREF_KEY = 'GOOSE_MODEL_LOCK_USER_PREF';

interface ModelsSectionProps {
  setView: (view: View) => void;
}

export default function ModelsSection({ setView }: ModelsSectionProps) {
  const [provider, setProvider] = useState<string | null>(null);
  const [displayModelName, setDisplayModelName] = useState<string>('');
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [isModelLocked, setIsModelLocked] = useState<boolean>(false);
  const { read, upsert, getProviders } = useConfig();
  const {
    getCurrentModelDisplayName,
    getCurrentProviderDisplayName,
    currentModel,
    currentProvider,
  } = useModelAndProvider();

  useEffect(() => {
    const loadLockState = async () => {
      try {
        const savedState = await read(MODEL_LOCK_USER_PREF_KEY, false);
        
        if (savedState === null || savedState === undefined) {
          // No user preference, use env var default
          const envDefault = window.appConfig.get('GOOSE_MODEL_LOCK');
          setIsModelLocked(envDefault === true);
        } else {
          // User has saved preference
          const isLocked = savedState === true || savedState === 'true';
          setIsModelLocked(isLocked);
        }
      } catch (error) {
        console.error('Error loading model lock state:', error);
        setIsModelLocked(false);
      }
    };
    loadLockState();
  }, [read]);

  const handleLockToggle = async (checked: boolean) => {
    setIsModelLocked(checked);
    await upsert(MODEL_LOCK_USER_PREF_KEY, checked, false);
    trackSettingToggled('model_locked', checked);
    window.dispatchEvent(new CustomEvent('model-lock-changed'));
  };

  const loadModelData = useCallback(async () => {
    try {
      setIsLoading(true);

      const modelDisplayName = await getCurrentModelDisplayName();
      setDisplayModelName(modelDisplayName);

      const providerDisplayName = await getCurrentProviderDisplayName();
      if (providerDisplayName) {
        setProvider(providerDisplayName);
      } else {
        const gooseProvider = (await read('GOOSE_PROVIDER', false)) as string;
        const providers = await getProviders(true);
        const providerDetailsList = providers.filter((provider) => provider.name === gooseProvider);

        if (providerDetailsList.length != 1) {
          toastError({
            title: UNKNOWN_PROVIDER_TITLE,
            msg: UNKNOWN_PROVIDER_MSG,
          });
          setProvider(gooseProvider);
        } else {
          const fallbackProviderDisplayName = providerDetailsList[0].metadata.display_name;
          setProvider(fallbackProviderDisplayName);
        }
      }
    } catch (error) {
      console.error('Error loading model data:', error);
    } finally {
      setIsLoading(false);
    }
  }, [read, getProviders, getCurrentModelDisplayName, getCurrentProviderDisplayName]);

  useEffect(() => {
    loadModelData();
  }, [loadModelData]);

  const prevModelRef = useRef<string | null>(null);
  const prevProviderRef = useRef<string | null>(null);

  useEffect(() => {
    if (
      currentModel &&
      currentProvider &&
      (currentModel !== prevModelRef.current || currentProvider !== prevProviderRef.current)
    ) {
      prevModelRef.current = currentModel;
      prevProviderRef.current = currentProvider;
      loadModelData();
    }
  }, [currentModel, currentProvider, loadModelData]);

  return (
    <section id="models" className="space-y-4 pr-4">
      <Card className="p-2 pb-4">
        <CardContent className="px-2">
          {isLoading ? (
            <>
              <div className="h-[20px] mb-1"></div>
              <div className="h-[16px]"></div>
            </>
          ) : (
            <div className="animate-in fade-in duration-100">
              <h3 className="text-text-default">{displayModelName}</h3>
              <h4 className="text-xs text-text-muted">{provider}</h4>
            </div>
          )}
          <ModelSettingsButtons setView={setView} isModelLocked={isModelLocked} />
          <div className="flex items-center justify-between mt-4 pt-4 border-t">
            <div>
              <h3 className="text-text-default text-sm">Lock Model</h3>
              <p className="text-xs text-text-muted">Prevent accidental model changes</p>
            </div>
            <Switch checked={isModelLocked} onCheckedChange={handleLockToggle} variant="mono" />
          </div>
        </CardContent>
      </Card>
      <Card className="pb-2 rounded-lg">
        <CardHeader className="pb-0">
          <CardTitle className="">Reset Provider and Model</CardTitle>
          <CardDescription>
            Clear your selected model and provider settings to start fresh
          </CardDescription>
        </CardHeader>
        <CardContent className="px-2">
          <ResetProviderSection setView={setView} />
        </CardContent>
      </Card>
    </section>
  );
}
