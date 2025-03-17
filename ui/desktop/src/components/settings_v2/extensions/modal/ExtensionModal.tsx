import React, { useState } from 'react';
import { Button } from '../../../ui/button';
import Modal from '../../../Modal';
import { Input } from '../../../ui/input';
import Select from 'react-select';
import { createDarkSelectStyles, darkSelectTheme } from '../../../ui/select-styles';
import { ExtensionFormData } from '../utils';
import EnvVarsSection from './EnvVarsSection';
import ExtensionConfigFields from './ExtensionConfigFields';
import { PlusIcon, Edit, Trash2, AlertTriangle } from 'lucide-react';

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
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [submitAttempted, setSubmitAttempted] = useState(false);

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
    if (showDeleteConfirmation) {
      return <AlertTriangle className="text-red-500" size={24} />;
    }
    return modalType === 'add' ? (
      <PlusIcon className="text-iconStandard" size={24} />
    ) : (
      <Edit className="text-iconStandard" size={24} />
    );
  };

  const isNameValid = () => {
    return formData.name.trim() !== '';
  };

  const isConfigValid = () => {
    return (
      (formData.type === 'stdio' && formData.cmd && formData.cmd.trim() !== '') ||
      (formData.type === 'sse' && formData.endpoint && formData.endpoint.trim() !== '')
    );
  };

  const isEnvVarsValid = () => {
    return formData.envVars.every(
      ({ key, value }) => (key === '' && value === '') || (key !== '' && value !== '')
    );
  };

  // Form validation
  const isFormValid = () => {
    return isNameValid() && isConfigValid() && isEnvVarsValid();
  };

  // Handle submit with validation
  const handleSubmit = () => {
    setSubmitAttempted(true);

    if (isFormValid()) {
      onSubmit(formData);
    }
  };

  // Create footer buttons based on current state
  const footerContent = showDeleteConfirmation ? (
    // Delete confirmation footer
    <>
      <div className="w-full px-6 py-4 bg-red-900/20 border-t border-red-500/30">
        <p className="text-red-400 text-sm mb-2">
          Are you sure you want to delete "{formData.name}"? This action cannot be undone.
        </p>
      </div>
      <Button
        onClick={() => onDelete && onDelete(formData.name)}
        className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-red-900/20 text-red-500 font-medium text-md"
      >
        <Trash2 className="h-4 w-4 mr-2" /> Confirm Delete
      </Button>
      <Button
        onClick={() => setShowDeleteConfirmation(false)}
        variant="ghost"
        className="w-full h-[60px] rounded-none hover:bg-bgSubtle text-textSubtle hover:text-textStandard text-md font-regular"
      >
        Cancel
      </Button>
    </>
  ) : (
    // Normal footer
    <>
      {modalType === 'edit' && onDelete && (
        <Button
          onClick={() => setShowDeleteConfirmation(true)}
          className="w-full h-[60px] rounded-none border-b border-borderSubtle bg-transparent hover:bg-bgSubtle text-red-500 font-medium text-md"
        >
          <Trash2 className="h-4 w-4 mr-2" /> Delete Extension
        </Button>
      )}
      <Button
        onClick={handleSubmit}
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
    </>
  );

  // Update title based on current state
  const modalTitle = showDeleteConfirmation ? `Delete Extension "${formData.name}"` : title;

  return (
    <Modal footer={footerContent} onClose={onClose}>
      {/* Title and Icon */}
      <div className="flex flex-col mb-6">
        <div>{getModalIcon()}</div>
        <div className="mt-2">
          <h2 className="text-2xl font-regular text-textStandard">{modalTitle}</h2>
        </div>
      </div>

      {showDeleteConfirmation ? (
        <div className="mb-6">
          <p className="text-textStandard">
            This will permanently remove this extension and all of its settings.
          </p>
        </div>
      ) : (
        <>
          {/* Form Fields */}
          {/* Name */}
          <div className="flex justify-between gap-4 mb-6">
            <div className="flex-1">
              <label className="text-sm font-medium mb-2 block text-textStandard">
                Extension Name
              </label>
              <div className="relative">
                <Input
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  placeholder="Enter extension name..."
                  className={`${!submitAttempted || formData.name.trim() !== '' ? 'border-borderSubtle' : 'border-red-500'} text-textStandard focus:border-borderStandard`}
                />
                {submitAttempted && !isNameValid() && (
                  <div className="absolute text-xs text-red-500 mt-1">Name is required</div>
                )}
              </div>
            </div>
            {/*Type Dropdown */}
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
                  { value: 'stdio', label: 'Standard IO (STDIO)' },
                  { value: 'sse', label: 'Security Service Edge (SSE)' },
                ]}
                styles={createDarkSelectStyles('200px')}
                theme={darkSelectTheme}
                isSearchable={false}
              />
            </div>
          </div>

          {/* Divider */}
          <hr className="border-t border-borderSubtle mb-6" />

          {/* Config Fields */}
          <div className="mb-6">
            <ExtensionConfigFields
              type={formData.type}
              full_cmd={formData.cmd || ''}
              endpoint={formData.endpoint || ''}
              onChange={(key, value) => setFormData({ ...formData, [key]: value })}
              submitAttempted={submitAttempted}
              isValid={isConfigValid()}
            />
          </div>

          {/* Divider */}
          <hr className="border-t border-borderSubtle mb-6" />

          {/* Environment Variables */}
          <div className="mb-6">
            <EnvVarsSection
              envVars={formData.envVars}
              onAdd={handleAddEnvVar}
              onRemove={handleRemoveEnvVar}
              onChange={Object.assign(handleEnvVarChange, { setSubmitAttempted })}
              submitAttempted={submitAttempted}
              isValid={isEnvVarsValid()}
            />
          </div>
        </>
      )}
    </Modal>
  );
}
