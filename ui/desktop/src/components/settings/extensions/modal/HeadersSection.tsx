import React from 'react';
import { Button } from '../../../ui/button';
import { Plus, X, Eye, EyeOff } from 'lucide-react';
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
  // Track which header values are visible (all masked by default)
  const [visibleIndexes, setVisibleIndexes] = React.useState<Set<number>>(new Set());
  const [showNewRowValue, setShowNewRowValue] = React.useState(false);

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
    const normalizedNewKey = newKey.trim().toLowerCase();
    const isDuplicate = headers.some(
      h => h.key.trim().toLowerCase() === normalizedNewKey
    );

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

    if (isDuplicate) {
      setInvalidFields({
        key: true,
        value: false,
      });
      setValidationError('A header with this name already exists');
      return;
    }

    setValidationError(null);
    setInvalidFields({ key: false, value: false });
    onAdd(newKey, newValue);
    setNewKey('');
    setNewValue('');
    setShowNewRowValue(false);
  };

  const handleRemove = (index: number) => {
    onRemove(index);
    // Clean up visibility state and reindex remaining entries
    setVisibleIndexes((prev) => {
      const newSet = new Set(prev);
      newSet.delete(index);
      const reindexed = new Set<number>();
      for (const visibleIdx of newSet) {
        if (visibleIdx > index) {
          reindexed.add(visibleIdx - 1);
        } else {
          reindexed.add(visibleIdx);
        }
      }
      return reindexed;
    });
  };

  const toggleVisibility = (index: number) => {
    setVisibleIndexes((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(index)) {
        newSet.delete(index);
      } else {
        newSet.add(index);
      }
      return newSet;
    });
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
        <label className="text-sm font-medium text-text-default mb-2 block">Request Headers</label>
        <p className="text-xs text-text-muted mb-4">
          Add custom HTTP headers to include in requests to the MCP server. Click the "+" button to
          add after filling both fields.
        </p>
      </div>
      <div className="grid grid-cols-[1fr_1fr_auto_auto] gap-2 items-center">
        {/* Existing headers */}
        {headers.map((header, index) => (
          <React.Fragment key={index}>
            <div className="relative">
              <Input
                value={header.key}
                onChange={(e) => onChange(index, 'key', e.target.value)}
                placeholder="Header name"
                className={cn(
                  'w-full text-text-default border-border-default hover:border-border-default',
                  isFieldInvalid(index, 'key') && 'border-red-500 focus:border-red-500'
                )}
              />
            </div>
            <Input
              type={visibleIndexes.has(index) ? 'text' : 'password'}
              value={header.value}
              onChange={(e) => onChange(index, 'value', e.target.value)}
              placeholder="Value"
              className={cn(
                'w-full text-text-default border-border-default hover:border-border-default',
                isFieldInvalid(index, 'value') && 'border-red-500 focus:border-red-500'
              )}
            />
            <Button
              onClick={() => toggleVisibility(index)}
              variant="ghost"
              className="group p-2 h-auto text-iconSubtle hover:bg-transparent"
              title={visibleIndexes.has(index) ? 'Hide value' : 'Show value'}
              aria-label={visibleIndexes.has(index) ? 'Hide header value' : 'Show header value'}
            >
              {visibleIndexes.has(index) ? (
                <Eye className="h-3 w-3 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
              ) : (
                <EyeOff className="h-3 w-3 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
              )}
            </Button>
            <Button
              onClick={() => handleRemove(index)}
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
            'w-full text-text-default border-border-default hover:border-border-default',
            invalidFields.key && 'border-red-500 focus:border-red-500'
          )}
        />
        <Input
          type={showNewRowValue ? 'text' : 'password'}
          value={newValue}
          onChange={(e) => {
            setNewValue(e.target.value);
            clearValidation();
          }}
          placeholder="Value"
          className={cn(
            'w-full text-text-default border-border-default hover:border-border-default',
            invalidFields.value && 'border-red-500 focus:border-red-500'
          )}
        />
        <Button
          onClick={() => setShowNewRowValue(!showNewRowValue)}
          variant="ghost"
          className="group p-2 h-auto text-iconSubtle hover:bg-transparent"
          title={showNewRowValue ? 'Hide value' : 'Show value'}
          aria-label={showNewRowValue ? 'Hide header value' : 'Show header value'}
        >
          {showNewRowValue ? (
            <Eye className="h-3 w-3 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
          ) : (
            <EyeOff className="h-3 w-3 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
          )}
        </Button>
        <Button
          onClick={handleAdd}
          variant="ghost"
          className="flex items-center justify-start gap-1 px-2 pr-4 text-sm rounded-full text-text-default bg-background-default border border-border-default hover:border-border-default transition-colors min-w-[60px] h-9 [&>svg]:!size-4"
        >
          <Plus /> Add
        </Button>
      </div>
      {validationError && <div className="mt-2 text-red-500 text-sm">{validationError}</div>}
    </div>
  );
}
