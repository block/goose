// ExtensionModal.tsx
import React, { useState } from 'react';
import { Button } from '../../../ui/button';
import Modal from '../../../Modal';
import { Input } from '../../../ui/input';
import Select from 'react-select';
import { createDarkSelectStyles, darkSelectTheme } from '../../../ui/select-styles';
import { ExtensionFormData } from '../ExtensionsSection';
import EnvVarsSection from './EnvVarsSection';
import ExtensionConfigFields from './ExtensionConfigFields';

interface ExtensionModalProps {
  title: string;
  initialData: ExtensionFormData;
  onClose: () => void;
  onSubmit: (formData: ExtensionFormData) => void;
  submitLabel: string;
}

export default function ExtensionModal({
  title,
  initialData,
  onClose,
  onSubmit,
  submitLabel,
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

  return (
    <Modal>
      <div className="space-y-6">
        <h2 className="text-xl font-medium">{title}</h2>

        <div className="flex justify-between gap-4">
          <div className="flex-1">
            <label className="text-sm font-medium mb-2 block">Extension Name</label>
            <Input
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              placeholder="Enter extension name..."
            />
          </div>
          <div className="w-[200px]">
            <label className="text-sm font-medium mb-2 block">Type</label>
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

        <ExtensionConfigFields
          type={formData.type}
          cmd={formData.cmd || ''}
          args={formData.args?.join(' ') || ''}
          endpoint={formData.endpoint || ''}
          onChange={(key, value) => setFormData({ ...formData, [key]: value })}
        />

        <EnvVarsSection
          envVars={formData.envVars}
          onAdd={handleAddEnvVar}
          onRemove={handleRemoveEnvVar}
          onChange={handleEnvVarChange}
        />

        <div className="flex justify-end gap-3 pt-4">
          <Button onClick={onClose} variant="ghost" className="hover:bg-subtle">
            Cancel
          </Button>
          <Button onClick={() => onSubmit(formData)} className="bg-[#393838] hover:bg-subtle">
            {submitLabel}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

// ExtensionConfigFields.tsx

// EnvVarsSection.tsx
