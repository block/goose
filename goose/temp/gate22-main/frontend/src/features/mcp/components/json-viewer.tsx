"use client";

import { useState } from "react";
import ReactJson from "@microlink/react-json-view";
import { Copy, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useTheme } from "next-themes";

interface JsonViewerProps {
  data: unknown;
  title?: string;
  className?: string;
  maxHeight?: string;
  collapsed?: boolean | number;
}

export function JsonViewer({
  data,
  title = "Schema",
  className,
  maxHeight = "400px",
  collapsed = 2, // Collapse at depth 2 by default
}: JsonViewerProps) {
  const [copied, setCopied] = useState(false);
  const { theme } = useTheme();

  const handleCopy = () => {
    navigator.clipboard.writeText(JSON.stringify(data, null, 2));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Theme configuration for react-json-view
  const jsonTheme = theme === "dark" ? "monokai" : "rjv-default";

  return (
    <div className={cn("overflow-hidden rounded-lg border bg-muted/10", className)}>
      {/* Header */}
      <div className="flex items-center justify-between border-b bg-background/50 px-3 py-2 backdrop-blur-sm">
        <span className="text-xs font-medium text-foreground/70">{title}</span>
        <Button
          variant="ghost"
          size="sm"
          className="h-6 w-6 p-0 text-muted-foreground transition-colors hover:text-foreground"
          onClick={handleCopy}
        >
          {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
        </Button>
      </div>

      {/* JSON Content */}
      <div
        className="json-viewer-container overflow-auto bg-background/30 p-3"
        style={{
          maxHeight: maxHeight,
        }}
      >
        <ReactJson
          src={data as object}
          theme={jsonTheme}
          collapsed={collapsed}
          enableClipboard={false}
          displayDataTypes={false}
          displayObjectSize={true}
          indentWidth={2}
          iconStyle="triangle"
          style={{
            backgroundColor: "transparent",
            fontSize: "13px",
            fontFamily:
              "ui-monospace, SFMono-Regular, 'SF Mono', Consolas, 'Liberation Mono', Menlo, monospace",
            lineHeight: "1.4",
          }}
          collapseStringsAfterLength={50}
          shouldCollapse={(field) => {
            // Collapse large arrays and objects at deeper levels
            if (field.namespace && field.namespace.length > 2) {
              if (field.type === "array" && Array.isArray(field.src) && field.src.length > 3)
                return true;
              if (
                field.type === "object" &&
                typeof field.src === "object" &&
                field.src !== null &&
                Object.keys(field.src).length > 3
              )
                return true;
            }
            return false;
          }}
        />
      </div>
    </div>
  );
}
