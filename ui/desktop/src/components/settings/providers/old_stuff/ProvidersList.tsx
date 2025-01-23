import React from 'react';

export const ProviderList = ({ providers, onProviderSelect }) => (
  <div className="grid grid-cols-1 gap-4">
    {providers.map((provider) => (
      <button
        key={provider.id}
        onClick={() => onProviderSelect(provider)}
        className="p-4 pt-3 border rounded-lg hover:border-blue-500 transition-colors text-left dark:border-gray-700 dark:hover:border-blue-400"
      >
        <h3 className="text-lg font-regular mb-1 dark:text-gray-200">{provider.name}</h3>
        <p className="font-light text-gray-600 dark:text-gray-400">{provider.description}</p>
      </button>
    ))}
  </div>
);
