"use client";

import { Column } from "@tanstack/react-table";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useEffect, useState } from "react";
import type { LucideIcon } from "lucide-react";

interface ColumnFilterProps<TData, TValue> {
  column: Column<TData, TValue>;
  options: string[];
  icon?: LucideIcon;
  optionIcon?: LucideIcon;
  placeholder?: string;
  placeholderIcon?: React.ComponentType<{ className?: string }>;
  allText?: string;
  className?: string;
  width?: string;
}

export function ColumnFilter<TData, TValue>({
  column,
  options,
  icon: Icon,
  optionIcon: OptionIcon,
  placeholder = "Select...",
  placeholderIcon: PlaceholderIcon,
  allText = "all",
  className,
  width = "w-[180px]",
}: ColumnFilterProps<TData, TValue>) {
  const [selectedValue, setSelectedValue] = useState("_all_");

  useEffect(() => {
    const filterValue = column.getFilterValue() as string;
    if (filterValue) {
      setSelectedValue(filterValue);
    }
  }, [column]);

  return (
    <Select
      value={selectedValue}
      onValueChange={(value) => {
        setSelectedValue(value);
        column.setFilterValue(value === "_all_" ? undefined : value);
      }}
    >
      <SelectTrigger className={`${width} h-8 ${className || ""}`}>
        {selectedValue === "_all_" ? (
          <div className="flex items-center gap-2">
            {PlaceholderIcon && <PlaceholderIcon className="h-4 w-4" />}
            {placeholder}
          </div>
        ) : (
          <SelectValue />
        )}
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="_all_">
          <div className="flex items-center gap-2">
            {Icon && <Icon className="h-4 w-4" />}
            {allText}
          </div>
        </SelectItem>
        {options.map((option) => (
          <SelectItem key={option} value={option}>
            <div className="flex items-center gap-2">
              {OptionIcon && <OptionIcon className="h-4 w-4" />}
              {option}
            </div>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
