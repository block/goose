"use client";

import { useState, useMemo, useCallback } from "react";
import { useRouter } from "next/navigation";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Search, Loader2, Plus, Check } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import Image from "next/image";
import { useMCPServerConfigurations } from "@/features/mcp/hooks/use-mcp-servers";
import { Button } from "@/components/ui/button";
import { BundleMCPStepperForm } from "@/features/bundle-mcp/components/bundle-mcp-stepper-form";
import { useCreateMCPServerBundle } from "@/features/bundle-mcp/hooks/use-bundle-mcp";
import { CreateMCPServerBundleInput } from "@/features/bundle-mcp/types/bundle-mcp.types";
import { cn } from "@/lib/utils";
import { useConnectedAccounts } from "@/features/connected-accounts/hooks/use-connected-account";
import { getOwnershipLabel } from "@/utils/configuration-labels";

export default function AvailableMCPServersPage() {
  const router = useRouter();
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("all");
  const [selectedConfigs, setSelectedConfigs] = useState<Set<string>>(new Set());
  const [isCreatingBundle, setIsCreatingBundle] = useState(false);
  const [isBundleDialogOpen, setIsBundleDialogOpen] = useState(false);

  // Fetch MCP configurations available to the member
  const {
    data: configurationsResponse,
    isLoading,
    error,
  } = useMCPServerConfigurations({
    limit: 100,
  });

  const { mutateAsync: createBundle } = useCreateMCPServerBundle();

  const handleSelectionChange = useCallback((ids: string[]) => {
    setSelectedConfigs((prev) => {
      const next = new Set(ids);
      if (next.size === prev.size) {
        let isSame = true;
        for (const id of prev) {
          if (!next.has(id)) {
            isSame = false;
            break;
          }
        }
        if (isSame) {
          return prev;
        }
      }
      return next;
    });
  }, []);

  // Fetch connected accounts for all configurations
  const { data: connectedAccounts = [] } = useConnectedAccounts();

  const configurations = useMemo(
    () => configurationsResponse?.data || [],
    [configurationsResponse?.data],
  );

  // Get unique categories from MCP servers
  const categories = useMemo(() => {
    const allCategories = configurations.flatMap((config) => config.mcp_server?.categories || []);
    return ["all", ...Array.from(new Set(allCategories))].sort();
  }, [configurations]);

  // Filter configurations based on search and category
  const filteredConfigurations = useMemo(() => {
    return configurations.filter((config) => {
      const matchesSearch =
        searchQuery.toLowerCase() === "" ||
        config.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        config.description?.toLowerCase().includes(searchQuery.toLowerCase()) ||
        config.mcp_server?.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        config.mcp_server?.description.toLowerCase().includes(searchQuery.toLowerCase());

      const matchesCategory =
        selectedCategory === "all" || config.mcp_server?.categories?.includes(selectedCategory);

      return matchesSearch && matchesCategory;
    });
  }, [searchQuery, selectedCategory, configurations]);

  const handleCardToggle = (configId: string) => {
    setSelectedConfigs((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(configId)) {
        newSet.delete(configId);
      } else {
        newSet.add(configId);
      }
      return newSet;
    });
  };

  const handleCreateBundle = async (values: CreateMCPServerBundleInput) => {
    setIsCreatingBundle(true);
    try {
      await createBundle(values);
      setSelectedConfigs(new Set());
      setIsBundleDialogOpen(false);
      router.push("/bundle-mcp");
    } catch (error) {
      console.error("Failed to create bundle:", error);
    } finally {
      setIsCreatingBundle(false);
    }
  };

  return (
    <div>
      {/* Header */}
      <div className="border-b px-4 py-3">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-2xl font-bold">Available MCP Servers</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Browse available MCP server configurations and create bundles for your AI agents
            </p>
          </div>
          {selectedConfigs.size > 0 && (
            <Button
              variant="default"
              disabled={isCreatingBundle}
              onClick={() => setIsBundleDialogOpen(true)}
            >
              <Plus className="mr-2 h-4 w-4" />
              Create Bundle ({selectedConfigs.size} selected)
            </Button>
          )}
        </div>
      </div>

      <div className="space-y-4 p-4">
        {/* Search and Filters */}
        <div className="flex flex-col gap-4 sm:flex-row">
          <div className="relative max-w-md flex-1">
            <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 transform text-muted-foreground" />
            <Input
              placeholder="Search available MCP servers..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10"
            />
          </div>
          <Select value={selectedCategory} onValueChange={setSelectedCategory}>
            <SelectTrigger className="w-full sm:w-[200px]">
              <SelectValue placeholder="Select category" />
            </SelectTrigger>
            <SelectContent>
              {categories.map((category) => (
                <SelectItem key={category} value={category}>
                  {category === "all" ? "All Categories" : category}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Selection info */}
        {selectedConfigs.size > 0 && (
          <div className="rounded-lg border border-primary/20 bg-primary/10 px-3 py-2 text-sm">
            <span className="font-medium">{selectedConfigs.size} configuration(s) selected.</span>{" "}
            Click the &quot;Create Bundle&quot; button above to bundle them together.
          </div>
        )}

        {/* Results count */}
        <div className="text-sm text-muted-foreground">
          Showing {filteredConfigurations.length} available MCP server configurations
        </div>

        {/* Loading state */}
        {isLoading && (
          <div className="flex items-center justify-center py-12">
            <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
          </div>
        )}

        {/* Error state */}
        {error && (
          <div className="py-12 text-center">
            <p className="text-red-500">
              Failed to load MCP server configurations. Please try again.
            </p>
          </div>
        )}

        {/* Configuration Cards Grid */}
        {!isLoading && !error && (
          <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
            {filteredConfigurations.map((config) => {
              const isSelected = selectedConfigs.has(config.id);
              const configAccounts = connectedAccounts.filter(
                (account) => account.mcp_server_configuration_id === config.id,
              );
              const requiresAccount = !!config.connected_account_ownership;
              const hasNoAccounts = requiresAccount && configAccounts.length === 0;

              return (
                <Card
                  key={config.id}
                  className={cn(
                    "group relative min-h-[240px] cursor-pointer transition-all hover:shadow-md",
                    isSelected && "bg-primary/5 ring-1 ring-primary",
                  )}
                  onClick={() => router.push(`/mcp-configuration/${config.id}`)}
                >
                  <div className="flex h-full flex-col">
                    {/* Bundle selector - absolute positioned */}
                    <div className="absolute top-6 right-6 z-10">
                      <button
                        className={cn(
                          "flex h-[34px] shrink-0 items-center gap-2 rounded-md border px-3 py-1.5 transition-all",
                          hasNoAccounts
                            ? "cursor-not-allowed border-button-disabled-border bg-button-disabled text-button-disabled-foreground opacity-40"
                            : isSelected
                              ? "border-primary bg-primary text-primary-foreground shadow-sm hover:bg-primary/90"
                              : "border-button-outline-border bg-button-outline text-button-outline-foreground hover:bg-button-outline-hover hover:shadow-sm",
                        )}
                        disabled={hasNoAccounts}
                        onClick={(e) => {
                          e.stopPropagation();
                          if (!hasNoAccounts) {
                            handleCardToggle(config.id);
                          }
                        }}
                        title={hasNoAccounts ? "Setup connected account first" : "Add to bundle"}
                      >
                        <div
                          className={cn(
                            "flex h-5 w-5 items-center justify-center rounded border transition-all",
                            hasNoAccounts
                              ? "border-button-disabled-border bg-button-disabled"
                              : isSelected
                                ? "border-primary-foreground bg-primary-foreground"
                                : "border-button-outline-border bg-button-outline",
                          )}
                        >
                          {isSelected && !hasNoAccounts && (
                            <Check className="h-3 w-3 text-primary" />
                          )}
                        </div>
                        <span
                          className={cn(
                            "text-sm font-semibold",
                            !hasNoAccounts && !isSelected && "group-hover:text-primary",
                          )}
                        >
                          Bundle
                        </span>
                      </button>
                    </div>

                    {/* Header content */}
                    <CardHeader className="block pb-3">
                      <div className="mb-2 flex items-start gap-3">
                        {config.mcp_server?.logo && (
                          <div className="flex size-10 shrink-0 items-center justify-center">
                            <Image
                              src={config.mcp_server.logo}
                              alt={`${config.mcp_server.name} logo`}
                              width={40}
                              height={40}
                              className="object-contain"
                              unoptimized
                            />
                          </div>
                        )}
                        <div className="min-w-0 flex-1 pr-28">
                          <CardTitle className="truncate text-base font-semibold">
                            {config.name}
                          </CardTitle>
                          <p className="mt-0.5 text-xs text-muted-foreground">
                            {config.mcp_server?.name}
                          </p>
                        </div>
                      </div>

                      {/* Connected Account Info */}
                      {config.connected_account_ownership && (
                        <div className="mb-2">
                          <div className="flex items-center gap-2">
                            <Badge variant="outline" className="text-xs">
                              {getOwnershipLabel(config.connected_account_ownership)}
                            </Badge>
                            {(() => {
                              if (configAccounts.length > 0) {
                                return (
                                  <span className="text-xs text-green-600">
                                    {configAccounts.length} account
                                    {configAccounts.length > 1 ? "s" : ""} available
                                  </span>
                                );
                              } else {
                                return (
                                  <span className="text-xs text-amber-600">
                                    Connect an account to use this MCP
                                  </span>
                                );
                              }
                            })()}
                          </div>
                        </div>
                      )}

                      <CardDescription className="line-clamp-2 text-sm text-muted-foreground">
                        {config.description || config.mcp_server?.description}
                      </CardDescription>
                    </CardHeader>

                    {/* Spacer to push content to bottom */}
                    <div className="flex-1" />

                    {/* Bottom content - fixed height for alignment */}
                    <CardContent className="px-6 pt-0 pb-0">
                      <div className="flex min-h-[28px] items-end justify-between gap-2">
                        {/* Categories - fixed height container */}
                        <div className="flex min-w-0 flex-1 flex-wrap items-end gap-1">
                          {config.mcp_server?.categories &&
                          config.mcp_server.categories.length > 0 ? (
                            config.mcp_server.categories.map((category) => (
                              <Badge key={category} variant="secondary" className="text-xs">
                                {category}
                              </Badge>
                            ))
                          ) : (
                            <div className="h-[22px]" />
                          )}
                        </div>
                      </div>
                    </CardContent>
                  </div>
                </Card>
              );
            })}
          </div>
        )}

        {/* Empty state */}
        {!isLoading && !error && filteredConfigurations.length === 0 && (
          <div className="py-12 text-center">
            <p className="text-muted-foreground">
              No MCP server configurations available. Please contact your administrator to assign
              MCP servers to your team.
            </p>
          </div>
        )}
      </div>

      {/* Bundle MCP Stepper Form Dialog */}
      <BundleMCPStepperForm
        isOpen={isBundleDialogOpen}
        onClose={() => setIsBundleDialogOpen(false)}
        availableConfigurations={configurations}
        connectedAccounts={connectedAccounts}
        selectedIds={Array.from(selectedConfigs)}
        onSelectionChange={handleSelectionChange}
        onSubmit={handleCreateBundle}
      />
    </div>
  );
}
