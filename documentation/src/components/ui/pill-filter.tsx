import { cn } from "@site/src/utils/cn";

export type PillFilterOption = {
  label: string;
  value: string;
};

interface PillFilterProps {
  options: PillFilterOption[];
  selectedValue: string;
  onChange: (value: string) => void;
}

export function PillFilter({ options, selectedValue, onChange }: PillFilterProps) {
  return (
    <div className="flex flex-wrap gap-2">
      {options.map((option) => (
        <button
          key={option.value}
          onClick={() => onChange(option.value)}
          className={cn(
            "px-4 py-2 rounded-full text-sm font-medium transition-colors",
            "border border-borderSubtle",
            selectedValue === option.value
              ? "dark:bg-white dark:text-black bg-black text-white border-borderProminent"
              : "bg-bgApp text-textStandard"
          )}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}