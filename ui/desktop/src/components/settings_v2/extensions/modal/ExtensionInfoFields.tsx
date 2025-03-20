import { Input } from '../../../ui/input';
import Select from 'react-select';
import React, { useState } from 'react';

interface ExtensionInfoFieldsProps {
  name: string;
  type: 'stdio' | 'sse' | 'builtin';
  onChange: (key: string, value: any) => void;
  submitAttempted: boolean;
}

export default function ExtensionInfoFields({
  name,
  type,
  onChange,
  submitAttempted,
}: ExtensionInfoFieldsProps) {
  const isNameValid = () => {
    return name.trim() !== '';
  };

  return (
    <div className="flex justify-between gap-4 mb-6">
      <div className="flex-1">
        <label className="text-sm font-medium mb-2 block text-textStandard">Extension Name</label>
        <div className="relative">
          <Input
            value={name}
            onChange={(e) => onChange('name', e.target.value)}
            placeholder="Enter extension name..."
            className={`${!submitAttempted || isNameValid() ? 'border-borderSubtle' : 'border-red-500'} text-textStandard focus:border-borderStandard`}
          />
          {submitAttempted && !isNameValid() && (
            <div className="absolute text-xs text-red-500 mt-1">Name is required</div>
          )}
        </div>
      </div>
      {/* Type Dropdown */}
      <div className="w-[200px]">
        <label className="text-sm font-medium mb-2 block text-textStandard">Type</label>
        <Select
          value={{ value: type, label: type.toUpperCase() }}
          onChange={(option: { value: string; label: string } | null) => {
            if (option) {
              onChange('type', option.value);
            }
          }}
          options={[
            { value: 'stdio', label: 'Standard IO (STDIO)' },
            { value: 'sse', label: 'Security Service Edge (SSE)' },
          ]}
          isSearchable={false}
        />
      </div>
    </div>
  );
}
