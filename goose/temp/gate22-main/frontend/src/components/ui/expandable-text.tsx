"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { ChevronDown, ChevronUp } from "lucide-react";
import { cn } from "@/lib/utils";

interface ExpandableTextProps {
  text: string;
  className?: string;
  expandButtonClassName?: string;
  maxLines?: number;
}

export function ExpandableText({
  text,
  className,
  expandButtonClassName,
  maxLines = 2,
}: ExpandableTextProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Check if text is long enough to need expansion
  // Consider both character count and newlines
  const lines = text.split("\n");
  const needsExpansion = text.length > 150 || lines.length > maxLines;

  if (!needsExpansion) {
    return (
      <span className={cn(className, "whitespace-pre-wrap break-words")}>
        {text}
      </span>
    );
  }

  return (
    <div>
      <span
        className={cn(
          className,
          "whitespace-pre-wrap break-words",
          !isExpanded && maxLines === 2 && "line-clamp-2",
          !isExpanded && maxLines === 3 && "line-clamp-3",
          !isExpanded && maxLines === 1 && "line-clamp-1",
        )}
      >
        {text}
      </span>
      <div className="mt-1">
        <Button
          variant="outline"
          size="sm"
          onClick={() => setIsExpanded(!isExpanded)}
          className={cn("h-6 px-2 text-xs font-normal", expandButtonClassName)}
        >
          {isExpanded ? (
            <>
              Show less <ChevronUp className="ml-1 h-3 w-3" />
            </>
          ) : (
            <>
              Show more <ChevronDown className="ml-1 h-3 w-3" />
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
