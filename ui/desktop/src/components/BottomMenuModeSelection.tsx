import React from 'react';
import { getApiUrl, getSecretKey } from '../config';

export const BottomMenuModeSelection = ({ selectedMode, setSelectedMode }) => {
  const modes = [
    {
      value: 'auto',
    },
    {
      value: 'approve',
    },
    {
      value: 'chat',
    },
  ];

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
    setSelectedMode(newMode);
  };

  return (
    <div className="absolute bottom-[24px] right-0 w-[120px] bg-bgApp rounded-lg border border-borderSubtle">
      <div>
        {modes.map((mode) => (
          <label key={mode.value} className="block cursor-pointer">
            <div
              className="flex items-center justify-between p-2 text-textStandard hover:bg-bgSubtle transition-colors"
              onClick={() => handleModeChange(mode.value)}
            >
              <div>
                <p className="text-sm">{mode.value}</p>
              </div>
              <div className="relative">
                <input
                  type="radio"
                  name="modes"
                  value={mode.value}
                  checked={selectedMode === mode.value}
                  onChange={() => handleModeChange(mode.value)}
                  className="peer sr-only"
                />
                <div
                  className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500
                  peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                  peer-checked:bg-white dark:peer-checked:bg-black
                  transition-all duration-200 ease-in-out"
                ></div>
              </div>
            </div>
          </label>
        ))}
      </div>
    </div>
  );
};
