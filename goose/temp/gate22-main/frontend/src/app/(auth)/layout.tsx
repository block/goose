"use client";

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

export default function AuthLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <QueryClientProvider client={queryClient}>
      {children}
      <ReactQueryDevtools initialIsOpen={false} />
      <Toaster
        closeButton
        toastOptions={{
          classNames: {
            closeButton:
              "text-muted-foreground! hover:text-red-500! h-6! w-6! bg-background! hover:bg-red-50! dark:hover:bg-red-950! rounded! border! border-border! hover:border-red-200! dark:hover:border-red-800! transition-colors! duration-150!",
            error: "bg-background! text-foreground! border-border! shadow-lg!",
            success: "bg-background! text-foreground! border-border! shadow-lg!",
          },
        }}
      />
    </QueryClientProvider>
  );
}
