"use client";

import React from "react";
import { toast } from "sonner";
import { useTheme } from "next-themes";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { useMetaInfo } from "@/components/context/metainfo";
import { LogOut, Sun, Moon, Monitor } from "lucide-react";

export function UserProfileDropdown() {
  const { user, logout } = useMetaInfo();
  const { theme, setTheme } = useTheme();

  const handleLogout = async () => {
    try {
      await logout();
      toast.success("Logged out successfully");
    } catch (error) {
      console.error("Logout failed:", error);
      toast.error("Failed to logout");
    }
  };

  // Get initials from user's first name or email
  const getInitials = () => {
    if (user.firstName) {
      return user.firstName.charAt(0).toUpperCase();
    }
    return user.email.charAt(0).toUpperCase();
  };

  // Get display name
  const getDisplayName = () => {
    if (user.firstName && user.lastName) {
      return `${user.firstName} ${user.lastName}`;
    }
    if (user.firstName) {
      return user.firstName;
    }
    return user.email;
  };

  // Get consistent avatar URL with grey background (avoid PII leakage)
  const getAvatarUrl = () => {
    if (user.pictureUrl && !user.pictureUrl.includes("ui-avatars.com")) {
      return user.pictureUrl;
    }
    // Use initials only to avoid leaking full name/email
    const initials = encodeURIComponent(getInitials());
    return `https://ui-avatars.com/api/?name=${initials}&background=e5e7eb&color=111827&bold=true`;
  };
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          variant="ghost"
          className="relative h-9 w-9 rounded-full transition-all duration-200 hover:bg-gray-100 focus-visible:ring-2 focus-visible:ring-gray-300 focus-visible:outline-none dark:hover:bg-gray-800 dark:focus-visible:ring-gray-600"
        >
          <Avatar className="h-9 w-9">
            <AvatarImage src={getAvatarUrl()} alt={getDisplayName()} />
            <AvatarFallback className="bg-gray-100 text-sm font-semibold text-gray-900 dark:bg-gray-800 dark:text-gray-100">
              {getInitials()}
            </AvatarFallback>
          </Avatar>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-64 shadow-lg" align="end" forceMount>
        <DropdownMenuLabel className="font-normal">
          <div className="flex flex-col space-y-2">
            <div className="flex items-center gap-3">
              <Avatar className="h-10 w-10">
                <AvatarImage src={getAvatarUrl()} alt={getDisplayName()} />
                <AvatarFallback className="bg-gray-100 font-semibold text-gray-900 dark:bg-gray-800 dark:text-gray-100">
                  {getInitials()}
                </AvatarFallback>
              </Avatar>
              <div className="flex flex-col space-y-1">
                <p className="text-sm leading-none font-medium">{getDisplayName()}</p>
                <p className="text-xs leading-none text-muted-foreground">{user.email}</p>
              </div>
            </div>
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />

        {/* Theme Selector */}
        <div className="mr-2 px-2 py-2">
          <p className="mb-2 text-xs font-medium text-muted-foreground">Theme</p>
          <div className="flex items-center gap-1">
            <Button
              variant={theme === "light" ? "secondary" : "ghost"}
              size="sm"
              onClick={() => setTheme("light")}
              className="h-8 flex-1 px-1"
            >
              <Sun className="h-4 w-4" />
              <span className="text-xs">Light</span>
            </Button>
            <Button
              variant={theme === "dark" ? "secondary" : "ghost"}
              size="sm"
              onClick={() => setTheme("dark")}
              className="h-8 flex-1 px-2"
            >
              <Moon className="h-4 w-4" />
              <span className="text-xs">Dark</span>
            </Button>
            <Button
              variant={theme === "system" ? "secondary" : "ghost"}
              size="sm"
              onClick={() => setTheme("system")}
              className="h-8 flex-1 px-2"
            >
              <Monitor className="h-4 w-4" />
              <span className="text-xs">System</span>
            </Button>
          </div>
        </div>

        <DropdownMenuSeparator />

        <DropdownMenuItem
          onClick={handleLogout}
          className="cursor-pointer text-destructive transition-colors duration-150 hover:bg-destructive/10 hover:text-destructive focus:bg-destructive/10 focus:text-destructive"
        >
          <LogOut className="mr-2 h-4 w-4" />
          <span>Log out</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
