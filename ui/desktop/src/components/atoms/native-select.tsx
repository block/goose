import { cn } from '../../utils';

interface NativeSelectOption {
  value: string;
  label: string;
  disabled?: boolean;
}

interface NativeSelectProps {
  label?: string;
  placeholder?: string;
  options: NativeSelectOption[];
  value?: string;
  onValueChange?: (value: string) => void;
  className?: string;
}

export function NativeSelect({
  label,
  placeholder,
  options,
  value,
  onValueChange,
  className,
}: NativeSelectProps) {
  const selectId = `native-select-${label?.toLowerCase().replace(/\s+/g, '-') || 'field'}`;
  return (
    <div className={cn('space-y-1.5', className)}>
      {label && (
        <label htmlFor={selectId} className="text-sm font-medium text-text-default">
          {label}
        </label>
      )}
      <select
        id={selectId}
        className="w-full rounded-md border border-border-default bg-background-default text-text-default px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-border-accent"
        defaultValue={value || ''}
        onChange={onValueChange ? (e) => onValueChange(e.target.value) : undefined}
      >
        {placeholder && (
          <option value="" disabled>
            {placeholder}
          </option>
        )}
        {options.map((opt) => (
          <option key={opt.value} value={opt.value} disabled={opt.disabled}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}
