import React, { useEffect, useState } from 'react';
import { ExternalLink, Plus } from 'lucide-react';

import Modal from '../../Modal';
import { Button } from '../../ui/button';
import { QUICKSTART_GUIDE_URL } from '../providers/modal/constants';
import { Input } from '../../ui/input';
import { Select } from '../../ui/Select';
import { useConfig } from '../../ConfigContext';
import { ToastFailureGeneral, ToastSuccessModelSwitch } from '../../settings/models/toasts';
import { initializeSystem } from '../../../../src/utils/providerUtils';

const ModalButtons = ({ onSubmit, onCancel }) => (
  <div>
    <Button
      type="submit"
      variant="ghost"
      onClick={onSubmit}
      className="w-full h-[60px] rounded-none border-borderSubtle text-base hover:bg-bgSubtle text-textProminent font-regular"
    >
      Add model
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

type AddModelModalProps = { onClose: () => void };
export const AddModelModal = ({ onClose }: AddModelModalProps) => {
  const { getProviders, upsert } = useConfig();
  const [providerOptions, setProviderOptions] = useState([]);
  const [provider, setProvider] = useState<string | null>(null);
  const [modelName, setModelName] = useState<string>('');

  const changeModel = async () => {
    try {
      await upsert('GOOSE_PROVIDER', provider, false);
      await upsert('GOOSE_MODEL', modelName, false);
      await initializeSystem(provider, modelName);
      ToastSuccessModelSwitch({ provider, name: modelName });
      onClose();
    } catch (e) {
      ToastFailureGeneral(e.message);
    }
  };

  useEffect(() => {
    (async () => {
      try {
        const providersResponse = await getProviders(false);
        const activeProviders = providersResponse.filter((provider) => provider.is_configured);
        setProviderOptions(
          activeProviders.map(({ metadata, name }) => ({
            value: name,
            label: metadata.display_name,
          }))
        );
      } catch (error) {
        console.error('Failed to load providers:', error);
      }
    })();
  }, [getProviders]);

  return (
    <div className="z-10">
      <Modal onClose={onClose} footer={<ModalButtons onSubmit={changeModel} onCancel={onClose} />}>
        <div className="flex flex-col items-center gap-8">
          <div className="flex flex-col items-center gap-3">
            <Plus size={24} className="text-textStandard" />
            <div className="text-textStandard font-medium">Add model</div>
            <div className="text-textSubtle text-center">
              Configure your AI model providers by adding their API keys. your Keys are stored
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
            <Select
              options={providerOptions}
              value={providerOptions.find((option) => option.value === provider) || null}
              onChange={(option) => {
                setProvider(option?.value || null);
                setModelName('');
              }}
              placeholder="Provider"
              isClearable
            />
            <Input
              className="border-2 px-4 py-5"
              placeholder="GPT"
              onChange={(event) => setModelName(event.target.value)}
              value={modelName}
            />
          </div>
        </div>
      </Modal>
    </div>
  );
};
