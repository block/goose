import { Input } from '../../../ui/input';
import { useTranslation } from 'react-i18next';

interface ExtensionConfigFieldsProps {
  type: 'stdio' | 'sse' | 'streamable_http' | 'builtin';
  full_cmd: string;
  endpoint: string;
  onChange: (key: string, value: string) => void;
  submitAttempted?: boolean;
  isValid?: boolean;
}

export default function ExtensionConfigFields({
  type,
  full_cmd,
  endpoint,
  onChange,
  submitAttempted = false,
  isValid,
}: ExtensionConfigFieldsProps) {
  const { t } = useTranslation();
  if (type === 'stdio') {
    return (
      <div className="space-y-4">
        <div>
          <label className="text-sm font-medium mb-2 block text-text-primary">
            {t('extensionModal.command')}
          </label>
          <div className="relative">
            <Input
              data-testid="extension-command-input"
              value={full_cmd}
              onChange={(e) => onChange('cmd', e.target.value)}
              placeholder={t('extensionModal.commandPlaceholder')}
              className={`w-full ${!submitAttempted || isValid ? 'border-border-primary' : 'border-red-500'} text-text-primary`}
            />
            {submitAttempted && !isValid && (
              <div className="absolute text-xs text-red-500 mt-1">
                {t('extensionModal.commandRequired')}
              </div>
            )}
          </div>
        </div>
      </div>
    );
  } else {
    return (
      <div>
        <label className="text-sm font-medium mb-2 block text-text-primary">
          {t('extensionModal.endpoint')}
        </label>
        <div className="relative">
          <Input
            data-testid="extension-endpoint-input"
            value={endpoint}
            onChange={(e) => onChange('endpoint', e.target.value)}
            placeholder={t('extensionModal.endpointPlaceholder')}
            className={`w-full ${!submitAttempted || isValid ? 'border-border-primary' : 'border-red-500'} text-text-primary`}
          />
          {submitAttempted && !isValid && (
            <div className="absolute text-xs text-red-500 mt-1">
              {t('extensionModal.endpointRequired')}
            </div>
          )}
        </div>
      </div>
    );
  }
}
