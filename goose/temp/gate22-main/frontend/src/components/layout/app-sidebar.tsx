"use client";

import Link from "next/link";
import Image from "next/image";
import { cn } from "@/lib/utils";
import { usePathname } from "next/navigation";
import React from "react";
import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
  useSidebar,
} from "@/components/ui/sidebar";
import { Separator } from "@/components/ui/separator";
import { HiOutlineServerStack } from "react-icons/hi2";
import { RiSettings3Line } from "react-icons/ri";
import { Link2, Settings2, Package, ScrollText, Network, Users2, CreditCard } from "lucide-react";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useTheme } from "next-themes";
import { usePermission } from "@/hooks/use-permissions";
import { PERMISSIONS } from "@/lib/rbac/permissions";
import { isSubscriptionEnabled } from "@/lib/feature-flags";

// Export sidebar items so they can be used in header
export const sidebarItems = [
  {
    title: "MCP Servers",
    url: `/mcp-servers`,
    icon: HiOutlineServerStack,
    permission: PERMISSIONS.MCP_SERVER_PAGE_VIEW,
    adminOnly: true,
  },
  {
    title: "Available MCP Servers",
    url: `/available-mcp-servers`,
    icon: HiOutlineServerStack,
    memberOnly: true,
  },
  {
    title: "Configured MCP Servers",
    url: `/mcp-configuration`,
    icon: Settings2,
  },
  {
    title: "Connected Accounts",
    url: `/connected-accounts`,
    icon: Link2,
  },
  {
    title: "MCP Bundles",
    url: `/bundle-mcp`,
    icon: Package,
  },
  {
    title: "Members",
    url: "/members",
    icon: Users2,
    adminOnly: true,
  },
  {
    title: "Teams",
    url: "/teams",
    icon: Network,
    adminOnly: true,
  },
  {
    title: "Organization Settings",
    url: "/organization-settings",
    icon: RiSettings3Line,
    adminOnly: true,
  },
];

export const mcpNavigationItems = [
  {
    title: "MCP Servers",
    url: "/mcp-servers",
    icon: HiOutlineServerStack,
    adminOnly: true,
  },
  {
    title: "Available MCP Servers",
    url: "/available-mcp-servers",
    icon: HiOutlineServerStack,
    memberOnly: true,
  },

  {
    title: "Configured MCP Servers",
    url: "/mcp-configuration",
    icon: Settings2,
  },
  {
    title: "Connected Accounts",
    url: "/connected-accounts",
    icon: Link2,
  },
  {
    title: "MCP Bundles",
    url: "/bundle-mcp",
    icon: Package,
  },
  {
    title: "Logs",
    url: "/logs",
    icon: ScrollText,
  },
];

export const organizationNavigationItems = [
  {
    title: "Members",
    url: "/members",
    icon: Users2,
    adminOnly: true,
  },
  {
    title: "Teams",
    url: "/teams",
    icon: Network,
    adminOnly: true,
  },
  {
    title: "Organization Settings",
    url: "/organization-settings",
    icon: RiSettings3Line,
    adminOnly: true,
  },
  ...(isSubscriptionEnabled()
    ? [
        {
          title: "Subscription",
          url: "/subscription",
          icon: CreditCard,
          adminOnly: true,
        },
      ]
    : []),
];

// Add settings routes to be accessible in header
export const settingsItem = {
  title: "Settings",
  url: "/organization-settings",
  icon: RiSettings3Line,
};

export function AppSidebar() {
  const { state } = useSidebar();
  const isCollapsed = state === "collapsed";
  const pathname = usePathname();
  const { resolvedTheme } = useTheme();
  const canViewMCPConfiguration = usePermission(PERMISSIONS.MCP_CONFIGURATION_PAGE_VIEW);
  const isAdmin = usePermission(PERMISSIONS.MCP_CONFIGURATION_CREATE);

  // Common filter for nav items based on permissions
  const filterNavItems = (items: typeof sidebarItems) =>
    items.filter((item) => {
      // Hide Configured MCP Servers for users without permission
      if (item.title === "Configured MCP Servers" && !canViewMCPConfiguration) {
        return false;
      }
      // Hide admin-only items from members
      if ("adminOnly" in item && item.adminOnly && !isAdmin) {
        return false;
      }
      // Hide member-only items from admins
      if ("memberOnly" in item && item.memberOnly && isAdmin) {
        return false;
      }
      return true;
    });

  const filteredMcpItems = filterNavItems(mcpNavigationItems as unknown as typeof sidebarItems);
  const filteredOrganizationItems = filterNavItems(
    organizationNavigationItems as unknown as typeof sidebarItems,
  );

  return (
    <Sidebar collapsible="icon" className="flex flex-col">
      <SidebarHeader className="flex flex-col gap-0 p-0">
        <div
          className={cn(
            "flex h-[60px] items-center px-6",
            isCollapsed ? "justify-center" : "justify-between gap-2",
          )}
        >
          {!isCollapsed && (
            <div className="relative flex h-7 w-auto items-center justify-center">
              <Image
                src={`/aci-dev-full-logo-${resolvedTheme ?? "light"}-bg.svg`}
                alt="ACI Dev Logo"
                width={150}
                height={28}
                priority
                className="h-full object-contain"
              />
            </div>
          )}
          <SidebarTrigger />
        </div>
        <Separator />
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {filteredMcpItems.map((item) => {
                const isActive = pathname === item.url || pathname.startsWith(item.url);
                return (
                  <SidebarMenuItem key={item.title}>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <SidebarMenuButton asChild>
                          <Link
                            href={item.url}
                            className={cn(
                              "flex h-9 items-center gap-3 px-4 transition-colors",
                              isCollapsed && "justify-center",
                              isActive && "bg-primary/10 font-medium text-primary",
                            )}
                          >
                            <item.icon
                              className={cn("h-5 w-5 shrink-0", isActive && "text-primary")}
                            />
                            {!isCollapsed && <span>{item.title}</span>}
                          </Link>
                        </SidebarMenuButton>
                      </TooltipTrigger>
                      {isCollapsed && <TooltipContent side="right">{item.title}</TooltipContent>}
                    </Tooltip>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>

            <Separator className="my-4" />

            <SidebarMenu>
              {filteredOrganizationItems.map((item) => {
                const isActive = pathname === item.url || pathname.startsWith(item.url);
                return (
                  <SidebarMenuItem key={item.title}>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <SidebarMenuButton asChild>
                          <Link
                            href={item.url}
                            className={cn(
                              "flex h-9 items-center gap-3 px-4 transition-colors",
                              isCollapsed && "justify-center",
                              isActive && "bg-primary/10 font-medium text-primary",
                            )}
                          >
                            <item.icon
                              className={cn("h-5 w-5 shrink-0", isActive && "text-primary")}
                            />
                            {!isCollapsed && <span>{item.title}</span>}
                          </Link>
                        </SidebarMenuButton>
                      </TooltipTrigger>
                      {isCollapsed && <TooltipContent side="right">{item.title}</TooltipContent>}
                    </Tooltip>
                  </SidebarMenuItem>
                );
              })}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
    </Sidebar>
  );
}
