"use client";

import React from "react";
import { BiCopy } from "react-icons/bi";
import { toast } from "sonner";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";

interface IdDisplayProps {
  id: string;
  dim?: boolean;
}

export function IdDisplay({ id, dim = true }: IdDisplayProps) {
  const copyToClipboard = () => {
    if (!navigator.clipboard) {
      console.error("Clipboard API not supported");
      toast.error("Your browser doesn't support copying to clipboard");
      return;
    }
    navigator.clipboard
      .writeText(id)
      .then(() => {
        toast.success("Copied to clipboard");
      })
      .catch((err) => {
        console.error("Failed to copy:", err);
        toast.error("Failed to copy ID to clipboard");
      });
  };

  return (
    <TooltipProvider>
      <div className="flex w-full items-center gap-1">
        <Tooltip>
          <TooltipTrigger asChild>
            <span
              className={`text-sm ${
                dim ? "text-muted-foreground" : "text-foreground"
              } min-w-0 cursor-default truncate`}
            >
              {id}
            </span>
          </TooltipTrigger>
          <TooltipContent>
            <p>{id}</p>
          </TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger asChild>
            <button
              onClick={(e) => {
                e.preventDefault();
                copyToClipboard();
              }}
              className="text-muted-foreground hover:text-foreground"
              aria-label="Copy app ID"
            >
              <BiCopy />
            </button>
          </TooltipTrigger>
          <TooltipContent>
            <p>Copy to clipboard</p>
          </TooltipContent>
        </Tooltip>
      </div>
    </TooltipProvider>
  );
}
