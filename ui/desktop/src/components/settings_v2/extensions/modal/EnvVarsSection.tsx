import React from 'react';
import { Button } from '../../../ui/button';
import { X } from 'lucide-react';
import { Input } from '../../../ui/input';

interface EnvVarsSectionProps {
  envVars: { key: string; value: string }[];
  onAdd: () => void;
  onRemove: (index: number) => void;
  onChange: (index: number, field: 'key' | 'value', value: string) => void;
}

export default function EnvVarsSection({
  envVars,
  onAdd,
  onRemove,
  onChange,
}: EnvVarsSectionProps) {
  return (
    <div>
      <div className="flex justify-between items-center mb-2">
        <label className="text-sm font-medium">Environment Variables</label>
        <Button onClick={onAdd} variant="ghost" className="text-sm hover:bg-subtle">
          Add Variable
        </Button>
      </div>

      <div className="space-y-2">
        {envVars.map((envVar, index) => (
          <div key={index} className="flex gap-2 items-start">
            <Input
              value={envVar.key}
              onChange={(e) => onChange(index, 'key', e.target.value)}
              placeholder="Key"
              className="flex-1"
            />
            <Input
              value={envVar.value}
              onChange={(e) => onChange(index, 'value', e.target.value)}
              placeholder="Value"
              className="flex-1"
            />
            <Button
              onClick={() => onRemove(index)}
              variant="ghost"
              className="p-2 h-auto hover:bg-subtle"
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        ))}
      </div>
    </div>
  );
}
