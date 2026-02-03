"use client";

import { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ChevronDown, ChevronRight } from "lucide-react";
import { ToolsSyncResult } from "../types/mcp.types";

interface SyncResultsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  results: ToolsSyncResult | null;
  serverName: string;
}

export function SyncResultsDialog({
  open,
  onOpenChange,
  results,
  serverName,
}: SyncResultsDialogProps) {
  const [expandedSections, setExpandedSections] = useState({
    created: false,
    updated: false,
    deleted: false,
    unchanged: false,
  });

  if (!results) return null;

  const toggleSection = (section: keyof typeof expandedSections) => {
    setExpandedSections((prev) => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  const CollapsibleSection = ({
    title,
    count,
    tools,
    colorClass,
    sectionKey,
    badgeClass,
  }: {
    title: string;
    count: number;
    tools: string[];
    colorClass: string;
    sectionKey: keyof typeof expandedSections;
    badgeClass: string;
  }) => {
    const isExpanded = expandedSections[sectionKey];

    return (
      <div>
        <Button
          variant="ghost"
          className="mb-3 flex h-auto items-center gap-2 p-0 hover:bg-transparent"
          onClick={() => toggleSection(sectionKey)}
        >
          {isExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
          <h3 className={`font-semibold ${colorClass}`}>
            {title} ({count})
          </h3>
        </Button>
        {isExpanded && count > 0 && (
          <div className="ml-6 flex flex-wrap gap-2">
            {tools.map((tool) => (
              <Badge key={tool} variant="outline" className={badgeClass}>
                {tool}
              </Badge>
            ))}
          </div>
        )}
        {isExpanded && count === 0 && (
          <div className="ml-6 text-sm text-muted-foreground">None</div>
        )}
      </div>
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[80vh] overflow-y-auto sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Tools Sync Results for {serverName}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Created Tools */}
          <CollapsibleSection
            title="Created Tools"
            count={results.tools_created.length}
            tools={results.tools_created}
            colorClass="text-gray-600"
            sectionKey="created"
            badgeClass="border-green-200 text-green-700"
          />

          {/* Updated Tools */}
          <CollapsibleSection
            title="Updated Tools"
            count={results.tools_updated.length}
            tools={results.tools_updated}
            colorClass="text-gray-600"
            sectionKey="updated"
            badgeClass="border-blue-200 text-blue-700"
          />

          {/* Deleted Tools */}
          <CollapsibleSection
            title="Deleted Tools"
            count={results.tools_deleted.length}
            tools={results.tools_deleted}
            colorClass="text-gray-600"
            sectionKey="deleted"
            badgeClass="border-red-200 text-red-700"
          />

          {/* Unchanged Tools */}
          <CollapsibleSection
            title="Unchanged Tools"
            count={results.tools_unchanged.length}
            tools={results.tools_unchanged}
            colorClass="text-gray-600"
            sectionKey="unchanged"
            badgeClass="border-gray-200 text-gray-700"
          />
        </div>
      </DialogContent>
    </Dialog>
  );
}
