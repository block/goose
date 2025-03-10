import React, { useEffect, useState } from 'react';
import { Switch } from '../../ui/switch';
import { ChevronDown, ChevronUp } from '../../icons';
import { getApiUrl, getSecretKey } from '../../../config';

function formatString(input: string): string {
  return input
    .toLowerCase() // Convert to lowercase
    .split('_') // Split by underscores
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1)) // Capitalize each word
    .join(' '); // Join with spaces
}

export const ExperimentSettings = () => {
  const [experiments, setExperiments] = useState([]);
  const [isExperimentsOpen, setIsExperimentsOpen] = useState(false);

  // Descriptions for experiments
  const descriptions = {
    GOOSE_SMART_APPROVE:
      'Smart approve helps your skip tool confirmation for read tool operation when Goose mode is approve.',
  };

  const handleToggle = async (key, enabled) => {
    try {
      const storeResponse = await fetch(getApiUrl('/config/experiment'), {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({
          key,
          value: enabled,
        }),
      });

      if (!storeResponse.ok) {
        const errorText = await storeResponse.text();
        console.error('Toggle experiment error:', errorText);
        throw new Error(`Failed to toggle experiment for ${key}`);
      }

      // Update the local state
      setExperiments((prevExperiments) =>
        prevExperiments.map(([experimentKey, value]) =>
          experimentKey === key ? [experimentKey, enabled] : [experimentKey, value]
        )
      );
    } catch (error) {
      console.error(`Error toggling experiment ${key}:`, error);
    }
  };

  useEffect(() => {
    const fetchExperiments = async () => {
      try {
        const response = await fetch(getApiUrl('/config'), {
          method: 'GET',
          headers: {
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
        });

        if (response.ok) {
          const data = await response.json();
          const experiments = Object.entries(data.config.experiments || {}).map(([key, value]) => [
            key,
            Boolean(value),
          ]);
          setExperiments(experiments);
        } else {
          console.error('Failed to fetch experiments:', await response.text());
        }
      } catch (error) {
        console.error('Error fetching experiments:', error);
      }
    };

    fetchExperiments();
  }, []);

  return (
    <div>
      <div className="mt-4 mb-4 flex justify-left items-center">
        <h4 className="font-medium text-textStandard">Experiments (Advanced)</h4>
        <div
          className="flex items-center cursor-pointer"
          onClick={() => setIsExperimentsOpen(!isExperimentsOpen)}
        >
          {isExperimentsOpen ? (
            <ChevronDown className="w-4 h-4 ml-1 text-textStandard" />
          ) : (
            <ChevronUp className="w-4 h-4 ml-1 text-textStandard" />
          )}
        </div>
      </div>

      {isExperimentsOpen && (
        <div>
          {experiments.map(([key, enabled]) => (
            <div key={key} className="flex justify-between items-center mt-4">
              <div className="flex flex-col text-left">
                <h3 className="text-sm font-semibold text-textStandard dark:text-gray-200">
                  {formatString(key)}
                </h3>
                <p className="text-xs text-textSubtle dark:text-gray-400 mt-[2px]">
                  {descriptions[key] || 'No description available.'}
                </p>
              </div>
              <button
                onClick={() => handleToggle(key, !enabled)}
                className={`relative inline-flex h-6 w-11 items-center rounded-full ${
                  enabled ? 'bg-indigo-500' : 'bg-bgProminent'
                } transition-colors duration-200 ease-in-out focus:outline-none`}
              >
                <span
                  className={`inline-block h-5 w-5 transform rounded-full bg-white shadow ${
                    enabled ? 'translate-x-[22px]' : 'translate-x-[2px]'
                  } transition-transform duration-200 ease-in-out`}
                />
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
