import { Search, X } from 'lucide-react';
import { useCallback, useState } from 'react';
import { cn } from '@/utils';

interface SearchInputProps {
  value?: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  debounceMs?: number;
  className?: string;
}

export function SearchInput({
  value: controlledValue,
  onChange,
  placeholder = 'Search...',
  debounceMs = 300,
  className,
}: SearchInputProps) {
  const [internalValue, setInternalValue] = useState('');
  const [debounceTimer, setDebounceTimer] = useState<ReturnType<typeof setTimeout> | null>(null);

  const currentValue = controlledValue ?? internalValue;

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newValue = e.target.value;

      if (controlledValue === undefined) {
        setInternalValue(newValue);
      }

      if (debounceTimer) clearTimeout(debounceTimer);

      if (debounceMs > 0) {
        const timer = setTimeout(() => onChange?.(newValue), debounceMs);
        setDebounceTimer(timer);
      } else {
        onChange?.(newValue);
      }
    },
    [controlledValue, onChange, debounceMs, debounceTimer]
  );

  const handleClear = useCallback(() => {
    if (controlledValue === undefined) {
      setInternalValue('');
    }
    onChange?.('');
  }, [controlledValue, onChange]);

  return (
    <div className={cn('relative', className)}>
      <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-text-muted pointer-events-none" />
      <input
        type="text"
        value={currentValue}
        onChange={handleChange}
        placeholder={placeholder}
        className="w-full pl-9 pr-8 py-2 text-sm bg-background-default border border-border-default rounded-md text-text-default placeholder:text-text-muted focus:outline-none focus:ring-1 focus:ring-border-accent transition-colors"
      />
      {currentValue && (
        <button
          onClick={handleClear}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded hover:bg-background-muted transition-colors"
        >
          <X className="h-3.5 w-3.5 text-text-muted" />
        </button>
      )}
    </div>
  );
}
