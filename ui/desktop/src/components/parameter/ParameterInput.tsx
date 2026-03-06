import React from 'react';
import { AlertTriangle, Trash2, ChevronDown, ChevronRight } from 'lucide-react';
import { Parameter } from '../../recipe';
import { useLocalization } from '../../contexts/LocalizationContext';

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
  const { t } = useLocalization();
  const { key, description, requirement } = parameter;
  const defaultValue = parameter.default || '';

  const handleToggleExpanded = (e: React.MouseEvent) => {
    // Only toggle if we're not clicking on the delete button
    if (onToggleExpanded && !(e.target as HTMLElement).closest('button')) {
      onToggleExpanded(key);
    }
  };

  return (
    <div className="parameter-input my-4 border rounded-lg bg-background-secondary shadow-sm relative">
      {/* Collapsed header - always visible */}
      <div
        className={`flex items-center justify-between p-4 ${onToggleExpanded ? 'cursor-pointer hover:bg-background-primary/50' : ''} transition-colors`}
        onClick={handleToggleExpanded}
      >
        <div className="flex items-center gap-2 flex-1">
          {onToggleExpanded && (
            <button
              type="button"
              className="p-1 hover:bg-background-primary rounded transition-colors"
              onClick={(e) => {
                e.stopPropagation();
                onToggleExpanded(key);
              }}
            >
              {isExpanded ? (
                <ChevronDown className="w-4 h-4 text-text-secondary" />
              ) : (
                <ChevronRight className="w-4 h-4 text-text-secondary" />
              )}
            </button>
          )}

          <div className="flex items-center gap-2">
            <span className="text-md font-bold text-text-primary">
              <code className="bg-background-primary px-2 py-1 rounded-md">{parameter.key}</code>
            </span>
            {isUnused && (
              <div
                className="flex items-center gap-1"
                title={t('recipes.parameterEditor.unusedTitle')}
              >
                <AlertTriangle className="w-4 h-4 text-orange-500" />
                <span className="text-xs text-orange-500 font-normal">
                  {t('recipes.parameterEditor.unused')}
                </span>
              </div>
            )}
          </div>
        </div>

        {onDelete && (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onDelete(key);
            }}
            className="p-1 text-red-500 hover:text-red-700 hover:bg-red-50 rounded transition-colors"
            title={t('recipes.parameterEditor.deleteTitle', { key })}
          >
            <Trash2 className="w-4 h-4" />
          </button>
        )}
      </div>

      {/* Expandable content - only shown when expanded */}
      {isExpanded && (
        <div className="px-4 pb-4 border-t border-border-primary">
          <div className="pt-4">
            <div className="mb-4">
              <label className="block text-md text-text-primary mb-2 font-semibold">
                {t('recipes.parameterEditor.descriptionLabel')}
              </label>
              <input
                type="text"
                value={description || ''}
                onChange={(e) => onChange(key, { description: e.target.value })}
                className="w-full p-3 border rounded-lg bg-background-primary text-text-primary focus:outline-none focus:ring-2 focus:ring-border-secondary"
                placeholder={t('recipes.parameterEditor.descriptionPlaceholder')}
              />
              <p className="text-sm text-text-secondary mt-1">
                {t('recipes.parameterEditor.descriptionHint')}
              </p>
            </div>

            {/* Controls for requirement, input type, and default value */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div>
                <label className="block text-md text-text-primary mb-2 font-semibold">
                  {t('recipes.parameterEditor.inputTypeLabel')}
                </label>
                <select
                  className="w-full p-3 border rounded-lg bg-background-primary text-text-primary"
                  value={parameter.input_type || 'string'}
                  onChange={(e) =>
                    onChange(key, { input_type: e.target.value as Parameter['input_type'] })
                  }
                >
                  <option value="string">{t('recipes.parameterEditor.inputTypes.string')}</option>
                  <option value="select">{t('recipes.parameterEditor.inputTypes.select')}</option>
                  <option value="number">{t('recipes.parameterEditor.inputTypes.number')}</option>
                  <option value="boolean">{t('recipes.parameterEditor.inputTypes.boolean')}</option>
                </select>
              </div>

              <div>
                <label className="block text-md text-text-primary mb-2 font-semibold">
                  {t('recipes.parameterEditor.requirementLabel')}
                </label>
                <select
                  className="w-full p-3 border rounded-lg bg-background-primary text-text-primary"
                  value={requirement}
                  onChange={(e) =>
                    onChange(key, { requirement: e.target.value as Parameter['requirement'] })
                  }
                >
                  <option value="required">
                    {t('recipes.parameterEditor.requirements.required')}
                  </option>
                  <option value="optional">
                    {t('recipes.parameterEditor.requirements.optional')}
                  </option>
                </select>
              </div>

              {/* The default value input is only shown for optional parameters */}
              {requirement === 'optional' && (
                <div>
                  <label className="block text-md text-text-primary mb-2 font-semibold">
                    {t('recipes.parameterEditor.defaultValueLabel')}
                  </label>
                  <input
                    type="text"
                    value={defaultValue}
                    onChange={(e) => onChange(key, { default: e.target.value })}
                    className="w-full p-3 border rounded-lg bg-background-primary text-text-primary"
                    placeholder={t('recipes.parameterEditor.defaultValuePlaceholder')}
                  />
                </div>
              )}
            </div>

            {/* Options field for select input type */}
            {parameter.input_type === 'select' && (
              <div className="mt-4">
                <label className="block text-md text-text-primary mb-2 font-semibold">
                  {t('recipes.parameterEditor.optionsLabel')}
                </label>
                <textarea
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
                  className="w-full p-3 border rounded-lg bg-background-primary text-text-primary focus:outline-none focus:ring-2 focus:ring-border-secondary"
                  placeholder={t('recipes.parameterEditor.optionsPlaceholder')}
                  rows={4}
                />
                <p className="text-sm text-text-secondary mt-1">
                  {t('recipes.parameterEditor.optionsHint')}
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
