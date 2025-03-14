import { Input } from '../../../ui/input';
import React from 'react';

interface ExtensionConfigFieldsProps {
  type: 'stdio' | 'sse' | 'builtin';
  cmd: string;
  args: string;
  endpoint: string;
  onChange: (key: string, value: any) => void;
}

export default function ExtensionConfigFields({
  type,
  cmd,
  args,
  endpoint,
  onChange,
}: ExtensionConfigFieldsProps) {
  if (type === 'stdio') {
    return (
      <div className="space-y-4">
        <div>
          <label className="text-sm font-medium mb-2 block text-textStandard">Command</label>
          <Input
            value={cmd}
            onChange={(e) => onChange('cmd', e.target.value)}
            placeholder="e.g. npx -y @modelcontextprotocol/my-extension <filepath>"
            className="w-full bg-bgSubtle border-borderSubtle text-textStandard"
          />
        </div>
        <div>
          <label className="text-sm font-medium mb-2 block text-textStandard">Arguments</label>
          <Input
            value={args}
            onChange={(e) =>
              onChange(
                'args',
                e.target.value.split(' ').filter((arg) => arg.length > 0)
              )
            }
            placeholder="Enter arguments..."
            className="w-full bg-bgSubtle border-borderSubtle text-textStandard"
          />
        </div>
      </div>
    );
  } else {
    return (
      <div>
        <label className="text-sm font-medium mb-2 block text-textStandard">Endpoint</label>
        <Input
          value={endpoint}
          onChange={(e) => onChange('endpoint', e.target.value)}
          placeholder="Enter endpoint URL..."
          className="w-full bg-bgSubtle border-borderSubtle text-textStandard"
        />
      </div>
    );
  }
}
