"use client";

import { Table } from "@tanstack/react-table";
import { Input } from "@/components/ui/input";
import { Search } from "lucide-react";
import { useState } from "react";

interface EnhancedDataTableToolbarProps<TData> {
  table: Table<TData>;
  placeholder?: string;
  showSearchInput?: boolean;
  filterComponent?: React.ReactNode;
}

export function EnhancedDataTableToolbar<TData>({
  table,
  placeholder = "Search...",
  showSearchInput,
  filterComponent,
}: EnhancedDataTableToolbarProps<TData>) {
  const [searchValue, setSearchValue] = useState("");

  const handleSearch = (value: string) => {
    setSearchValue(value);
    table.setGlobalFilter(value);
  };

  // Don't render toolbar if there's no search input and no filter component
  if (!showSearchInput && !filterComponent) {
    return null;
  }

  return (
    <div className="flex items-center justify-between pb-4">
      <div className="flex items-center gap-4">
        {showSearchInput && (
          <div className="relative">
            <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 transform text-muted-foreground" />
            <Input
              placeholder={placeholder}
              value={searchValue}
              onChange={(event) => handleSearch(event.target.value)}
              className="w-[250px] pl-10"
            />
          </div>
        )}

        {filterComponent}
      </div>
    </div>
  );
}
