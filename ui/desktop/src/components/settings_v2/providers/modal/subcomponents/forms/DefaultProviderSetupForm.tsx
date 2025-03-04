import React from 'react';
import { Input } from '../../../../../ui/input';
import { Lock } from 'lucide-react';
import { PROVIDER_REGISTRY } from '../../../ProviderRegistry';

export default function DefaultProviderSetupForm({ configValues, setConfigValues, provider }) {
  const providerEntry = PROVIDER_REGISTRY.find((p) => p.name === provider.name);
  const parameters = providerEntry?.details?.parameters || [];

  return (
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
    </div>
  );
}
