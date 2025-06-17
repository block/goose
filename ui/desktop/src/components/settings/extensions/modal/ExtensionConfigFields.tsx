import { Input } from '../../../ui/input';

interface ExtensionConfigFieldsProps {
  type: 'stdio' | 'sse' | 'streamable_http' | 'builtin';
  full_cmd: string;
  endpoint: string;
  onChange: (key: string, value: string) => void;
  submitAttempted?: boolean;
  isValid?: boolean;
}

export default function ExtensionConfigFields({
  type,
  full_cmd,
  endpoint,
  onChange,
  submitAttempted = false,
  isValid,
}: ExtensionConfigFieldsProps) {
  if (type === 'stdio') {
    return (
      <div className="space-y-4">
        <div>
          <label className="text-sm font-medium mb-2 block text-textStandard">Command</label>
          <div className="relative">
            <Input
              value={full_cmd}
              onChange={(e) => onChange('cmd', e.target.value)}
              placeholder="e.g. npx -y @modelcontextprotocol/my-extension <filepath>"
              className={`w-full ${!submitAttempted || isValid ? 'border-borderSubtle' : 'border-red-500'} text-textStandard`}
            />
            {submitAttempted && !isValid && (
              <div className="absolute text-xs text-red-500 mt-1">Command is required</div>
            )}
          </div>
        </div>
      </div>
    );
  } else {
    return (
      <div>
        <label className="text-sm font-medium mb-2 block text-textStandard">Endpoint</label>
        <div className="relative">
          <Input
            value={endpoint}
            onChange={(e) => onChange('endpoint', e.target.value)}
            placeholder="Enter endpoint URL..."
            className={`w-full ${!submitAttempted || isValid ? 'border-borderSubtle' : 'border-red-500'} text-textStandard`}
          />
          {submitAttempted && !isValid && (
            <div className="absolute text-xs text-red-500 mt-1">Endpoint URL is required</div>
          )}
        </div>
      </div>
    );
  }
}
