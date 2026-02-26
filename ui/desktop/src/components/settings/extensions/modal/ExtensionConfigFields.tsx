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
          <label className="text-sm font-medium mb-2 block text-text-primary">Command</label>
          <div className="relative">
            <Input
              data-testid="extension-command-input"
              value={full_cmd}
              onChange={(e) => onChange('cmd', e.target.value)}
              placeholder="e.g. npx -y @modelcontextprotocol/my-extension <filepath>"
              className={`w-full ${!submitAttempted || isValid ? 'border-border-primary' : 'border-red-500'} text-text-primary`}
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
        <label className="text-sm font-medium mb-2 block text-text-primary">Endpoint</label>
        <div className="relative">
          <Input
            data-testid="extension-endpoint-input"
            value={endpoint}
            onChange={(e) => onChange('endpoint', e.target.value)}
            placeholder="Enter endpoint URL..."
            className={`w-full ${!submitAttempted || isValid ? 'border-border-primary' : 'border-red-500'} text-text-primary`}
          />
          {submitAttempted && !isValid && (
            <div className="absolute text-xs text-red-500 mt-1">Endpoint URL is required</div>
          )}
        </div>
      </div>
    );
  }
}
