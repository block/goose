"use client";

import Link from "next/link";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
} from "@/components/ui/breadcrumb";
import { sidebarItems, settingsItem } from "./app-sidebar";

interface BreadcrumbLinksProps {
  pathname: string;
}

export const BreadcrumbLinks = ({ pathname }: BreadcrumbLinksProps) => {
  const segments = pathname.split("/").filter(Boolean);
  let cumulativePath = "";
  const allRoutes = [...sidebarItems, settingsItem];
  const breadcrumbs = segments.map((segment, index) => {
    cumulativePath += "/" + segment;

    if (index === 0) {
      const matchingRoute = allRoutes.find(
        (item) => item.url === cumulativePath || item.url.startsWith(cumulativePath),
      );

      if (matchingRoute) {
        return { label: matchingRoute.title, href: cumulativePath };
      }
    }

    return {
      label: segment.charAt(0).toUpperCase() + segment.slice(1),
      href: cumulativePath,
    };
  });

  const breadcrumbsList = [];

  for (let i = 0; i < breadcrumbs.length; i++) {
    if (i > 0) {
      breadcrumbsList.push(
        <span key={i * 2 - 1} className="text-muted-foreground">
          /
        </span>,
      );
    }

    breadcrumbsList.push(
      <BreadcrumbItem key={i * 2}>
        <Link href={breadcrumbs[i].href}>{breadcrumbs[i].label}</Link>
      </BreadcrumbItem>,
    );
  }

  return (
    <Breadcrumb>
      <BreadcrumbList>
        {breadcrumbs.length > 0 ? (
          breadcrumbsList
        ) : (
          <BreadcrumbItem>
            <BreadcrumbLink href="/">Home</BreadcrumbLink>
          </BreadcrumbItem>
        )}
      </BreadcrumbList>
    </Breadcrumb>
  );
};
