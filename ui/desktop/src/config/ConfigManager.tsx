import React, { useEffect, useState } from 'react';
import { ConfigAPI, ConfigResponse } from './api';

export const ConfigManager: React.FC = () => {
  const [config, setConfig] = useState<ConfigResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [newKey, setNewKey] = useState('');
  const [newValue, setNewValue] = useState('');

  useEffect(() => {
    loadConfig();
  }, []);

  const loadConfig = async () => {
    try {
      const data = await ConfigAPI.readAllConfig();
      setConfig(data);
      setError(null);
    } catch (err) {
      setError('Failed to load configuration');
      console.error(err);
    }
  };

  const handleUpsert = async () => {
    try {
      await ConfigAPI.upsertConfig({
        key: newKey,
        value: newValue as any, // You might want to add proper parsing here
      });
      await loadConfig();
      setNewKey('');
      setNewValue('');
      setError(null);
    } catch (err) {
      setError('Failed to update configuration');
      console.error(err);
    }
  };

  const handleRemove = async (key: string) => {
    try {
      await ConfigAPI.removeConfig(key);
      await loadConfig();
      setError(null);
    } catch (err) {
      setError('Failed to remove configuration');
      console.error(err);
    }
  };

  return (
    <div className="p-4">
      <h2 className="text-2xl font-bold mb-4">Configuration Manager</h2>

      {error && (
        <div className="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
          {error}
        </div>
      )}

      <div className="mb-4">
        <h3 className="text-lg font-semibold mb-2">Add/Update Configuration</h3>
        <div className="flex gap-2">
          <input
            type="text"
            value={newKey}
            onChange={(e) => setNewKey(e.target.value)}
            placeholder="Key"
            className="border p-2 rounded"
          />
          <input
            type="text"
            value={newValue}
            onChange={(e) => setNewValue(e.target.value)}
            placeholder="Value"
            className="border p-2 rounded"
          />
          <button onClick={handleUpsert} className="bg-blue-500 text-white px-4 py-2 rounded">
            Save
          </button>
        </div>
      </div>

      <div>
        <h3 className="text-lg font-semibold mb-2">Current Configuration</h3>
        {config && (
          <div className="border rounded">
            {Object.entries(config.config).map(([key, value]) => (
              <div key={key} className="p-2 border-b flex justify-between items-center">
                <div>
                  <span className="font-medium">{key}:</span> <span>{JSON.stringify(value)}</span>
                </div>
                <button
                  onClick={() => handleRemove(key)}
                  className="text-red-500 hover:text-red-700"
                >
                  Remove
                </button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
