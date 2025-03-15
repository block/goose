import React from 'react';
import { Button } from '../../../ui/button';
import { Plus, X } from 'lucide-react';
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
      <div className="flex justify-between items-start mb-2">
        <label className="text-sm font-medium text-textStandard">Environment Variables</label>
        <Button
          onClick={onAdd}
          variant="ghost"
          className="flex items-center gap-0.5 px-1 py-0.5 text-s font-medium rounded-full bg-gray-200 text-black hover:bg-gray-300 h-6"
        >
          <Plus className="h-0.5 w-0.5" /> Add
        </Button>
      </div>

      <div className="space-y-2">
        {envVars.map((envVar, index) => (
          <div key={index} className="flex gap-2 items-start">
            <Input
              value={envVar.key}
              onChange={(e) => onChange(index, 'key', e.target.value)}
              placeholder="Variable name"
              className="flex-1 bg-bgSubtle border-borderSubtle text-textStandard"
            />
            <Input
              value={envVar.value}
              onChange={(e) => onChange(index, 'value', e.target.value)}
              placeholder="Value"
              className="flex-1 bg-bgSubtle border-borderSubtle text-textStandard"
            />
            <Button
              onClick={() => onRemove(index)}
              variant="ghost"
              className="group p-2 h-auto text-iconSubtle hover:bg-transparent"
            >
              <X className="h-4 w-4 text-gray-400 group-hover:text-white group-hover:drop-shadow-sm transition-all" />
            </Button>
          </div>
        ))}
      </div>
    </div>
  );
}
