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
  stderrLogPath,
}: ExtensionConfigFieldsProps & { stderrLogPath?: string }) {
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

        <div>
          <label className="text-sm font-medium mb-2 block text-textStandard">
            Stderr Log Path (optional)
          </label>
          <Input
            value={stderrLogPath || ''}
            onChange={(e) => onChange('stderrLogPath', e.target.value)}
            placeholder="e.g. /tmp/my-extension/stderr.log"
            className="w-full border-borderSubtle text-textStandard"
          />
          <p className="text-xs text-textMuted mt-1">
            Path to redirect stderr output for debugging. Leave empty to disable.
          </p>
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
