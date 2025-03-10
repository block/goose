import * as RadioGroup from '@radix-ui/react-radio-group';
import React, { useEffect, useState } from 'react';
import { getApiUrl, getSecretKey } from '../../../config';

export const ModeSelection = () => {
  const modes = [
    {
      value: 'auto',
      label: 'Completely autonomous',
      description: 'Full file modification capabilities, edit, create, and delete files freely.',
    },
    {
      value: 'approve',
      label: 'Approval needed',
      description:
        'Classifies the tool as either a read-only tool or write tool. Write tools will ask for human approval.',
    },
    {
      value: 'chat',
      label: 'Chat only',
      description: 'Engage with the selected provider without using tools or extensions.',
    },
  ];

  const [currentMode, setCurrentMode] = useState('auto');

  const handleModeChange = async (newMode: string) => {
    const storeResponse = await fetch(getApiUrl('/configs/store'), {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Secret-Key': getSecretKey(),
      },
      body: JSON.stringify({
        key: 'GOOSE_MODE',
        value: newMode,
        isSecret: false,
      }),
    });

    if (!storeResponse.ok) {
      const errorText = await storeResponse.text();
      console.error('Store response error:', errorText);
      throw new Error(`Failed to store new goose mode: ${newMode}`);
    }
    setCurrentMode(newMode);
  };

  useEffect(() => {
    const fetchCurrentMode = async () => {
      try {
        const response = await fetch(getApiUrl('/configs/get?key=GOOSE_MODE'), {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
        });

        if (response.ok) {
          const { value } = await response.json();
          if (value) {
            setCurrentMode(value);
          }
        }
      } catch (error) {
        console.error('Error fetching current mode:', error);
      }
    };

    fetchCurrentMode();
  }, []);

  return (
    <div>
      <h4 className="font-medium mb-4 text-textStandard">Mode Selection</h4>

      <RadioGroup.Root
        className="flex flex-col space-y-2"
        value={currentMode}
        onValueChange={handleModeChange}
      >
        {modes.map((mode) => (
          <RadioGroup.Item
            key={mode.value}
            value={mode.value}
            className="flex items-center justify-between p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded transition-all cursor-pointer"
          >
            <div className="flex flex-col text-left">
              <h3 className="text-sm font-semibold text-textStandard dark:text-gray-200">
                {mode.label}
              </h3>
              <p className="text-xs text-textSubtle dark:text-gray-400 mt-[2px]">
                {mode.description}
              </p>
            </div>
            <div className="flex-shrink-0">
              <div className="w-4 h-4 flex items-center justify-center rounded-full border border-gray-500 dark:border-gray-400">
                {currentMode === mode.value && (
                  <div className="w-2 h-2 bg-black dark:bg-white rounded-full" />
                )}
              </div>
            </div>
          </RadioGroup.Item>
        ))}
      </RadioGroup.Root>
    </div>
  );
};
