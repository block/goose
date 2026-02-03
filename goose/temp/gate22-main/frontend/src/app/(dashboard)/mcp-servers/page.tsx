"use client";

import { useState, useMemo, useEffect } from "react";
import { useRouter } from "next/navigation";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { usePermission } from "@/hooks/use-permissions";
import { PERMISSIONS } from "@/lib/rbac/permissions";
import { useMetaInfo } from "@/components/context/metainfo";
import { Shield } from "lucide-react";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Search, Loader2 } from "lucide-react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import Image from "next/image";
import { useMCPServers } from "@/features/mcp/hooks/use-mcp-servers";
import { Button } from "@/components/ui/button";
import { Plus } from "lucide-react";
import { PermissionGuard } from "@/components/rbac/permission-guard";

export default function MCPServersPage() {
  const router = useRouter();
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState("all");
  const [page, setPage] = useState(0);
  const pageSize = 100;
  const canView = usePermission(PERMISSIONS.MCP_CONFIGURATION_PAGE_VIEW);
  const { activeOrg } = useMetaInfo();

  // Redirect members to Available MCP Servers page
  useEffect(() => {
    if (activeOrg && !canView) {
      router.push("/available-mcp-servers");
      // Don't show error toast - this redirect might be due to intentional role switching
    }
  }, [activeOrg, canView, router]);

  // Fetch MCP servers using the new hook
  const {
    data: serversResponse,
    isLoading,
    error,
  } = useMCPServers({
    offset: page * pageSize,
    limit: pageSize,
  });

  const servers = useMemo(() => serversResponse?.data || [], [serversResponse?.data]);

  // Get unique categories
  const categories = useMemo(() => {
    const allCategories = servers.flatMap((server) => server.categories);
    return ["all", ...Array.from(new Set(allCategories))].sort();
  }, [servers]);

  // Filter servers based on search and category
  const filteredServers = useMemo(() => {
    return servers.filter((server) => {
      const matchesSearch =
        searchQuery.toLowerCase() === "" ||
        server.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        server.description.toLowerCase().includes(searchQuery.toLowerCase());

      const matchesCategory =
        selectedCategory === "all" || server.categories.includes(selectedCategory);

      return matchesSearch && matchesCategory;
    });
  }, [searchQuery, selectedCategory, servers]);

  // Show access denied for non-admins only after org context is loaded
  if (activeOrg && !canView) {
    return (
      <div className="flex min-h-[60vh] flex-col items-center justify-center text-center">
        <Shield className="mb-4 h-12 w-12 text-muted-foreground" />
        <h3 className="mb-2 text-lg font-semibold">Access Restricted</h3>
        <p className="max-w-md text-muted-foreground">
          This page is restricted to administrators only. Redirecting to Available MCP Servers...
        </p>
      </div>
    );
  }

  return (
    <div>
      {/* Header */}
      <div className="border-b px-4 py-3">
        <div className="flex items-start justify-between">
          <div>
            <h2 className="text-2xl font-bold">MCP Servers</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Browse and configure Model Context Protocol servers to extend your AI agent&apos;s
              capabilities
            </p>
          </div>
          <PermissionGuard permission={PERMISSIONS.CUSTOM_MCP_SERVER_CREATE}>
            <Button
              onClick={() => router.push("/mcp-servers/custom/new")}
              className="flex items-center gap-2"
            >
              <Plus className="h-4 w-4" />
              Add Custom MCP Server
            </Button>
          </PermissionGuard>
        </div>
      </div>

      <div className="space-y-4 p-4">
        {/* Search and Filters */}
        <div className="flex flex-col gap-4 sm:flex-row">
          <div className="relative max-w-md flex-1">
            <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 transform text-muted-foreground" />
            <Input
              placeholder="Search MCP servers..."
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

        {/* Results count */}
        <div className="text-sm text-muted-foreground">
          Showing {filteredServers.length} of {servers.length} MCP servers
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
            <p className="text-red-500">Failed to load MCP servers. Please try again.</p>
          </div>
        )}

        {/* Integration Grid */}
        {!isLoading && !error && (
          <div className="grid grid-cols-1 gap-6 md:grid-cols-2 lg:grid-cols-3">
            {filteredServers.map((server) => (
              <Card
                key={server.id}
                className="flex min-h-[180px] cursor-pointer flex-col transition-shadow hover:shadow-md"
                onClick={() => router.push(`/mcp-servers/${server.id}`)}
              >
                <CardHeader className="flex flex-1 flex-col pb-2">
                  <div className="mb-2 flex items-center gap-2">
                    <div className="flex size-8 shrink-0 items-center justify-center">
                      <Image
                        src={server.logo}
                        alt={`${server.name} logo`}
                        width={32}
                        height={32}
                        className="object-contain"
                        unoptimized
                      />
                    </div>
                    <CardTitle className="text-lg">{server.name}</CardTitle>
                  </div>
                  <CardDescription className="line-clamp-2 flex-1 text-sm">
                    {server.description}
                  </CardDescription>
                </CardHeader>
                <CardContent className="pt-0 pb-0">
                  {/* Categories */}
                  <div className="flex flex-wrap gap-1">
                    {server.categories.map((category) => (
                      <Badge key={category} variant="secondary" className="text-xs">
                        {category}
                      </Badge>
                    ))}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}

        {/* Empty state */}
        {!isLoading && !error && filteredServers.length === 0 && (
          <div className="py-12 text-center">
            <p className="text-muted-foreground">No MCP servers found matching your criteria.</p>
          </div>
        )}

        {/* Pagination */}
        {servers.length > pageSize && (
          <div className="mt-6 flex justify-center gap-2">
            <Button
              variant="outline"
              onClick={() => setPage(Math.max(0, page - 1))}
              disabled={page === 0}
            >
              Previous
            </Button>
            <span className="flex items-center px-3 text-sm text-muted-foreground">
              Page {page + 1} of {Math.ceil(servers.length / pageSize)}
            </span>
            <Button
              variant="outline"
              onClick={() => setPage(page + 1)}
              disabled={(page + 1) * pageSize >= servers.length}
            >
              Next
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
