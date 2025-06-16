import { cn } from "@site/src/utils/cn";

export type SidebarFilterOption = {
  label: string;
  value: string;
  count?: number;
};

export type SidebarFilterGroup = {
  title: string;
  options: SidebarFilterOption[];
};

interface SidebarFilterProps {
  groups: SidebarFilterGroup[];
  selectedValues: Record<string, string[]>;
  onChange: (groupTitle: string, values: string[]) => void;
}

export function SidebarFilter({ groups, selectedValues, onChange }: SidebarFilterProps) {
  const toggleValue = (groupTitle: string, value: string) => {
    const currentValues = selectedValues[groupTitle] || [];
    const newValues = currentValues.includes(value)
      ? currentValues.filter((v) => v !== value)
      : [...currentValues, value];
    onChange(groupTitle, newValues);
  };

  return (
    <div className="w-64 pr-8">
      {groups.map((group) => (
        <div key={group.title} className="mb-8">
          <h3 className="text-lg font-medium mb-4 text-textProminent">
            {group.title}
          </h3>
          <div className="space-y-2">
            {group.options.map((option) => (
              <label
                key={option.value}
                className="flex items-center justify-between group cursor-pointer"
              >
                <div className="flex items-center">
                  <input
                    type="checkbox"
                    checked={(selectedValues[group.title] || []).includes(option.value)}
                    onChange={() => toggleValue(group.title, option.value)}
                    className="form-checkbox h-4 w-4 text-purple-600 transition duration-150 ease-in-out"
                  />
                  <span className="ml-2 text-sm text-textStandard group-hover:text-textProminent">
                    {option.label}
                  </span>
                </div>
                {option.count !== undefined && (
                  <span className="text-sm text-textSubtle">
                    {option.count}
                  </span>
                )}
              </label>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}