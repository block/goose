import React from 'react';
import { Modal, ModalContent, ModalHeader, ModalTitle } from '../ui/modal';
import { Button } from '../ui/button';
import { FullExtensionConfig } from '../../extensions';
import { getApiUrl, getSecretKey } from '../../config';
import { showToast } from '../ui/toast';

interface ConfigureExtensionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: () => void;
  extension: FullExtensionConfig | null;
}

export function ConfigureExtensionModal({
  isOpen,
  onClose,
  onSubmit,
  extension,
}: ConfigureExtensionModalProps) {
  const [envValues, setEnvValues] = React.useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = React.useState(false);

  // Reset form when dialog closes or extension changes
  React.useEffect(() => {
    if (!isOpen || !extension) {
      setEnvValues({});
    }
  }, [isOpen, extension]);

  const handleExtensionConfigSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!extension) return;

    setIsSubmitting(true);
    try {
      // First store all environment variables
      if (extension.env_keys?.length > 0) {
        for (const envKey of extension.env_keys) {
          const value = envValues[envKey];
          if (!value) continue;

          const storeResponse = await fetch(getApiUrl("/secrets/store"), {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              'X-Secret-Key': getSecretKey(),
            },
            body: JSON.stringify({
              key: envKey,
              value: value.trim(),
            }),
          });

          if (!storeResponse.ok) {
            throw new Error(`Failed to store environment variable: ${envKey}`);
          }
        }
      }

      // Then add the system configuration
      const systemConfig = {
        type: extension.type,
        ...(extension.type === 'stdio' && {
          cmd: extension.cmd,
          args: extension.args || [],
        }),
        ...(extension.type === 'sse' && {
          uri: extension.uri,
        }),
        ...(extension.type === 'builtin' && {
          name: extension.name,
        }),
        env_keys: extension.env_keys,
      };

      const response = await fetch(getApiUrl('/systems/add'), {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': getSecretKey(),
        },
        body: JSON.stringify(systemConfig)
      });

      if (!response.ok) {
        throw new Error('Failed to add system configuration');
      }

      showToast(`Succesfully configured the ${extension.name} extension`, 'success');
      onSubmit();
      onClose();
    } catch (error) {
      console.error('Error configuring extension:', error);
      showToast("Failed to configure extension", "error");
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!extension) return null;

  return (
    <Modal open={isOpen} onOpenChange={onClose}>
      <ModalContent>
        <ModalHeader>
          <ModalTitle>Configure {extension.name}</ModalTitle>
        </ModalHeader>
        <form onSubmit={handleExtensionConfigSubmit}>
          <div className="py-4 space-y-4">
            {extension.env_keys?.length > 0 ? (
              <>
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  Please provide the required environment variables for this extension:
                </p>
                <div className="space-y-4">
                  {extension.env_keys?.map((envVarName) => (
                    <div key={envVarName}>
                      <label
                        htmlFor={envVarName}
                        className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1"
                      >
                        {envVarName}
                      </label>
                      <input
                        type="text"
                        id={envVarName}
                        name={envVarName}
                        placeholder={envVarName}
                        value={envValues[envVarName] || ''}
                        onChange={(e) =>
                          setEnvValues((prev) => ({
                            ...prev,
                            [envVarName]: e.target.value,
                          }))
                        }
                        className="block w-full rounded-md border border-gray-300 dark:border-gray-600 
                                 bg-white dark:bg-gray-800 px-3 py-2 text-sm 
                                 placeholder-gray-400 dark:placeholder-gray-500
                                 focus:outline-none focus:ring-2 focus:ring-indigo-500 dark:focus:ring-indigo-400
                                 disabled:cursor-not-allowed disabled:opacity-50"
                        required
                      />
                    </div>
                  ))}
                </div>
              </>
            ) : (
              <p className="text-sm text-gray-500 dark:text-gray-400">
                This extension doesn't require any environment variables.
              </p>
            )}
          </div>
          <div className="flex justify-end gap-2 mt-4">
            <Button 
              variant="outline" 
              onClick={onClose} 
              type="button"
              disabled={isSubmitting}
            >
              Cancel
            </Button>
            <Button 
              type="submit"
              disabled={isSubmitting}
            >
              {isSubmitting ? 'Saving...' : 'Save Configuration'}
            </Button>
          </div>
        </form>
      </ModalContent>
    </Modal>
  );
}