import React, { useState } from 'react';
import { Button } from '../../ui/button';
import { useEscapeKey } from '../../../hooks/useEscapeKey';

interface JsonSchemaEditorProps {
  isOpen: boolean;
  onClose: () => void;
  value: string;
  onChange: (value: string) => void;
  error?: string;
}

export default function JsonSchemaEditor({
  isOpen,
  onClose,
  value,
  onChange,
  error,
}: JsonSchemaEditorProps) {
  const [localValue, setLocalValue] = useState(value);
  const [localError, setLocalError] = useState('');

  useEscapeKey(isOpen, onClose);

  React.useEffect(() => {
    if (isOpen) {
      setLocalValue(value);
      setLocalError('');
    }
  }, [isOpen, value]);

  const handleSave = () => {
    if (localValue.trim()) {
      try {
        JSON.parse(localValue.trim());
        setLocalError('');
      } catch {
        setLocalError('Invalid JSON format');
        return;
      }
    }

    onChange(localValue);
    onClose();
  };

  const handleCancel = () => {
    setLocalValue(value);
    setLocalError('');
    onClose();
  };

  const insertExample = () => {
    const example = `{
  "type": "object",
  "properties": {
    "result": {
      "type": "string",
      "description": "The main result"
    },
    "status": {
      "type": "string",
      "enum": ["success", "error"],
      "description": "Operation status"
    },
    "data": {
      "type": "object",
      "properties": {
        "items": {
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      }
    }
  },
  "required": ["result", "status"]
}`;
    setLocalValue(example);
  };

  if (!isOpen) return null;

  return (
    <div
      className="fixed inset-0 z-[400] flex items-center justify-center bg-black/50"
      onClick={(e) => {
        // Close modal when clicking backdrop
        if (e.target === e.currentTarget) {
          handleCancel();
        }
      }}
    >
      <div className="bg-background-default border border-border-subtle rounded-lg p-6 w-[800px] max-w-[90vw] max-h-[90vh] overflow-hidden flex flex-col">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-medium text-text-standard">JSON Schema Editor</h3>
          <button
            type="button"
            onClick={handleCancel}
            className="text-text-muted hover:text-text-standard text-2xl leading-none"
          >
            ×
          </button>
        </div>

        <div className="flex-1 flex flex-col min-h-0">
          <div className="mb-4">
            <div className="flex items-center justify-between mb-2">
              <label className="block text-sm font-medium text-text-standard">
                Response JSON Schema
              </label>
              <Button
                type="button"
                onClick={insertExample}
                variant="ghost"
                size="sm"
                className="text-xs"
              >
                Insert Example
              </Button>
            </div>
            <p className="text-xs text-text-muted mb-3">
              Define the expected structure of the AI's response using JSON Schema format
            </p>
          </div>

          <div className="flex-1 min-h-0">
            <textarea
              value={localValue}
              onChange={(e) => {
                setLocalValue(e.target.value);
                setLocalError('');
              }}
              className={`w-full h-full min-h-[400px] p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none font-mono text-sm ${
                localError || error ? 'border-red-500' : 'border-border-subtle'
              }`}
              placeholder={`{
  "type": "object",
  "properties": {
    "result": {
      "type": "string",
      "description": "The main result"
    }
  },
  "required": ["result"]
}`}
            />
            {(localError || error) && (
              <p className="text-red-500 text-sm mt-2">{localError || error}</p>
            )}
          </div>
        </div>

        <div className="flex justify-end space-x-3 mt-6 pt-4 border-t border-border-subtle">
          <Button type="button" onClick={handleCancel} variant="ghost">
            Cancel
          </Button>
          <Button type="button" onClick={handleSave} variant="default">
            Save Schema
          </Button>
        </div>
      </div>
    </div>
  );
}
