import { FaKey, FaExclamationCircle, FaPencilAlt, FaTrash, FaPlus } from 'react-icons/fa';
import React from 'react';

export const ProviderCard = ({
  provider,
  secrets,
  isExpanded,
  isSupported,
  isChangingProvider,
  toggleProvider,
  handleAddOrEditKey,
  handleDeleteKey,
  handleSelectProvider,
  getProviderStatus,
}) => {
  const hasUnsetKeys = getProviderStatus(provider);

  return (
    <div key={provider.id} className="border dark:border-gray-700 rounded-lg p-4">
      <div className="flex items-center justify-between">
        <button
          className="flex-1 flex items-center justify-between"
          onClick={() => isSupported && toggleProvider(provider.id)}
        >
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center">
              <FaKey className={`${isSupported ? 'text-gray-500' : 'text-red-500'}`} />
            </div>
            <div className="text-left">
              <div className="flex items-center gap-2">
                <h3 className="font-medium dark:text-white">{provider.name}</h3>
                {provider.id.toLowerCase() ===
                  localStorage.getItem('GOOSE_PROVIDER')?.toLowerCase() && (
                  <span className="text-xs px-2 py-0.5 bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200 rounded-full">
                    Selected Provider
                  </span>
                )}
                {!isSupported && (
                  <span className="text-xs px-2 py-0.5 bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200 rounded-full">
                    Not Supported
                  </span>
                )}
              </div>
              <p className="text-sm text-gray-500">
                {isSupported ? provider.description : 'Provider not supported'}
              </p>
            </div>
          </div>
          {isSupported && hasUnsetKeys && <FaExclamationCircle className="text-yellow-500" />}
        </button>
      </div>

      {isSupported && isExpanded && (
        <div className="mt-4 pl-11">
          {provider.keys.map((key) => {
            const secret = secrets.find((s) => s.key === key);
            return (
              <div key={key} className="py-2 flex items-center justify-between">
                <div>
                  <p className="text-sm font-mono dark:text-gray-300">{key}</p>
                  <p className="text-xs text-gray-500">Source: {secret?.location || 'none'}</p>
                </div>
                <div className="flex items-center gap-2">
                  <span
                    className={`px-2 py-1 rounded text-xs ${
                      secret?.is_set
                        ? 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200'
                        : 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200'
                    }`}
                  >
                    {secret?.is_set ? 'Key set' : 'Missing'}
                  </span>
                  <button
                    onClick={() => handleAddOrEditKey(key, provider.name)}
                    className="p-1.5 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-full hover:bg-gray-100 dark:hover:bg-gray-700"
                    title={secret?.is_set ? 'Edit key' : 'Add key'}
                  >
                    {secret?.is_set ? <FaPencilAlt size={14} /> : <FaPlus size={14} />}
                  </button>
                  <button
                    onClick={() => handleDeleteKey(provider.id, key)}
                    className={`p-1.5 rounded-full ${
                      secret?.is_set
                        ? 'text-red-500 hover:text-red-700 dark:text-red-400 dark:hover:text-red-200 hover:bg-red-100 dark:hover:bg-red-900'
                        : 'text-gray-300 dark:text-gray-600 cursor-not-allowed'
                    }`}
                    title={
                      secret?.is_set
                        ? 'Delete key from keychain'
                        : 'No key to delete - Add a key first before deleting'
                    }
                    disabled={!secret?.is_set}
                    aria-disabled={!secret?.is_set}
                  >
                    <FaTrash size={14} />
                  </button>
                </div>
              </div>
            );
          })}

          {provider.id.toLowerCase() !== localStorage.getItem('GOOSE_PROVIDER')?.toLowerCase() && (
            <button
              onClick={() => handleSelectProvider(provider.id)}
              disabled={isChangingProvider}
              className="mt-4 text-sm px-2 py-1 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50"
            >
              Set as Active
            </button>
          )}
        </div>
      )}
    </div>
  );
};
