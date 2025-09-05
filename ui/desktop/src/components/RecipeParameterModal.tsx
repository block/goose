import React, { useState } from 'react';
import { X } from 'lucide-react';
import { Button } from './ui/button';
import { Recipe, RecipeParameter } from '../recipe';

interface RecipeParameterModalProps {
  recipe: Recipe;
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (values: Record<string, string>) => void;
}

export const RecipeParameterModal: React.FC<RecipeParameterModalProps> = ({
  recipe,
  isOpen,
  onClose,
  onSubmit,
}) => {
  const [values, setValues] = useState<Record<string, string>>(() => {
    const initialValues: Record<string, string> = {};
    recipe.parameters?.forEach((param) => {
      initialValues[param.key] = param.default || '';
    });
    return initialValues;
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  if (!isOpen) return null;

  const handleInputChange = (key: string, value: string) => {
    setValues((prev) => ({ ...prev, [key]: value }));
    // Clear error when user starts typing
    if (errors[key]) {
      setErrors((prev) => {
        const newErrors = { ...prev };
        delete newErrors[key];
        return newErrors;
      });
    }
  };

  const validateAndSubmit = () => {
    const newErrors: Record<string, string> = {};

    // Validate required fields
    recipe.parameters?.forEach((param) => {
      if (param.requirement === 'required' && !values[param.key]?.trim()) {
        newErrors[param.key] = `${param.description} is required`;
      }
    });

    if (Object.keys(newErrors).length > 0) {
      setErrors(newErrors);
      return;
    }

    onSubmit(values);
    onClose();
  };

  const renderInput = (param: RecipeParameter) => {
    const value = values[param.key] || '';

    switch (param.input_type) {
      case 'select':
        return (
          <select
            value={value}
            onChange={(e) => handleInputChange(param.key, e.target.value)}
            className="w-full px-3 py-2 bg-background-default border border-borderStandard rounded-md text-textStandard focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">Select...</option>
            {param.options?.map((option: string) => (
              <option key={option} value={option}>
                {option}
              </option>
            ))}
          </select>
        );

      case 'boolean':
        return (
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id={param.key}
              checked={value === 'true'}
              onChange={(e) => handleInputChange(param.key, e.target.checked ? 'true' : 'false')}
              className="w-4 h-4 text-blue-600 bg-background-default border-borderStandard rounded focus:ring-blue-500"
            />
            <label htmlFor={param.key} className="text-sm text-textStandard">
              {value === 'true' ? 'Yes' : 'No'}
            </label>
          </div>
        );

      case 'number':
        return (
          <input
            type="number"
            value={value}
            onChange={(e) => handleInputChange(param.key, e.target.value)}
            placeholder={param.description}
            className="w-full px-3 py-2 bg-background-default border border-borderStandard rounded-md text-textStandard placeholder-textPlaceholder focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        );

      default: // string, file, date
        return (
          <input
            type="text"
            value={value}
            onChange={(e) => handleInputChange(param.key, e.target.value)}
            placeholder={param.description}
            className="w-full px-3 py-2 bg-background-default border border-borderStandard rounded-md text-textStandard placeholder-textPlaceholder focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        );
    }
  };

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-background-default border border-borderStandard rounded-lg shadow-xl max-w-md w-full mx-4 max-h-[80vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-borderSubtle">
          <div>
            <h2 className="text-lg font-semibold text-textStandard">{recipe.title}</h2>
            <p className="text-sm text-textSubtle mt-1">Configure recipe parameters</p>
          </div>
          <Button onClick={onClose} variant="ghost" size="sm" className="p-1">
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Parameters Form */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {recipe.parameters?.map((param) => (
            <div key={param.key} className="space-y-2">
              <label className="block">
                <span className="text-sm font-medium text-textStandard">
                  {param.description}
                  {param.requirement === 'required' && <span className="text-red-500 ml-1">*</span>}
                </span>
                {param.requirement === 'optional' && param.default && (
                  <span className="text-xs text-textSubtle ml-2">(default: {param.default})</span>
                )}
              </label>
              {renderInput(param)}
              {errors[param.key] && (
                <p className="text-xs text-red-500 mt-1">{errors[param.key]}</p>
              )}
            </div>
          ))}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-2 p-4 border-t border-borderSubtle">
          <Button onClick={onClose} variant="outline" size="sm">
            Cancel
          </Button>
          <Button
            onClick={validateAndSubmit}
            variant="default"
            size="sm"
            className="bg-blue-600 hover:bg-blue-700 text-white"
          >
            Run Recipe
          </Button>
        </div>
      </div>
    </div>
  );
};
