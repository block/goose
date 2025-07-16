import React from 'react';
import { Button } from '../../../ui/button';
import { Plus, X, Edit } from 'lucide-react';
import { Input } from '../../../ui/input';
import { cn } from '../../../../utils';

interface EnvVarsSectionProps {
  envVars: { key: string; value: string; isEdited?: boolean }[];
  onAdd: (key: string, value: string) => void;
  onRemove: (index: number) => void;
  onChange: (index: number, field: 'key' | 'value', value: string) => void;
  submitAttempted: boolean;
  onPendingInputChange?: (hasPendingInput: boolean) => void;
}

export default function EnvVarsSection({
  envVars,
  onAdd,
  onRemove,
  onChange,
  submitAttempted,
  onPendingInputChange,
}: EnvVarsSectionProps) {
  const [newKey, setNewKey] = React.useState('');
  const [newValue, setNewValue] = React.useState('');
  const [validationError, setValidationError] = React.useState<string | null>(null);
  const [invalidFields, setInvalidFields] = React.useState<{ key: boolean; value: boolean }>({
    key: false,
    value: false,
  });

  // Notify parent when pending input changes
  React.useEffect(() => {
    const hasPendingInput = newKey.trim() !== '' || newValue.trim() !== '';
    onPendingInputChange?.(hasPendingInput);
  }, [newKey, newValue, onPendingInputChange]);

  const handleAdd = () => {
    const keyEmpty = !newKey.trim();
    const valueEmpty = !newValue.trim();
    const keyHasSpaces = newKey.includes(' ');

    if (keyEmpty || valueEmpty) {
      setInvalidFields({
        key: keyEmpty,
        value: valueEmpty,
      });
      setValidationError('Both variable name and value must be entered');
      return;
    }

    if (keyHasSpaces) {
      setInvalidFields({
        key: true,
        value: false,
      });
      setValidationError('Variable name cannot contain spaces');
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

  const handleEdit = (index: number) => {
    // Mark this env var as edited
    onChange(index, 'value', envVars[index].value === '••••••••' ? '' : envVars[index].value);

    // Mark as edited in the parent component
    const updatedEnvVar = {
      ...envVars[index],
      isEdited: true,
    };

    // Update the envVars array with the edited flag
    const newEnvVars = [...envVars];
    newEnvVars[index] = updatedEnvVar;
  };

  return (
    <div>
      <div className="relative mb-2">
        <label className="text-sm font-medium text-textStandard mb-2 block">
          Environment Variables
        </label>
        <p className="text-xs text-textSubtle mb-4">
          Add key-value pairs for environment variables. Click the "+" button to add after filling
          both fields. For existing secret values, click the edit button to modify.
        </p>
      </div>
      <div className="grid grid-cols-[1fr_1fr_auto_auto] gap-2 items-center">
        {/* Existing environment variables */}
        {envVars.map((envVar, index) => (
          <React.Fragment key={index}>
            <div className="relative">
              <Input
                value={envVar.key}
                onChange={(e) => onChange(index, 'key', e.target.value)}
                placeholder="Variable name"
                className={cn(
                  'w-full text-textStandard border-borderSubtle hover:border-borderStandard',
                  isFieldInvalid(index, 'key') && 'border-red-500 focus:border-red-500'
                )}
              />
            </div>
            <div className="relative">
              <Input
                value={envVar.value}
                readOnly={envVar.value === '••••••••' && !envVar.isEdited}
                onChange={(e) => {
                  // If this is the first edit of a placeholder value, clear it
                  const newValue =
                    envVar.value === '••••••••' && !envVar.isEdited ? '' : e.target.value;
                  onChange(index, 'value', newValue);
                }}
                placeholder="Value"
                className={cn(
                  'w-full border-borderSubtle',
                  envVar.value === '••••••••' && !envVar.isEdited
                    ? 'text-textSubtle opacity-60 cursor-not-allowed hover:border-borderSubtle'
                    : 'text-textStandard hover:border-borderStandard',
                  isFieldInvalid(index, 'value') && 'border-red-500 focus:border-red-500'
                )}
              />
            </div>
            {envVar.value === '••••••••' && !envVar.isEdited && (
              <Button
                onClick={() => handleEdit(index)}
                variant="ghost"
                className="group p-2 h-auto text-iconSubtle hover:bg-transparent"
              >
                <Edit className="h-3 w-3 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
              </Button>
            )}
            {(envVar.value !== '••••••••' || envVar.isEdited) && (
              <div className="w-8 h-8"></div> /* Empty div to maintain grid spacing */
            )}
            <Button
              onClick={() => onRemove(index)}
              variant="ghost"
              className="group p-2 h-auto text-iconSubtle hover:bg-transparent"
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
            'w-full text-textStandard border-borderSubtle hover:border-borderStandard',
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
            'w-full text-textStandard border-borderSubtle hover:border-borderStandard',
            invalidFields.value && 'border-red-500 focus:border-red-500'
          )}
        />
        <div className="col-span-2">
          <Button
            onClick={handleAdd}
            variant="ghost"
            className="flex items-center justify-start gap-1 px-2 pr-4 text-sm rounded-full text-textStandard bg-background-default border border-borderSubtle hover:border-borderStandard transition-colors min-w-[60px] h-9 [&>svg]:!size-4"
          >
            <Plus /> Add
          </Button>
        </div>
      </div>
      {validationError && <div className="mt-2 text-red-500 text-sm">{validationError}</div>}
    </div>
  );
}
