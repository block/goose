"use client";

import { cn } from "@/lib/utils";
import { Check } from "lucide-react";
import { ColorTheme } from "../types";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { SELECTOR_COLORS } from "../constants";

interface ColorSelectorProps {
  value?: ColorTheme;
  onChange?: (color: ColorTheme) => void;
  className?: string;
}

export const ColorSelector: React.FC<ColorSelectorProps> = ({
  value,
  onChange,
  className,
}) => {
  const selectedColor = SELECTOR_COLORS.find(color => color.value === value) || SELECTOR_COLORS[0];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="outline"
          className={cn("w-[120px] h-9 px-3 flex items-center gap-2", className)}
        >
          <div 
            className="h-5 w-5 rounded-full"
            style={{ backgroundColor: selectedColor.value }}
          />
          <span className="text-sm">{selectedColor.name}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="p-4">
        <div className="flex gap-4 flex-wrap min-w-[180px]">
          {SELECTOR_COLORS.map((color) => (
            <button
              key={color.value}
              className={cn(
                "h-8 w-8 rounded-full transition-transform hover:scale-110 focus:outline-none focus:ring-2 focus:ring-stone-950 focus:ring-offset-2 dark:focus:ring-stone-300",
                "flex items-center justify-center",
                {
                  "ring-2 ring-offset-2 ring-stone-950 dark:ring-stone-300": value === color.value,
                }
              )}
              style={{ backgroundColor: color.value }}
              onClick={() => onChange?.(color.value)}
              type="button"
              title={color.name}
            >
              {value === color.value && (
                <Check className="h-4 w-4 text-white" />
              )}
            </button>
          ))}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
