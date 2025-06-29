import React, { useState } from 'react';

export interface Parameter {
  name: string;
  promptMessage: string;
  defaultValue?: string;
  requirement: 'required' | 'optional' | 'interactive';
}

interface ParameterInputProps {
  parameter: Parameter;
  value: string;
  onChange: (name: string, value: Partial<Parameter>) => void;
}

const ParameterInput: React.FC<ParameterInputProps> = ({ parameter, value, onChange }) => {
  const [requirement, setRequirement] = useState(parameter.requirement || 'required');
  const [defaultValue, setDefaultValue] = useState(parameter.defaultValue || '');

  const handleRequirementChange = (newRequirement: 'required' | 'optional' | 'interactive') => {
    setRequirement(newRequirement);
    onChange(parameter.name, { requirement: newRequirement, defaultValue });
  };

  return (
    <div className="parameter-input my-4">
      <label className="block text-md text-textProminent mb-2 font-bold">
        {parameter.promptMessage || `Enter value for ${parameter.name}`}
      </label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(parameter.name, { defaultValue: e.target.value })}
        className="w-full p-3 border rounded-lg bg-bgApp text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
        placeholder={parameter.defaultValue || ''}
        disabled={requirement === 'interactive'}
      />
      <div className="mt-2">
        <label className="mr-2">Requirement:</label>
        <select
          className="p-2 border rounded"
          value={requirement}
          onChange={(e) => handleRequirementChange(e.target.value as Parameter['requirement'])}
        >
          <option value="required">Required</option>
          <option value="optional">Optional</option>
          <option value="interactive">Interactive</option>
        </select>
      </div>
      {requirement === 'optional' && (
        <div className="mt-2">
          <label className="block mb-1">Default Value:</label>
          <input
            type="text"
            value={defaultValue}
            onChange={(e) => setDefaultValue(e.target.value)}
            className="w-full p-2 border rounded"
            placeholder="Enter default value"
          />
        </div>
      )}
    </div>
  );
};

export default ParameterInput;
