"use client";

import * as React from "react";
import { Check, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";

interface MultiSelectProps {
  value: string[];
  onValueChange: (value: string[]) => void;
  children: React.ReactNode;
}

const MultiSelectContext = React.createContext<{
  value: string[];
  onValueChange: (value: string[]) => void;
} | null>(null);

export function MultiSelect({ value, onValueChange, children }: MultiSelectProps) {
  return (
    <MultiSelectContext.Provider value={{ value, onValueChange }}>
      <Popover>{children}</Popover>
    </MultiSelectContext.Provider>
  );
}

export function MultiSelectTrigger({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <PopoverTrigger asChild>
      <button
        type="button"
        className={cn(
          "flex min-h-10 w-full items-center justify-between rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:outline-none disabled:cursor-not-allowed disabled:opacity-50",
          className,
        )}
      >
        {children}
      </button>
    </PopoverTrigger>
  );
}

export function MultiSelectValue({ placeholder }: { placeholder?: string }) {
  const context = React.useContext(MultiSelectContext);
  if (!context) throw new Error("MultiSelectValue must be used within MultiSelect");

  const { value } = context;

  return (
    <div className="flex flex-1 flex-wrap gap-1">
      {value.length > 0 ? (
        value.map((item) => (
          <Badge key={item} variant="secondary" className="mr-1">
            {item}
            <button
              type="button"
              className="ml-1 rounded-full ring-offset-background outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
              onClick={(e) => {
                e.stopPropagation();
                context.onValueChange(value.filter((v) => v !== item));
              }}
            >
              <X className="h-3 w-3" />
            </button>
          </Badge>
        ))
      ) : (
        <span className="text-muted-foreground">{placeholder}</span>
      )}
    </div>
  );
}

export function MultiSelectContent({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  const [search, setSearch] = React.useState("");

  return (
    <PopoverContent className={cn("w-full p-0", className)} align="start">
      <Command>
        <CommandInput placeholder="Search..." value={search} onValueChange={setSearch} />
        <CommandEmpty>No items found.</CommandEmpty>
        <CommandList>
          <CommandGroup>
            {React.Children.map(children, (child) => {
              if (React.isValidElement(child)) {
                const childProps = child.props as { value?: string };
                if (childProps.value) {
                  const itemValue = childProps.value.toLowerCase();
                  const searchValue = search.toLowerCase();
                  if (searchValue && !itemValue.includes(searchValue)) {
                    return null;
                  }
                }
                return child;
              }
              return child;
            })}
          </CommandGroup>
        </CommandList>
      </Command>
    </PopoverContent>
  );
}

export function MultiSelectItem({ value, children }: { value: string; children: React.ReactNode }) {
  const context = React.useContext(MultiSelectContext);
  if (!context) throw new Error("MultiSelectItem must be used within MultiSelect");

  const isSelected = context.value.includes(value);

  return (
    <CommandItem
      onSelect={() => {
        if (isSelected) {
          context.onValueChange(context.value.filter((v) => v !== value));
        } else {
          context.onValueChange([...context.value, value]);
        }
      }}
    >
      <Check className={cn("mr-2 h-4 w-4", isSelected ? "opacity-100" : "opacity-0")} />
      {children}
    </CommandItem>
  );
}
