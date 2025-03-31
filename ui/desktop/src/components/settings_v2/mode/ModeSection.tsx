import React, { useEffect, useState } from 'react';
import { getApiUrl, getSecretKey } from '../../../config';
import { all_goose_modes, filterGooseModes, ModeSelectionItem } from './ModeSelectionItem';
import ExtensionList from '@/src/components/settings_v2/extensions/subcomponents/ExtensionList';
import { Button } from '@/src/components/ui/button';
import { Plus } from 'lucide-react';
import { GPSIcon } from '@/src/components/ui/icons';

export const ModeSection = () => {
  const [currentMode, setCurrentMode] = useState('auto');
  const [previousApproveModel, setPreviousApproveModel] = useState('');

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
    // Only track the previous approve if current mode is approve related but new mode is not.
    if (currentMode.includes('approve') && !newMode.includes('approve')) {
      setPreviousApproveModel(currentMode);
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
    <section id="mode">
      <div className="flex justify-between items-center mb-6 px-8">
        <h1 className="text-3xl font-medium text-textStandard">Mode</h1>
      </div>
      <div className="px-8">
        <p className="text-sm text-textStandard mb-6">
          Configure how Goose interacts with tools and extensions
        </p>
        <div>
          {filterGooseModes(currentMode, all_goose_modes, previousApproveModel).map((mode) => (
            <ModeSelectionItem
              key={mode.key}
              mode={mode}
              currentMode={currentMode}
              showDescription={true}
              isApproveModeConfigure={false}
              handleModeChange={handleModeChange}
            />
          ))}
        </div>
      </div>
    </section>
  );
};
