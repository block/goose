"use client";

import { MCPToolBasic } from "../types/mcp.types";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Code2, FileJson, Loader2, AlertCircle } from "lucide-react";
import { JsonViewer } from "./json-viewer";
import { useMCPTool } from "../hooks/use-mcp-servers";

interface ToolSchemaDrawerProps {
  tool: MCPToolBasic | null;
  isOpen: boolean;
  onClose: () => void;
}

export function ToolSchemaDrawer({ tool, isOpen, onClose }: ToolSchemaDrawerProps) {
  // Fetch full tool details including schema when drawer opens
  const { data: fullTool, isLoading, error } = useMCPTool(tool?.name || "");

  if (!tool) return null;

  const renderSchemaContent = (
    schema?: Record<string, unknown>,
    type: "input" | "output" = "input",
  ) => {
    // Show loading state while fetching
    if (isLoading) {
      return (
        <div className="flex flex-col items-center rounded-lg border border-dashed bg-muted/5 py-8">
          <Loader2 className="mb-2 h-8 w-8 animate-spin text-muted-foreground/30" />
          <p className="text-center text-xs font-medium text-muted-foreground">Loading schema...</p>
        </div>
      );
    }

    // Show error state if fetch failed
    if (error) {
      return (
        <div className="flex flex-col items-center rounded-lg border border-dashed bg-destructive/5 py-8">
          <AlertCircle className="mb-2 h-8 w-8 text-destructive/50" />
          <p className="text-center text-xs font-medium text-destructive">Failed to load schema</p>
          <p className="mt-1 text-center text-xs text-muted-foreground">
            {error instanceof Error ? error.message : "Unknown error"}
          </p>
        </div>
      );
    }

    // Only input schema is available from the API
    if (type === "output") {
      return (
        <div className="rounded-lg border border-dashed bg-muted/5 py-8">
          <FileJson className="mx-auto mb-2 h-8 w-8 text-muted-foreground/30" />
          <p className="text-center text-xs font-medium text-muted-foreground">
            Output schema not available
          </p>
        </div>
      );
    }

    if (!schema) {
      return (
        <div className="rounded-lg border border-dashed bg-muted/5 py-8">
          <FileJson className="mx-auto mb-2 h-8 w-8 text-muted-foreground/30" />
          <p className="text-center text-xs font-medium text-muted-foreground">No {type} schema</p>
        </div>
      );
    }

    return (
      <JsonViewer
        data={schema}
        title={`${type.charAt(0).toUpperCase() + type.slice(1)} Parameters`}
        className="border"
        maxHeight="400px"
        collapsed={2}
      />
    );
  };

  return (
    <Sheet open={isOpen} onOpenChange={onClose}>
      <SheetContent className="w-full overflow-y-auto p-0 sm:max-w-2xl">
        <SheetHeader className="p-6 pb-0">
          <SheetTitle className="text-xl leading-none font-semibold tracking-tight">
            {tool.name}
          </SheetTitle>
          {tool.description && (
            <SheetDescription className="mt-2 text-sm leading-relaxed whitespace-pre-wrap text-muted-foreground">
              {tool.description}
            </SheetDescription>
          )}
        </SheetHeader>
        <div className="p-6 pt-4">
          {/* Schema Content with Compact Tabs */}
          <div className="space-y-3">
            <h3 className="text-xs font-semibold tracking-wider text-foreground/80 uppercase">
              Schema Definition
            </h3>

            <Tabs defaultValue="input" className="w-full">
              <TabsList className="grid h-9 w-full grid-cols-2 border bg-muted/20 px-1">
                <TabsTrigger
                  value="input"
                  className="h-7 text-xs font-medium data-[state=active]:border data-[state=active]:border-muted-foreground/20 data-[state=active]:bg-background data-[state=active]:shadow-sm"
                >
                  <Code2 className="mr-1.5 h-3 w-3" />
                  Input
                </TabsTrigger>
                <TabsTrigger
                  value="output"
                  className="h-7 text-xs font-medium data-[state=active]:border data-[state=active]:border-muted-foreground/20 data-[state=active]:bg-background data-[state=active]:shadow-sm"
                >
                  <FileJson className="mr-1.5 h-3 w-3" />
                  Output
                </TabsTrigger>
              </TabsList>

              <TabsContent value="input" className="mt-3 space-y-3">
                {renderSchemaContent(fullTool?.input_schema, "input")}
              </TabsContent>

              <TabsContent value="output" className="mt-3 space-y-3">
                {renderSchemaContent(undefined, "output")}
              </TabsContent>
            </Tabs>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}
