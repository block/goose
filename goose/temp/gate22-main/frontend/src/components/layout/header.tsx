"use client";

// import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogTrigger,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
// import { GoBell } from "react-icons/go";
import { BsQuestionCircle, BsDiscord, BsBook, BsExclamationTriangle } from "react-icons/bs";
import { Separator } from "@/components/ui/separator";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { BreadcrumbLinks } from "./BreadcrumbLinks";
import { usePathname } from "next/navigation";
import { OrgSelector } from "./org-selector";
import { RoleSelector } from "./role-selector";
import { UserProfileDropdown } from "./user-profile-dropdown";
import { useSubscriptionStatus } from "@/features/settings/hooks/use-subscription-status";
import { PermissionGuard } from "@/components/rbac/permission-guard";
import { PERMISSIONS } from "@/lib/rbac/permissions";
import { isSubscriptionEnabled } from "@/lib/feature-flags";
import Link from "next/link";

export const Header = () => {
  const pathname = usePathname();
  const { data: subscriptionStatus } = useSubscriptionStatus();

  return (
    <header className="sticky top-0 z-50 flex-shrink-0 bg-background">
      <div className="flex w-full items-center justify-between px-4 py-3">
        <div className="flex items-center gap-4">
          {/* Organization Selector and Breadcrumbs */}
          <div className="flex items-center gap-2">
            <div className="w-44">
              <OrgSelector />
            </div>
            <span className="text-muted-foreground">/</span>
            <BreadcrumbLinks pathname={pathname} />
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Usage Exceeded Warning */}
          {isSubscriptionEnabled() && (
            <PermissionGuard permission={PERMISSIONS.SUBSCRIPTION_PAGE_VIEW}>
              {subscriptionStatus?.is_usage_exceeded && (
                <TooltipProvider>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <div className="flex h-9 cursor-default items-center rounded-md bg-yellow-500 px-3 text-black">
                        <BsExclamationTriangle className="mr-1 h-4 w-4" />
                        <span className="text-xs font-medium">Alert</span>
                      </div>
                    </TooltipTrigger>
                    <TooltipContent side="bottom" className="max-w-sm">
                      <p className="text-sm">
                        Your organization&apos;s current usage has exceeded the limit of its
                        subscription. MCP access will be affected until the usage is reduced. Please
                        see the{" "}
                        <Link href="/settings/subscription" className="font-medium underline">
                          subscription
                        </Link>{" "}
                        page for details.
                      </p>
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              )}
            </PermissionGuard>
          )}

          <Dialog>
            <DialogTrigger asChild>
              <Button variant="outline" className="h-9 px-2">
                <BsQuestionCircle />
                <span>Support</span>
              </Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Support</DialogTitle>
              </DialogHeader>
              <p>For support or to report a bug, please email us at support@aipolabs.xyz</p>
            </DialogContent>
          </Dialog>

          <a href="https://discord.gg/bT2eQ2m9vm" target="_blank" rel="noopener noreferrer">
            <Button variant="outline" className="h-9 px-2">
              <BsDiscord />
              <span>Discord</span>
            </Button>
          </a>

          <a
            href="https://gate22-docs.aci.dev/introduction/overview"
            target="_blank"
            rel="noopener noreferrer"
          >
            <Button variant="outline" className="h-9 px-2">
              <BsBook />
              <span>Docs</span>
            </Button>
          </a>

          {/* <Button variant="outline" className="px-2 mx-2">
            <GoBell />
          </Button> */}

          <div className="mx-1 h-6 w-px bg-border" />

          <RoleSelector />

          <UserProfileDropdown />
        </div>
      </div>
      <Separator />
    </header>
  );
};
