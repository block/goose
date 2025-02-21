import React from 'react';
import { Card } from '../ui/card';
import { Lock } from 'lucide-react';
import { Input } from '../ui/input';
import { Button } from '../ui/button';
import { required_keys, default_key_value } from './models/hardcoded_stuff';
import { isSecretKey } from './api_keys/utils';
import { OllamaBattleGame } from './OllamaBattleGame';

interface ProviderSetupModalProps {
  provider: string;
  _model: string;
  _endpoint: string;
  title?: string;
  onSubmit: (configValues: { [key: string]: string }) => void;
  onCancel: () => void;
  forceBattle?: boolean;
}

export function ProviderSetupModal({
  provider,
  _model,
  _endpoint,
  title,
  onSubmit,
  onCancel,
  forceBattle = false,
}: ProviderSetupModalProps) {
  const [configValues, setConfigValues] = React.useState<{ [key: string]: string }>(
    default_key_value
  );
  const requiredKeys = required_keys[provider] || ['API Key'];
  const headerText = title || `Setup ${provider}`;

  const shouldShowBattle = React.useMemo(() => {
    if (forceBattle) return true;
    if (provider.toLowerCase() !== 'ollama') return false;

    const now = new Date();
    return now.getMinutes() === 0;
  }, [provider, forceBattle]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubmit(configValues);
  };

  return (
    <div className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors animate-[fadein_200ms_ease-in_forwards]">
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] bg-bgApp rounded-xl overflow-hidden shadow-none p-[16px] pt-[24px] pb-0">
        <div className="px-4 pb-0 space-y-8">
          {/* Header */}
          <div className="flex">
            <h2 className="text-2xl font-regular text-textStandard">{headerText}</h2>
          </div>

          {provider.toLowerCase() === 'ollama' && shouldShowBattle ? (
            <OllamaBattleGame onComplete={onSubmit} requiredKeys={requiredKeys} />
          ) : (
            <form onSubmit={handleSubmit}>
              <div className="mt-[24px] space-y-4">
                {requiredKeys.map((keyName) => (
                  <div key={keyName}>
                    <Input
                      type={isSecretKey(keyName) ? 'password' : 'text'}
                      value={configValues[keyName] || ''}
                      onChange={(e) =>
                        setConfigValues((prev) => ({
                          ...prev,
                          [keyName]: e.target.value,
                        }))
                      }
                      placeholder={keyName}
                      className="w-full h-14 px-4 font-regular rounded-lg border shadow-none border-gray-300 bg-white text-lg placeholder:text-gray-400 font-regular text-gray-900"
                      required
                    />
                  </div>
                ))}
                <div
                  className="flex text-gray-600 dark:text-gray-300"
                  onClick={() => {
                    if (provider.toLowerCase() === 'ollama') {
                      onCancel();
                      onSubmit({ forceBattle: 'true' });
                    }
                  }}
                >
                  <Lock className="w-6 h-6" />
                  <span className="text-sm font-light ml-4 mt-[2px]">{`Your configuration values will be stored securely in the keychain and used only for making requests to ${provider}`}</span>
                </div>
              </div>

              {/* Actions */}
              <div className="mt-[8px] -ml-8 -mr-8 pt-8">
                <Button
                  type="submit"
                  variant="ghost"
                  className="w-full h-[60px] rounded-none border-t border-borderSubtle text-md hover:bg-bgSubtle text-textProminent font-regular"
                >
                  Submit
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  onClick={onCancel}
                  className="w-full h-[60px] rounded-none border-t border-borderSubtle hover:text-textStandard text-textSubtle hover:bg-bgSubtle text-md font-regular"
                >
                  Cancel
                </Button>
              </div>
            </form>
          )}
        </div>
      </Card>
    </div>
  );
}
