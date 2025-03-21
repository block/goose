import { Package, Puzzle } from "lucide-react";
import { Checkbox } from "./ui/checkbox";
import { Label } from "./ui/label";
import type { FilterCategory } from "../types/filters";

interface FilterSidebarProps {
  functions: FilterCategory[];
  selected: {
    functions: string[];
    extensions: string[];
  };
  onFilterChange: (type: string, value: string) => void;
  extensions?: string[];
}

export function FilterSidebar({ 
  functions, 
  selected, 
  onFilterChange,
  extensions = [
    "developer",
    "computer controller",
    "memory",
    "jetbrains",
    "git",
    "figma",
    "google drive",
    "tavily web search"
  ]
}: FilterSidebarProps) {
  return (
    <div className="w-64 pr-6 space-y-6">
      {/* Function Filter */}
      <div className="space-y-3">
        <h3 className="flex items-center gap-2 text-sm font-medium text-textProminent">
          <Package className="h-4 w-4" />
          Function
        </h3>
        <div className="space-y-2">
          {functions.map((func) => (
            <div key={func.id} className="flex items-center space-x-2">
              <Checkbox 
                id={`function-${func.id}`}
                checked={selected.functions.includes(func.id)}
                onCheckedChange={() => onFilterChange('functions', func.id)}
                className="border-borderSubtle"
              />
              <Label htmlFor={`function-${func.id}`} className="text-sm text-textStandard">
                {func.name}
              </Label>
            </div>
          ))}
        </div>
      </div>

      {/* Extensions Filter */}
      <div className="space-y-3">
        <h3 className="flex items-center gap-2 text-sm font-medium text-textProminent">
          <Puzzle className="h-4 w-4" />
          Extensions
        </h3>
        <div className="space-y-2">
          {extensions.map((extension) => (
            <div key={extension} className="flex items-center space-x-2">
              <Checkbox 
                id={`extension-${extension}`}
                checked={selected.extensions.includes(extension)}
                onCheckedChange={() => onFilterChange('extensions', extension)}
                className="border-borderSubtle"
              />
              <Label htmlFor={`extension-${extension}`} className="text-sm text-textStandard capitalize">
                {extension}
              </Label>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}