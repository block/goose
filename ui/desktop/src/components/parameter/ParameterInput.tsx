import React from 'react';
import { Parameter } from '../../recipe';

interface ParameterInputProps {
  parameter: Parameter;
  onChange: (name: string, updatedParameter: Partial<Parameter>) => void;
}

const ParameterInput: React.FC<ParameterInputProps> = ({ parameter, onChange }) => {
  // All values are derived directly from props, maintaining the controlled component pattern
  const { key, description, requirement } = parameter;
  const defaultValue = parameter.default || '';

  return (
    <div className="parameter-input my-4 p-4 border rounded-lg bg-bgSubtle shadow-sm">
      <h3 className="text-lg font-bold text-textProminent mb-4">
        Parameter: <code className="bg-bgApp px-2 py-1 rounded-md">{parameter.key}</code>
      </h3>

      <div className="mb-4">
        <label className="block text-md text-textStandard mb-2 font-semibold">description</label>
        <input
          type="text"
          value={description || ''}
          onChange={(e) => onChange(key, { description: e.target.value })}
          className="w-full p-3 border rounded-lg bg-bgApp text-textStandard focus:outline-none focus:ring-2 focus:ring-borderProminent"
          placeholder={`E.g., "Enter the name for the new component"`}
        />
        <p className="text-sm text-textSubtle mt-1">This is the message the end-user will see.</p>
      </div>

      {/* Controls for requirement and default value */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <label className="block text-md text-textStandard mb-2 font-semibold">Requirement</label>
          <select
            className="w-full p-3 border rounded-lg bg-bgApp text-textStandard"
            value={requirement}
            onChange={(e) =>
              onChange(key, { requirement: e.target.value as Parameter['requirement'] })
            }
          >
            <option value="required">Required</option>
            <option value="optional">Optional</option>
          </select>
        </div>

        {/* The default value input is only shown for optional parameters */}
        {requirement === 'optional' && (
          <div>
            <label className="block text-md text-textStandard mb-2 font-semibold">
              Default Value
            </label>
            <input
              type="text"
              value={defaultValue}
              onChange={(e) => onChange(key, { default: e.target.value })}
              className="w-full p-3 border rounded-lg bg-bgApp text-textStandard"
              placeholder="Enter default value"
            />
          </div>
        )}
      </div>
    </div>
  );
};

export default ParameterInput;
