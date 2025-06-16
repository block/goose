import { Input } from '../../../ui/input';
import { Select } from '../../../ui/Select';

interface ExtensionInfoFieldsProps {
  name: string;
  type: 'stdio' | 'sse' | 'streamable_http' | 'builtin';
  description: string;
  onChange: (key: string, value: string) => void;
  submitAttempted: boolean;
}

export default function ExtensionInfoFields({
  name,
  type,
  description,
  onChange,
  submitAttempted,
}: ExtensionInfoFieldsProps) {
  const isNameValid = () => {
    return name.trim() !== '';
  };

  return (
    <div className="flex flex-col gap-4 mb-6">
      {/* Top row with Name and Type side by side */}
      <div className="flex justify-between gap-4">
        <div className="flex-1">
          <label className="text-sm font-medium mb-2 block text-textStandard">Extension Name</label>
          <div className="relative">
            <Input
              value={name}
              onChange={(e) => onChange('name', e.target.value)}
              placeholder="Enter extension name..."
              className={`${!submitAttempted || isNameValid() ? 'border-borderSubtle' : 'border-red-500'} text-textStandard focus:border-borderStandard`}
            />
            {submitAttempted && !isNameValid() && (
              <div className="absolute text-xs text-red-500 mt-1">Name is required</div>
            )}
          </div>
        </div>

        {/* Type Dropdown */}
        <div className="w-[200px]">
          <label className="text-sm font-medium mb-2 block text-textStandard">Type</label>
          <Select
            value={{
              value: type,
              label:
                type === 'stdio'
                  ? 'STDIO'
                  : type === 'sse'
                    ? 'SSE'
                    : type === 'streamable_http'
                      ? 'HTTP'
                      : type.toUpperCase(),
            }}
            onChange={(newValue: unknown) => {
              const option = newValue as { value: string; label: string } | null;
              if (option) {
                onChange('type', option.value);
              }
            }}
            options={[
              { value: 'stdio', label: 'Standard IO (STDIO)' },
              { value: 'sse', label: 'Server-Sent Events (SSE)' },
              { value: 'streamable_http', label: 'Streamable HTTP' },
            ]}
            isSearchable={false}
          />
        </div>
      </div>

      {/* Bottom row with Description spanning full width */}
      <div className="w-full">
        <label className="text-sm font-medium mb-2 block text-textStandard">Description</label>
        <div className="relative">
          <Input
            value={description}
            onChange={(e) => onChange('description', e.target.value)}
            placeholder="Optional description..."
            className={`text-textStandard focus:border-borderStandard`}
          />
        </div>
      </div>
    </div>
  );
}
