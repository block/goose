import React from 'react';
import { Card } from '../../ui/card';
import { Button } from '../../ui/button';
import { Input } from '../../ui/input';
import { FullExtensionConfig } from '../../../extensions';
import { getApiUrl, getSecretKey } from '../../../config';
import { addExtension } from '../../../extensions';
import { toast } from 'react-toastify';
import { ToastError, ToastSuccess } from '../models/toasts';

interface ConfigureExtensionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: () => void;
  extension: FullExtensionConfig | null;
}

export function ConfigureBuiltInExtensionModal({
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

          const storeResponse = await fetch(getApiUrl('/configs/store'), {
            method: 'POST',
            headers: {
              'Content-Type': 'application/json',
              'X-Secret-Key': getSecretKey(),
            },
            body: JSON.stringify({
              key: envKey,
              value: value.trim(),
              isSecret: true,
            }),
          });

          if (!storeResponse.ok) {
            throw new Error(`Failed to store environment variable: ${envKey}`);
          }
        }
      }

      const response = await addExtension(extension);

      if (!response.ok) {
        throw new Error('Failed to add system configuration');
      }

      ToastSuccess({
        title: extension.name,
        msg: `Successfully configured extension`,
      });
      onSubmit();
      onClose();
    } catch (error) {
      console.error('Error configuring extension:', error);
      ToastError({
        title: extension.name,
        msg: `Failed to configure the extension`,
        traceback: error.message,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!extension || !isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm">
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[440px] bg-white dark:bg-gray-800 rounded-xl shadow-xl overflow-hidden p-[16px] pt-[24px] pb-0">
        <div className="px-8 pb-0 space-y-8">
          {/* Header */}
          <div className="flex">
            <h2 className="text-2xl font-regular dark:text-white text-gray-900">
              Configure {extension.name}
            </h2>
          </div>

          {/* Form */}
          <form onSubmit={handleExtensionConfigSubmit}>
            <div className="mt-[24px]">
              {extension.env_keys?.length > 0 ? (
                <>
                  <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
                    Please provide the required environment variables for this extension:
                  </p>
                  <div className="space-y-4">
                    {extension.env_keys?.map((envVarName) => (
                      <div key={envVarName}>
                        <label
                          htmlFor={envVarName}
                          className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
                        >
                          {envVarName}
                        </label>
                        <Input
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
                          className="w-full h-14 px-4 font-regular rounded-lg border shadow-none border-gray-300 bg-white text-lg placeholder:text-gray-400 font-regular text-gray-900 dark:bg-gray-800 dark:border-gray-600 dark:text-white dark:placeholder:text-gray-500"
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

            {/* Actions */}
            <div className="mt-[8px] ml-[-24px] mr-[-24px] pt-[16px]">
              <Button
                type="submit"
                variant="ghost"
                disabled={isSubmitting}
                className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-indigo-500 hover:bg-indigo-50 dark:hover:bg-indigo-900/20 dark:border-gray-600 text-lg font-regular"
              >
                {isSubmitting ? 'Saving...' : 'Save Configuration'}
              </Button>
              <Button
                type="button"
                variant="ghost"
                onClick={onClose}
                disabled={isSubmitting}
                className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-gray-400 hover:bg-gray-50 dark:border-gray-600 text-lg font-regular"
              >
                Cancel
              </Button>
            </div>
          </form>
        </div>
      </Card>
    </div>
  );
}
