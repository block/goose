import { PROVIDER_REGISTRY } from '../../../ProviderRegistry';
import { Input } from '../../../../../ui/input';

import React, { useState, useEffect, useCallback } from 'react';
import { RefreshCw } from 'lucide-react';
import CustomRadio from '../../../../../ui/CustomRadio';

export default function OllamaForm({
  configValues,
  setConfigValues,
  provider,
}: {
  configValues: Record<string, string>;
  setConfigValues: React.Dispatch<React.SetStateAction<Record<string, string>>>;
  provider: { name: string; [key: string]: unknown };
}) {
  const providerEntry = PROVIDER_REGISTRY.find((p) => p.name === provider.name);
  const parameters = providerEntry?.details?.parameters || [];
  const [isCheckingLocal, setIsCheckingLocal] = useState(false);
  const [isLocalAvailable, setIsLocalAvailable] = useState(false);

  const handleConnectionTypeChange = useCallback(
    (value: string) => {
      setConfigValues((prev) => ({
        ...prev,
        connection_type: value,
      }));
    },
    [setConfigValues]
  );

  // Function to handle input changes and auto-select/deselect the host radio
  const handleInputChange = (paramName: string, value: string) => {
    // Update the parameter value
    setConfigValues((prev) => ({
      ...prev,
      [paramName]: value,
    }));

    // If the user is typing, auto-select the host radio button
    if (value && configValues.connection_type !== 'host') {
      handleConnectionTypeChange('host');
    }
    // If the input becomes empty and the host radio is selected, switch to local if available
    else if (!value && configValues.connection_type === 'host') {
      if (isLocalAvailable) {
        handleConnectionTypeChange('local');
      }
      // If local is not available, we keep the host selected but leave the input empty
    }
  };

  const checkLocalAvailability = useCallback(async () => {
    setIsCheckingLocal(true);

    // Dummy implementation - simulates checking local availability
    try {
      console.log('Checking for local Ollama instance...');
      // Simulate a network request with a delay
      await new Promise((resolve) => setTimeout(resolve, 800));

      // Randomly determine if Ollama is available (for demo purposes)
      const isAvailable = Math.random() > 0.3;
      setIsLocalAvailable(isAvailable);

      if (isAvailable) {
        console.log('Local Ollama instance found');
        // Enable local radio button
      } else {
        console.log('No local Ollama instance found');
        // If current selection is local, switch to host
        if (configValues.connection_type === 'local') {
          handleConnectionTypeChange('host');
        }
      }
    } catch (error) {
      console.error('Error checking for local Ollama:', error);
      setIsLocalAvailable(false);
    } finally {
      setIsCheckingLocal(false);
    }
  }, [configValues.connection_type, handleConnectionTypeChange]);

  // Check local availability on initial load
  useEffect(() => {
    checkLocalAvailability();
  }, [checkLocalAvailability]);

  return (
    <div className="mt-4 space-y-4">
      <div className="font-medium text-gray-900 dark:text-gray-100 mb-2">Connection</div>

      {/* Local Option */}
      <div className="flex items-center mb-3 justify-between">
        <div className="flex items-center">
          <span className="text-gray-700 dark:text-gray-300">Background App</span>
          <button
            type="button"
            className="ml-2 p-1 rounded-full hover:bg-gray-100 dark:hover:bg-gray-800"
            onClick={checkLocalAvailability}
            disabled={isCheckingLocal}
          >
            <RefreshCw
              className={`w-4 h-4 ${isCheckingLocal ? 'animate-spin' : ''} text-gray-600 dark:text-gray-400`}
            />
          </button>
        </div>

        <CustomRadio
          id="connection-local"
          name="connection_type"
          value="local"
          checked={configValues.connection_type === 'local'}
          onChange={() => handleConnectionTypeChange('local')}
          disabled={!isLocalAvailable}
        />
      </div>

      {/* Other Parameters */}
      {parameters
        .filter((param) => param.name !== 'host_url') // Skip host_url as we handle it above
        .map((parameter) => (
          <div key={parameter.name} className="flex items-center mb-4">
            <div className="flex-grow">
              <Input
                type={parameter.is_secret ? 'password' : 'text'}
                value={configValues[parameter.name] || ''}
                onChange={(e) => handleInputChange(parameter.name, e.target.value)}
                placeholder={
                  parameter.default ? parameter.default : parameter.name.replace(/_/g, ' ')
                }
                className="w-full h-14 px-4 font-regular rounded-lg border shadow-none border-gray-300 dark:border-gray-700 bg-white dark:bg-gray-900 text-lg placeholder:text-gray-400 dark:placeholder:text-gray-500 font-regular text-gray-900 dark:text-gray-100"
                required={parameter.default == null}
              />
            </div>
            <div className="ml-4">
              <CustomRadio
                id={`connection-host-${parameter.name}`}
                name="connection_type"
                value="host"
                checked={configValues.connection_type === 'host'}
                onChange={() => handleConnectionTypeChange('host')}
              />
            </div>
          </div>
        ))}
    </div>
  );
}
