import React from 'react';
import { Button } from '../../../ui/button';
import { Plus, X } from 'lucide-react';
import { Input } from '../../../ui/input';
import { cn } from '../../../../utils';

interface HeadersSectionProps {
  headers: { key: string; value: string; isEdited?: boolean }[];
  onAdd: (key: string, value: string) => void;
  onRemove: (index: number) => void;
  onChange: (index: number, field: 'key' | 'value', value: string) => void;
  submitAttempted: boolean;
  onPendingInputChange: (
    hasPendingInput: boolean,
    pendingHeader: { key: string; value: string } | null
  ) => void;
}

export default function HeadersSection({
  headers,
  onAdd,
  onRemove,
  onChange,
  submitAttempted,
  onPendingInputChange,
}: HeadersSectionProps) {
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
    const pendingHeader =
      newKey.trim() && newValue.trim() ? { key: newKey, value: newValue } : null;
    onPendingInputChange(hasPendingInput, pendingHeader);
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
      setValidationError('Both header name and value must be entered');
      return;
    }

    if (keyHasSpaces) {
      setInvalidFields({
        key: true,
        value: false,
      });
      setValidationError('Header name cannot contain spaces');
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
    const value = headers[index][field].trim();
    return value === '';
  };

  return (
    <div>
      <div className="relative mb-2">
        <label className="text-sm font-medium text-textStandard mb-2 block">Request Headers</label>
        <p className="text-xs text-textSubtle mb-4">
          Add custom HTTP headers to include in requests to the MCP server. Click the "+" button to
          add after filling both fields.
        </p>
      </div>
      <div className="grid grid-cols-[1fr_1fr_auto] gap-2 items-center">
        {/* Existing headers */}
        {headers.map((header, index) => (
          <React.Fragment key={index}>
            <div className="relative">
              <Input
                value={header.key}
                onChange={(e) => onChange(index, 'key', e.target.value)}
                placeholder="Header name"
                className={cn(
                  'w-full text-textStandard border-borderSubtle hover:border-borderStandard',
                  isFieldInvalid(index, 'key') && 'border-red-500 focus:border-red-500'
                )}
              />
            </div>
            <div className="relative">
              <Input
                value={header.value}
                onChange={(e) => onChange(index, 'value', e.target.value)}
                placeholder="Value"
                className={cn(
                  'w-full text-textStandard border-borderSubtle hover:border-borderStandard',
                  isFieldInvalid(index, 'value') && 'border-red-500 focus:border-red-500'
                )}
              />
            </div>
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
          placeholder="Header name"
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
        <Button
          onClick={handleAdd}
          variant="ghost"
          className="flex items-center justify-start gap-1 px-2 pr-4 text-sm rounded-full text-textStandard bg-background-default border border-borderSubtle hover:border-borderStandard transition-colors min-w-[60px] h-9 [&>svg]:!size-4"
        >
          <Plus /> Add
        </Button>
      </div>
      {validationError && <div className="mt-2 text-red-500 text-sm">{validationError}</div>}
    </div>
  );
}
