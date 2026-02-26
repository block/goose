import { Input } from '@/components/atoms/input';

interface ExtensionTimeoutFieldProps {
  timeout: number;
  onChange: (key: string, value: string | number) => void;
  submitAttempted: boolean;
}

export default function ExtensionTimeoutField({
  timeout,
  onChange,
  submitAttempted,
}: ExtensionTimeoutFieldProps) {
  const inputId = 'extension-timeout';

  const isTimeoutValid = () => {
    // Check if timeout is not undefined, null, or empty string
    if (timeout === undefined || timeout === null) {
      return false;
    }

    // Convert to number if it's a string
    const timeoutValue = typeof timeout === 'string' ? Number(timeout) : timeout;

    // Check if it's a valid number (not NaN) and is a positive number
    return !Number.isNaN(timeoutValue) && timeoutValue > 0;
  };

  return (
    <div className="flex flex-col gap-4 mb-6">
      {/* Row with Timeout and timeout input side by side */}
      <div className="flex flex-col relative">
        <div className="flex-1">
          <label htmlFor={inputId} className="text-sm font-medium mb-2 block text-text-default">
            Timeout
          </label>
        </div>

        <Input
          id={inputId}
          type="number"
          value={timeout}
          onChange={(e) => onChange('timeout', e.target.value)}
          className={`${!submitAttempted || isTimeoutValid() ? 'border-border-default' : 'border-red-500'} text-text-default focus:border-border-default`}
        />
        {submitAttempted && !isTimeoutValid() && (
          <div className="text-xs text-red-500 mt-1">Timeout is required</div>
        )}
      </div>
    </div>
  );
}
