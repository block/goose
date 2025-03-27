import React, { useEffect, useState } from 'react';
import { ExternalLink, Plus } from 'lucide-react';

import Modal from '../../../Modal';
import { Button } from '../../../ui/button';
import { QUICKSTART_GUIDE_URL } from '../../providers/modal/constants';
import { Input } from '../../../ui/input';
import { Select } from '../../../ui/Select';
import { useConfig } from '../../../ConfigContext';
import { changeModel as switchModel } from '../index';
import type { View } from '../../../../App';

const ModalButtons = ({ onSubmit, onCancel, isValid, validationErrors }) => (
  <div>
    <Button
      type="submit"
      variant="ghost"
      onClick={onSubmit}
      className="w-full h-[60px] rounded-none border-borderSubtle text-base hover:bg-bgSubtle text-textProminent font-regular"
    >
      Select model
    </Button>
    <Button
      type="button"
      variant="ghost"
      onClick={onCancel}
      className="w-full h-[60px] rounded-none border-t border-borderSubtle hover:text-textStandard text-textSubtle hover:bg-bgSubtle text-base font-regular"
    >
      Cancel
    </Button>
  </div>
);

type AddModelModalProps = {
  onClose: () => void;
  setView: (view: View) => void;
};
export const AddModelModal = ({ onClose, setView }: AddModelModalProps) => {
  const { getProviders, upsert } = useConfig();
  const [providerOptions, setProviderOptions] = useState([]);
  const [modelOptions, setModelOptions] = useState([]);
  const [provider, setProvider] = useState<string | null>(null);
  const [model, setModel] = useState<string>('');
  const [isCustomModel, setIsCustomModel] = useState(false);
  const [validationErrors, setValidationErrors] = useState({
    provider: '',
    model: '',
  });
  const [isValid, setIsValid] = useState(true);
  const [attemptedSubmit, setAttemptedSubmit] = useState(false);

  // Validate form data
  const validateForm = () => {
    const errors = {
      provider: '',
      model: '',
    };
    let formIsValid = true;

    if (!provider) {
      errors.provider = 'Please select a provider';
      formIsValid = false;
    }

    if (!model) {
      errors.model = 'Please select or enter a model';
      formIsValid = false;
    }

    setValidationErrors(errors);
    setIsValid(formIsValid);
    return formIsValid;
  };

  const changeModel = async () => {
    setAttemptedSubmit(true);
    const isFormValid = validateForm();

    if (isFormValid) {
      await switchModel({ model: model, provider: provider, writeToConfig: upsert });
      onClose();
    }
  };

  // Re-validate when inputs change and after attempted submission
  useEffect(() => {
    if (attemptedSubmit) {
      validateForm();
    }
  }, [provider, model, attemptedSubmit]);

  useEffect(() => {
    (async () => {
      try {
        const providersResponse = await getProviders(false);
        const activeProviders = providersResponse.filter((provider) => provider.is_configured);
        // Create provider options and add "Use other provider" option
        setProviderOptions([
          ...activeProviders.map(({ metadata, name }) => ({
            value: name,
            label: metadata.display_name,
          })),
          {
            value: 'configure_providers',
            label: 'Use other provider',
          },
        ]);

        // Format model options by provider
        const formattedModelOptions = [];
        activeProviders.forEach(({ metadata, name }) => {
          if (metadata.known_models && metadata.known_models.length > 0) {
            formattedModelOptions.push({
              label: metadata.display_name,
              options: metadata.known_models.map((modelName) => ({
                value: modelName,
                label: modelName,
                provider: name,
              })),
            });
          }
        });

        // Add the "Custom model" option to each provider group
        formattedModelOptions.forEach((group) => {
          group.options.push({
            value: 'custom',
            label: 'Use custom model',
            provider: group.options[0]?.provider,
          });
        });

        setModelOptions(formattedModelOptions);
      } catch (error) {
        console.error('Failed to load providers:', error);
      }
    })();
  }, [getProviders]);

  // Filter model options based on selected provider
  const filteredModelOptions = provider
    ? modelOptions.filter((group) => group.options[0]?.provider === provider)
    : [];

  // Handle model selection change
  const handleModelChange = (selectedOption) => {
    if (selectedOption?.value === 'custom') {
      setIsCustomModel(true);
      setModel('');
    } else {
      setIsCustomModel(false);
      setModel(selectedOption?.value || '');
    }
  };

  return (
    <div className="z-10">
      <Modal
        onClose={onClose}
        footer={
          <ModalButtons
            onSubmit={changeModel}
            onCancel={onClose}
            isValid={isValid}
            validationErrors={validationErrors}
          />
        }
      >
        <div className="flex flex-col items-center gap-8">
          <div className="flex flex-col items-center gap-3">
            <Plus size={24} className="text-textStandard" />
            <div className="text-textStandard font-medium">Switch models</div>
            <div className="text-textSubtle text-center">
              Configure your AI model providers by adding their API keys. Your keys are stored
              securely and encrypted locally.
            </div>
            <div>
              <a
                href={QUICKSTART_GUIDE_URL}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center justify-center text-textStandard font-medium text-sm"
              >
                <ExternalLink size={16} className="mr-1" />
                View quick start guide
              </a>
            </div>
          </div>

          <div className="w-full flex flex-col gap-4">
            <div>
              <Select
                options={providerOptions}
                value={providerOptions.find((option) => option.value === provider) || null}
                onChange={(option) => {
                  if (option?.value === 'configure_providers') {
                    // Navigate to ConfigureProviders view
                    setView('ConfigureProviders');
                    onClose(); // Close the current modal
                  } else {
                    setProvider(option?.value || null);
                    setModel('');
                    setIsCustomModel(false);
                  }
                }}
                placeholder="Provider"
                isClearable
              />
              {attemptedSubmit && validationErrors.provider && (
                <div className="text-red-500 text-sm mt-1">{validationErrors.provider}</div>
              )}
            </div>

            {provider && (
              <>
                {!isCustomModel ? (
                  <div>
                    <Select
                      options={filteredModelOptions}
                      onChange={handleModelChange}
                      value={model ? { value: model, label: model } : null}
                      placeholder="Select a model"
                    />
                    {attemptedSubmit && validationErrors.model && (
                      <div className="text-red-500 text-sm mt-1">{validationErrors.model}</div>
                    )}
                  </div>
                ) : (
                  <div className="flex flex-col gap-2">
                    <div className="flex justify-between">
                      <label className="text-sm text-textSubtle">Custom model name</label>
                      <button
                        onClick={() => setIsCustomModel(false)}
                        className="text-sm text-textSubtle"
                      >
                        Back to model list
                      </button>
                    </div>
                    <Input
                      className="border-2 px-4 py-5"
                      placeholder="Type model name here"
                      onChange={(event) => setModel(event.target.value)}
                      value={model}
                    />
                    {attemptedSubmit && validationErrors.model && (
                      <div className="text-red-500 text-sm mt-1">{validationErrors.model}</div>
                    )}
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </Modal>
    </div>
  );
};
