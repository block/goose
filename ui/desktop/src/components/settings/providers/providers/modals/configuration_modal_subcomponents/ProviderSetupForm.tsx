import React from 'react';
import { Input } from '../../../../../ui/input';
import { Lock } from 'lucide-react';
import { isSecretKey } from '../../../../api_keys/utils';
import ProviderSetupFormProps from '../interfaces/ProviderSetupFormProps';
import ParameterSchema from '../../interfaces/ParameterSchema';

/**
 * Renders the form with required input fields and the "lock" info row.
 * The submit/cancel buttons are in a separate ProviderSetupActions component.
 */
export default function ProviderSetupForm({
  configValues,
  setConfigValues,
  onSubmit,
  provider,
}: ProviderSetupFormProps) {
  const parameters: ParameterSchema[] = provider.parameters;
  return (
    <form onSubmit={onSubmit}>
      <div className="mt-[24px] space-y-4">
        {parameters.map((parameter) => (
          <div key={parameter.name}>
            <Input
              type={parameter.is_secret ? 'password' : 'text'}
              value={configValues[parameter.name] || ''}
              onChange={(e) =>
                setConfigValues((prev) => ({
                  ...prev,
                  [parameter.name]: e.target.value,
                }))
              }
              placeholder={parameter.name}
              className="w-full h-14 px-4 font-regular rounded-lg border shadow-none border-gray-300 bg-white text-lg placeholder:text-gray-400 font-regular text-gray-900"
              required
            />
          </div>
        ))}
        <div className="flex text-gray-600 dark:text-gray-300">
          <Lock className="w-6 h-6" />
          <span className="text-sm font-light ml-4 mt-[2px]">
            Your configuration values will be stored securely in the keychain and used only for
            making requests to {provider.name}.
          </span>
        </div>
      </div>
      {/* The action buttons are not in this form; they're in ProviderSetupActions. */}
    </form>
  );
}
