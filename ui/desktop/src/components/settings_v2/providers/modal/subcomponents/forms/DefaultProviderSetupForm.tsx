import React from 'react';
import { Input } from '../../../../../ui/input';
import { Lock } from 'lucide-react';
import ProviderSetupFormProps from '../../interfaces/ProviderSetupFormProps';
import ParameterSchema from '../../../interfaces/ParameterSchema';
import { PROVIDER_REGISTRY } from '../../../ProviderRegistry';

export default function DefaultProviderSetupForm({
  configValues,
  setConfigValues,
  onSubmit,
  provider,
}: ProviderSetupFormProps) {
  const providerEntry = PROVIDER_REGISTRY.find((p) => p.name === provider.name);
  const parameters: ParameterSchema[] = providerEntry.details.parameters;

  return (
    <form onSubmit={onSubmit}>
      <div className="mt-4 space-y-4">
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
              placeholder={parameter.name.replace(/_/g, ' ')}
              className="w-full h-14 px-4 font-regular rounded-lg border shadow-none border-gray-300 bg-white text-lg placeholder:text-gray-400 font-regular text-gray-900"
              required
            />
          </div>
        ))}

        <div className="flex items-start mt-2 text-gray-600 dark:text-gray-300">
          <Lock className="w-5 h-5 mt-1" />
          <span className="text-sm font-light ml-2">Keys are stored in a secure .env file</span>
        </div>
      </div>
    </form>
  );
}
