import React, { useEffect, useState } from 'react';
import { getApiUrl, getSecretKey } from '../../config';

interface SecretSource {
  key: string;
  source: string;
  is_set: boolean;
}

interface SecretsListResponse {
  secrets: SecretSource[];
}

type SVGComponentProps = {
  className?: string;
  width?: number | string;
  height?: number | string;
  fill?: string;
  stroke?: string;
  strokeWidth?: number | string;
  viewBox?: string;
  xmlns?: string;
  // Add any other specific props you need
};

export const SecretsList = () => {
  const [secrets, setSecrets] = useState<SecretSource[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchSecrets();
  }, []);

  const fetchSecrets = async () => {
    try {
      const response = await fetch(getApiUrl('/secrets/list'), {
        headers: {
          'X-Secret-Key': getSecretKey(),
        },
      });
      
      if (!response.ok) {
        throw new Error('Failed to fetch secrets');
      }
      
      const data: SecretsListResponse = await response.json();
      setSecrets(data.secrets);
    } catch (error) {
      console.error('Error fetching secrets:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleAddKey = async (key: string, value: string) => {
    try {
      const response = await fetch(getApiUrl('/secrets/store'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify({ key, value }),
      });

      if (!response.ok) {
        throw new Error('Failed to store secret');
      }

      // Refresh the secrets list
      fetchSecrets();
    } catch (error) {
      console.error('Error storing secret:', error);
    }
  };

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h2 className="text-2xl font-semibold">Environment Variables</h2>
        <button
          onClick={() => {/* Open add key modal */}}
          className="px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600"
        >
          Add Key
        </button>
      </div>

      <div className="space-y-4">
        {secrets.map((secret) => (
          <div
            key={secret.key}
            className="flex items-center justify-between p-4 bg-white dark:bg-gray-800 rounded-lg shadow"
          >
            <div>
              <h3 className="text-lg font-medium">{secret.key}</h3>
              <p className="text-sm text-gray-500">
                {secret.is_set ? (
                  <span className="text-green-500">
                    ✓ Set from {secret.source}
                  </span>
                ) : (
                  <span className="text-gray-500">Not set</span>
                )}
              </p>
            </div>
            <div className="flex items-center space-x-2">
              {secret.is_set && (
                <>
                  <button className="p-2 text-gray-400 hover:text-gray-600">
                    <EyeIcon className="w-5 h-5" />
                  </button>
                  <button className="p-2 text-gray-400 hover:text-gray-600">
                    <ClipboardIcon className="w-5 h-5" />
                  </button>
                </>
              )}
              <button className="p-2 text-gray-400 hover:text-gray-600">
                <PencilIcon className="w-5 h-5" />
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

const EyeIcon: React.FC<SVGComponentProps> = (props) => (
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" {...props}>
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
  </svg>
);

const ClipboardIcon: React.FC<SVGComponentProps> = (props) => (
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" {...props}>
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3" />
  </svg>
);

const PencilIcon: React.FC<SVGComponentProps> = (props) => (
  <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" {...props}>
    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z" />
  </svg>
); 