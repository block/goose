import { Input } from '@/components/atoms/input';

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
  const inputId = type === 'stdio' ? 'extension-config-command' : 'extension-config-endpoint';

  if (type === 'stdio') {
    return (
      <div className="space-y-4">
        <div>
          <label
            htmlFor={inputId}
            className="text-sm font-medium mb-2 block text-text-default"
          >
            Command
          </label>
          <div className="relative">
            <Input
              id={inputId}
              value={full_cmd}
              onChange={(e) => onChange('cmd', e.target.value)}
              placeholder="e.g. npx -y @modelcontextprotocol/my-extension <filepath>"
              className={`w-full ${!submitAttempted || isValid ? 'border-border-default' : 'border-red-500'} text-text-default`}
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
        <label
          htmlFor={inputId}
          className="text-sm font-medium mb-2 block text-text-default"
        >
          Endpoint
        </label>
        <div className="relative">
          <Input
            id={inputId}
            value={endpoint}
            onChange={(e) => onChange('endpoint', e.target.value)}
            placeholder="Enter endpoint URL..."
            className={`w-full ${!submitAttempted || isValid ? 'border-border-default' : 'border-red-500'} text-text-default`}
          />
          {submitAttempted && !isValid && (
            <div className="absolute text-xs text-red-500 mt-1">Endpoint URL is required</div>
          )}
        </div>
      </div>
    );
  }
}
