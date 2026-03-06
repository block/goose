import { Input } from '../../../ui/input';
import { useLocalization } from '../../../../contexts/LocalizationContext';

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
  const { t } = useLocalization();
  if (type === 'stdio') {
    return (
      <div className="space-y-4">
        <div>
          <label className="text-sm font-medium mb-2 block text-text-primary">
            {t('extensions.fields.command')}
          </label>
          <div className="relative">
            <Input
              value={full_cmd}
              onChange={(e) => onChange('cmd', e.target.value)}
              placeholder={t('extensions.fields.commandPlaceholder')}
              className={`w-full ${!submitAttempted || isValid ? 'border-border-primary' : 'border-red-500'} text-text-primary`}
            />
            {submitAttempted && !isValid && (
              <div className="absolute text-xs text-red-500 mt-1">
                {t('extensions.fields.commandRequired')}
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
          {t('extensions.fields.endpoint')}
        </label>
        <div className="relative">
          <Input
            value={endpoint}
            onChange={(e) => onChange('endpoint', e.target.value)}
            placeholder={t('extensions.fields.endpointPlaceholder')}
            className={`w-full ${!submitAttempted || isValid ? 'border-border-primary' : 'border-red-500'} text-text-primary`}
          />
          {submitAttempted && !isValid && (
            <div className="absolute text-xs text-red-500 mt-1">
              {t('extensions.fields.endpointRequired')}
            </div>
          )}
        </div>
      </div>
    );
  }
}
