import { AlertTriangle, ChevronDown, ChevronRight, Trash2 } from 'lucide-react';
import type React from 'react';
import type { Parameter } from '@/recipe';

interface ParameterInputProps {
  parameter: Parameter;
  onChange: (name: string, updatedParameter: Partial<Parameter>) => void;
  onDelete?: (parameterKey: string) => void;
  isUnused?: boolean;
  isExpanded?: boolean;
  onToggleExpanded?: (parameterKey: string) => void;
}

const ParameterInput: React.FC<ParameterInputProps> = ({
  parameter,
  onChange,
  onDelete,
  isUnused = false,
  isExpanded = true,
  onToggleExpanded,
}) => {
  const { key, description, requirement } = parameter;
  const defaultValue = parameter.default || '';
  const idBase = `recipe-param-${key.replace(/[^a-zA-Z0-9_-]/g, '-')}`;

  return (
    <div className="parameter-input my-4 border rounded-lg bg-background-muted shadow-sm relative">
      {/* Collapsed header - always visible */}
      <div
        className={`flex items-center justify-between p-4 ${onToggleExpanded ? 'hover:bg-background-default/50' : ''} transition-colors`}
      >
        <button
          type="button"
          disabled={!onToggleExpanded}
          onClick={() => onToggleExpanded?.(key)}
          className={
            onToggleExpanded
              ? 'flex items-center gap-2 flex-1 text-left'
              : 'flex items-center gap-2 flex-1 text-left cursor-default'
          }
        >
          {onToggleExpanded &&
            (isExpanded ? (
              <ChevronDown className="w-4 h-4 text-text-muted" />
            ) : (
              <ChevronRight className="w-4 h-4 text-text-muted" />
            ))}

          <div className="flex items-center gap-2">
            <span className="text-md font-bold text-text-default">
              <code className="bg-background-default px-2 py-1 rounded-md">{parameter.key}</code>
            </span>
            {isUnused && (
              <span
                className="flex items-center gap-1"
                title="This parameter is not used in the instructions or prompt. It will be available for manual input but may not be needed."
              >
                <AlertTriangle className="w-4 h-4 text-orange-500" />
                <span className="text-xs text-orange-500 font-normal">Unused</span>
              </span>
            )}
          </div>
        </button>

        {onDelete && (
          <button
            type="button"
            onClick={() => onDelete(key)}
            className="p-1 text-red-500 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
            title={`Delete parameter: ${key}`}
          >
            <Trash2 className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* Expandable content - only shown when expanded */}
      {isExpanded && (
        <div className="px-4 pb-4 border-t border-border-default">
          <div className="pt-4">
            <div className="mb-4">
              <label
                htmlFor={`${idBase}-description`}
                className="block text-md text-text-default mb-2 font-semibold"
              >
                description
              </label>
              <input
                id={`${idBase}-description`}
                type="text"
                value={description || ''}
                onChange={(e) => onChange(key, { description: e.target.value })}
                className="w-full p-3 border rounded-lg bg-background-default text-text-default focus:outline-none focus:ring-2 focus:ring-border-strong"
                placeholder={`E.g., "Enter the name for the new component"`}
              />
              <p className="text-sm text-text-muted mt-1">
                This is the message the end-user will see.
              </p>
            </div>

            {/* Controls for requirement, input type, and default value */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label
                  htmlFor={`${idBase}-input-type`}
                  className="block text-md text-text-default mb-2 font-semibold"
                >
                  Input Type
                </label>
                <select
                  id={`${idBase}-input-type`}
                  className="w-full p-3 border rounded-lg bg-background-default text-text-default"
                  value={parameter.input_type || 'string'}
                  onChange={(e) =>
                    onChange(key, { input_type: e.target.value as Parameter['input_type'] })
                  }
                >
                  <option value="string">String</option>
                  <option value="select">Select</option>
                  <option value="number">Number</option>
                  <option value="boolean">Boolean</option>
                </select>
              </div>

              <div>
                <label
                  htmlFor={`${idBase}-requirement`}
                  className="block text-md text-text-default mb-2 font-semibold"
                >
                  Requirement
                </label>
                <select
                  id={`${idBase}-requirement`}
                  className="w-full p-3 border rounded-lg bg-background-default text-text-default"
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
                  <label
                    htmlFor={`${idBase}-default-value`}
                    className="block text-md text-text-default mb-2 font-semibold"
                  >
                    Default Value
                  </label>
                  <input
                    id={`${idBase}-default-value`}
                    type="text"
                    value={defaultValue}
                    onChange={(e) => onChange(key, { default: e.target.value })}
                    className="w-full p-3 border rounded-lg bg-background-default text-text-default"
                    placeholder="Enter default value"
                  />
                </div>
              )}
            </div>

            {/* Options field for select input type */}
            {parameter.input_type === 'select' && (
              <div className="mt-4">
                <label
                  htmlFor={`${idBase}-options`}
                  className="block text-md text-text-default mb-2 font-semibold"
                >
                  Options (one per line)
                </label>
                <textarea
                  id={`${idBase}-options`}
                  value={(parameter.options || []).join('\n')}
                  onChange={(e) => {
                    // Don't filter out empty lines - preserve them so user can type on new lines
                    const options = e.target.value.split('\n');
                    onChange(key, { options });
                  }}
                  onKeyDown={(e) => {
                    // Allow Enter key to work normally in textarea (prevent form submission or modal close)
                    if (e.key === 'Enter') {
                      e.stopPropagation();
                    }
                  }}
                  className="w-full p-3 border rounded-lg bg-background-default text-text-default focus:outline-none focus:ring-2 focus:ring-border-strong"
                  placeholder="Option 1&#10;Option 2&#10;Option 3"
                  rows={4}
                />
                <p className="text-sm text-text-muted mt-1">
                  Enter each option on a new line. These will be shown as dropdown choices.
                </p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};

export default ParameterInput;
