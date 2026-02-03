"use client";

import { cn } from "@/lib/utils";

interface FormattedTextProps {
  text: string;
  className?: string;
}

export function FormattedText({ text, className }: FormattedTextProps) {
  // Split text by newlines and render each line
  const lines = text.split("\n");

  // Filter out empty lines at the beginning and end
  const trimmedLines = lines.reduce((acc: string[], line) => {
    // Skip empty lines at the beginning
    if (acc.length === 0 && line.trim() === "") return acc;

    // Add the line
    acc.push(line);

    return acc;
  }, []);

  // Remove trailing empty lines
  while (
    trimmedLines.length > 0 &&
    trimmedLines[trimmedLines.length - 1].trim() === ""
  ) {
    trimmedLines.pop();
  }

  return (
    <div className={cn("space-y-1", className)}>
      {trimmedLines.map((line, index) => (
        <div key={index}>
          {line.trim() === "" ? (
            // Render empty lines as spacers
            <div className="h-2" />
          ) : (
            // Render text lines
            <span>{line}</span>
          )}
        </div>
      ))}
    </div>
  );
}
