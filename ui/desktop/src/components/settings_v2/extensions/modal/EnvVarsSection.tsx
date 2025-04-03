import React from 'react';
import { Button } from '../../../ui/button';
import { Plus, X } from 'lucide-react';
import { Input } from '../../../ui/input';
import { cn } from '../../../../utils';

interface EnvVarsSectionProps {
  envVars: { key: string; value: string }[];
  onAdd: (key: string, value: string) => void;
  onRemove: (index: number) => void;
  onChange: (index: number, field: 'key' | 'value', value: string) => void;
  submitAttempted: boolean;
}

export default function EnvVarsSection({
  envVars,
  onAdd,
  onRemove,
  onChange,
  submitAttempted,
}: EnvVarsSectionProps) {
  const [newKey, setNewKey] = React.useState('');
  const [newValue, setNewValue] = React.useState('');
  const [validationError, setValidationError] = React.useState<string | null>(null);
  const [invalidFields, setInvalidFields] = React.useState<{ key: boolean; value: boolean }>({
    key: false,
    value: false,
  });

  const handleAdd = () => {
    const keyEmpty = !newKey.trim();
    const valueEmpty = !newValue.trim();

    if (keyEmpty || valueEmpty) {
      setInvalidFields({
        key: keyEmpty,
        value: valueEmpty,
      });
      setValidationError('Both variable name and value must be entered');
      return;
    }

    setValidationError(null);
    setInvalidFields({ key: false, value: false });
    onAdd(newKey, newValue);
    setNewKey('');
    setNewValue('');
  };

  const clearValidation = () => {
    setValidationError(null);
    setInvalidFields({ key: false, value: false });
  };

  const isFieldInvalid = (index: number, field: 'key' | 'value') => {
    if (!submitAttempted) return false;
    const value = envVars[index][field].trim();
    return value === '';
  };

  return (
    <div>
      <div className="relative mb-2">
        <label className="text-sm font-medium text-textStandard mb-2 block">
          Environment Variables
        </label>
        <p className="text-xs text-textSubtle mb-2">
          Add key-value pairs for environment variables. Click the "+" button to add after filling
          both fields.
        </p>
      </div>
      <div className="grid grid-cols-[1fr_1fr_auto] gap-2 items-center">
        {/* Existing environment variables */}
        {envVars.map((envVar, index) => (
          <React.Fragment key={index}>
            <div className="relative">
              <Input
                value={envVar.key}
                onChange={(e) => onChange(index, 'key', e.target.value)}
                placeholder="Variable name"
                className={cn(
                  'w-full border-borderSubtle text-textStandard',
                  isFieldInvalid(index, 'key') && 'border-red-500 focus:border-red-500'
                )}
              />
            </div>
            <div className="relative">
              <Input
                value={envVar.value}
                onChange={(e) => onChange(index, 'value', e.target.value)}
                placeholder="Value"
                className={cn(
                  'w-full border-borderSubtle text-textStandard',
                  isFieldInvalid(index, 'value') && 'border-red-500 focus:border-red-500'
                )}
              />
            </div>
            <Button
              onClick={() => onRemove(index)}
              variant="ghost"
              className="group p-2 h-auto text-iconSubtle hover:bg-transparent min-w-[60px] flex justify-start"
            >
              <X className="h-3 w-3 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
            </Button>
          </React.Fragment>
        ))}

        {/* Empty row with Add button */}
        <Input
          value={newKey}
          onChange={(e) => {
            setNewKey(e.target.value);
            clearValidation();
          }}
          placeholder="Variable name"
          className={cn(
            'w-full border-borderStandard text-textStandard',
            invalidFields.key && 'border-red-500 focus:border-red-500'
          )}
        />
        <Input
          value={newValue}
          onChange={(e) => {
            setNewValue(e.target.value);
            clearValidation();
          }}
          placeholder="Value"
          className={cn(
            'w-full border-borderStandard text-textStandard',
            invalidFields.value && 'border-red-500 focus:border-red-500'
          )}
        />
        <Button
          onClick={handleAdd}
          variant="ghost"
          className="flex items-center justify-start gap-1 px-2 pr-4 text-s font-medium rounded-full dark:bg-slate-400 dark:text-gray-300 bg-gray-300 dark:bg-slate text-slate-400 dark:hover:bg-slate-300 hover:bg-gray-500 hover:text-white dark:hover:text-gray-900 transition-colors min-w-[60px] h-9"
        >
          <Plus className="h-3 w-3" /> Add
        </Button>
      </div>
      {validationError && <div className="mt-2 text-red-500 text-sm">{validationError}</div>}
    </div>
  );
}
