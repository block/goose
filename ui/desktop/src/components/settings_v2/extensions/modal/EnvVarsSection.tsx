import React from 'react';
import { Button } from '../../../ui/button';
import { Plus, X } from 'lucide-react';
import { Input } from '../../../ui/input';

interface EnvVarsSectionProps {
  envVars: { key: string; value: string }[];
  onAdd: () => void;
  onRemove: (index: number) => void;
  onChange: (index: number, field: 'key' | 'value', value: string) => void;
  submitAttempted: boolean;
  isValid: boolean;
}

export default function EnvVarsSection({
  envVars,
  onAdd,
  onRemove,
  onChange,
  submitAttempted,
  isValid,
}: EnvVarsSectionProps) {
  return (
    <div>
      <div className="relative mb-2">
        {' '}
        {/* Added relative positioning with minimal margin */}
        <label className="text-sm font-medium text-textStandard mb-2 block">
          Environment Variables
        </label>
        {submitAttempted && !isValid && (
          <div className="text-xs text-red-500 mt-1">
            {' '}
            {/* Removed absolute positioning */}
            Environment variables must consist of sets of variable names and values
          </div>
        )}
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
                className={`w-full bg-bgSubtle border-borderSubtle text-textStandard`}
              />
            </div>
            <div className="relative">
              <Input
                value={envVar.value}
                onChange={(e) => onChange(index, 'value', e.target.value)}
                placeholder="Value"
                className={`w-full bg-bgSubtle border-borderSubtle text-textStandard`}
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
          placeholder="Variable name"
          className="w-full border-borderStandard text-textStandard"
          disabled
        />
        <Input
          placeholder="Value"
          className="w-full border-borderStandard text-textStandard"
          disabled
        />
        <Button
          onClick={onAdd}
          variant="ghost"
          className="flex items-center justify-start gap-1 px-2 pr-4 text-s font-medium rounded-full dark:bg-slate-400 dark:text-gray-300 bg-gray-300 text-slate-400 dark:hover:bg-slate-300 hover:bg-gray-500 hover:text-white dark:hover:text-gray-900 transition-colors min-w-[60px] h-9"
        >
          <Plus className="h-3 w-3" /> Add
        </Button>
      </div>
    </div>
  );
}
