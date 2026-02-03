"use client";

import { Loader2 } from "lucide-react";
import { useMetaInfo } from "@/components/context/metainfo";

export const TokenRefreshOverlay = () => {
  const { isTokenRefreshing } = useMetaInfo();

  if (!isTokenRefreshing) return null;

  return (
    <div className="fixed inset-0 z-[100] bg-background/80 backdrop-blur-sm">
      <div className="flex h-full items-center justify-center">
        <div className="flex flex-col items-center gap-2">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <p className="text-sm text-muted-foreground">Updating permissions...</p>
        </div>
      </div>
    </div>
  );
};
