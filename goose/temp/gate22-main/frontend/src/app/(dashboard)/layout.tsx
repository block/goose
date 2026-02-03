"use client";

import { Header } from "@/components/layout/header";
import { Footer } from "@/components/layout/footer";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { AppSidebar } from "@/components/layout/app-sidebar";
import { MetaInfoProvider } from "@/components/context/metainfo";
import { TokenRefreshOverlay } from "@/components/layout/token-refresh-overlay";
import { Toaster } from "@/components/ui/sonner";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: false,
      refetchOnWindowFocus: true,
      refetchOnReconnect: true,
      refetchInterval: 60_000,
      refetchIntervalInBackground: true,
      staleTime: 5 * 60_000,
      gcTime: 15 * 60_000,
      placeholderData: (prev: unknown) => prev,
    },
  },
});

export default function DashboardLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <div className="h-full">
      <QueryClientProvider client={queryClient}>
        <MetaInfoProvider>
          <TokenRefreshOverlay />
          <SidebarProvider className="h-full">
            <AppSidebar />
            <SidebarInset className="flex h-full flex-col overflow-hidden">
              <Header />
              <main className="flex-1 overflow-y-auto">{children}</main>
            </SidebarInset>
          </SidebarProvider>
        </MetaInfoProvider>
        <Footer />
        <ReactQueryDevtools initialIsOpen={false} />
        <Toaster
          richColors
          position="bottom-right"
          duration={3000}
          toastOptions={{
            style: {
              background: "hsl(var(--background))",
              color: "hsl(var(--foreground))",
              border: "1px solid hsl(var(--border))",
              backdropFilter: "none",
              opacity: 1,
            },
            className: "group",
            classNames: {
              toast: "group-[.toaster]:shadow-lg group-[.toaster]:bg-background",
            },
          }}
        />
      </QueryClientProvider>
    </div>
  );
}
