import React, { useState } from 'react';
import { Button } from '../../../ui/button';
import Modal from '../../../Modal';
import { Input } from '../../../ui/input';
import Select from 'react-select';
import { createDarkSelectStyles, darkSelectTheme } from '../../../ui/select-styles';
import { ExtensionFormData } from '../ExtensionsSection';
import EnvVarsSection from './EnvVarsSection';
import ExtensionConfigFields from './ExtensionConfigFields';
import { PlusIcon, Edit, Trash2 } from 'lucide-react';

interface ExtensionModalProps {
  title: string;
  initialData: ExtensionFormData;
  onClose: () => void;
  onSubmit: (formData: ExtensionFormData) => void;
  onDelete?: (name: string) => void;
  submitLabel: string;
  modalType: 'add' | 'edit';
}

export default function ExtensionModal({
  title,
  initialData,
  onClose,
  onSubmit,
  onDelete,
  submitLabel,
  modalType,
}: ExtensionModalProps) {
  const [formData, setFormData] = useState<ExtensionFormData>(initialData);

  const handleAddEnvVar = () => {
    setFormData({
      ...formData,
      envVars: [...formData.envVars, { key: '', value: '' }],
    });
  };

  const handleRemoveEnvVar = (index: number) => {
    const newEnvVars = [...formData.envVars];
    newEnvVars.splice(index, 1);
    setFormData({
      ...formData,
      envVars: newEnvVars,
    });
  };

  const handleEnvVarChange = (index: number, field: 'key' | 'value', value: string) => {
    const newEnvVars = [...formData.envVars];
    newEnvVars[index][field] = value;
    setFormData({
      ...formData,
      envVars: newEnvVars,
    });
  };

  // Function to determine which icon to display with proper styling
  const getModalIcon = () => {
    return modalType === 'add' ? (
      <PlusIcon className="text-iconStandard" size={24} />
    ) : (
      <Edit className="text-iconStandard" size={24} />
    );
  };

  return (
    <Modal>
      {/* Title and Icon */}
      <div className="flex flex-col mb-6">
        <div>{getModalIcon()}</div>
        <div className="mt-2">
          <h2 className="text-2xl font-regular text-textStandard">{title}</h2>
        </div>
      </div>

      {/* Form Fields */}
      <div className="flex justify-between gap-4 mb-6">
        <div className="flex-1">
          <label className="text-sm font-medium mb-2 block text-textStandard">Extension Name</label>
          <Input
            value={formData.name}
            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
            placeholder="Enter extension name..."
            className="bg-bgSubtle border-borderSubtle text-textStandard focus:border-borderStandard"
          />
        </div>
        <div className="w-[200px]">
          <label className="text-sm font-medium mb-2 block text-textStandard">Type</label>
          <Select
            value={{ value: formData.type, label: formData.type.toUpperCase() }}
            onChange={(option: { value: string; label: string } | null) =>
              setFormData({
                ...formData,
                type: (option?.value as 'stdio' | 'sse' | 'builtin') || 'stdio',
              })
            }
            options={[
              { value: 'stdio', label: 'STDIO' },
              { value: 'sse', label: 'SSE' },
            ]}
            styles={createDarkSelectStyles('200px')}
            theme={darkSelectTheme}
            isSearchable={false}
          />
        </div>
      </div>

      {/* Config Fields */}
      <div className="mb-6">
        <ExtensionConfigFields
          type={formData.type}
          full_cmd={formData.cmd || ''}
          endpoint={formData.endpoint || ''}
          onChange={(key, value) => setFormData({ ...formData, [key]: value })}
        />
      </div>

      {/* Environment Variables */}
      <div className="mb-6">
        <EnvVarsSection
          envVars={formData.envVars}
          onAdd={handleAddEnvVar}
          onRemove={handleRemoveEnvVar}
          onChange={handleEnvVarChange}
        />
      </div>

      {/* Action Buttons */}
      <div className="absolute bottom-0 left-0 right-0 flex flex-col border-t border-borderSubtle">
        {modalType === 'edit' && onDelete && (
          <Button
            onClick={() => onDelete(formData.name)}
            className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-bgSubtle text-red-500 font-medium text-md"
          >
            <Trash2 className="h-4 w-4 mr-2" /> Delete Extension
          </Button>
        )}
        <Button
          onClick={() => onSubmit(formData)}
          className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-bgSubtle text-textProminent font-medium text-md"
        >
          {submitLabel}
        </Button>
        <Button
          onClick={onClose}
          variant="ghost"
          className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-md font-regular"
        >
          Cancel
        </Button>
      </div>
    </Modal>
  );
}
