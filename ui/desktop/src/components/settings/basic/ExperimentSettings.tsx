import React, { useEffect, useState } from 'react';
import { Switch } from '../../ui/switch';
import { ChevronDown, ChevronUp } from '../../icons';
import { getApiUrl, getSecretKey } from '../../../config';

export const ExperimentSettings = () => {
  const [experiments, setExperiments] = useState([]);
  const [isExperimentsOpen, setIsExperimentsOpen] = useState(false);

  // Descriptions for experiments
  const descriptions = {
    GOOSE_SMART_APPROVE:
      'Smart approve helps your skip tool confirmation for read tool operation when Goose mode is approve.',
  };

  const handleToggle = async (key, checked) => {
    try {
      const storeResponse = await fetch(getApiUrl('/config/experiment'), {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({
          key,
          value: checked,
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
          experimentKey === key ? [experimentKey, checked] : [experimentKey, value]
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
          {experiments.map(([key, value]) => (
            <div key={key} className="flex justify-between items-center mt-4">
              <div className="flex flex-col text-left">
                <h3 className="text-sm font-semibold text-textStandard dark:text-gray-200">
                  {key}
                </h3>
                <p className="text-xs text-textSubtle dark:text-gray-400 mt-[2px]">
                  {descriptions[key] || 'No description available.'}
                </p>
              </div>
              <Switch
                variant="mono"
                checked={value}
                onCheckedChange={(checked) => handleToggle(key, checked)}
                className="ml-4"
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
