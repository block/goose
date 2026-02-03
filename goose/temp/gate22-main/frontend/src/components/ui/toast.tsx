"use client";

import { useTheme } from "next-themes";
import { Toaster as Sonner, ToasterProps, toast as sonnerToast } from "sonner";
import {
  CheckCircle2,
  XCircle,
  AlertCircle,
  Info,
  X,
  Loader2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import "./toast.css";

const Toaster = ({ ...props }: ToasterProps) => {
  const { theme = "system" } = useTheme();

  return (
    <Sonner
      theme={theme as ToasterProps["theme"]}
      className="toaster group"
      position="bottom-right"
      toastOptions={{
        unstyled: true,
        classNames: {
          toast: cn(
            "group toast group-[.toaster]:pointer-events-auto",
            "w-full rounded-lg border p-4",
            "group-[.toaster]:bg-background group-[.toaster]:text-foreground",
            "group-[.toaster]:border-border group-[.toaster]:shadow-lg",
            "data-[type=success]:border-green-200 dark:data-[type=success]:border-green-800",
            "data-[type=error]:border-red-200 dark:data-[type=error]:border-red-800",
            "data-[type=warning]:border-yellow-200 dark:data-[type=warning]:border-yellow-800",
            "data-[type=info]:border-blue-200 dark:data-[type=info]:border-blue-800",
          ),
          title: "text-sm font-semibold",
          description: "text-sm opacity-90 mt-1",
          actionButton: cn(
            "inline-flex items-center justify-center rounded-md text-sm font-medium",
            "h-8 px-3",
            "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
            "transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
            "disabled:pointer-events-none disabled:opacity-50",
          ),
          cancelButton: cn(
            "inline-flex items-center justify-center rounded-md text-sm font-medium",
            "h-8 px-3",
            "border border-input bg-background hover:bg-accent hover:text-accent-foreground",
            "transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
            "disabled:pointer-events-none disabled:opacity-50",
          ),
          closeButton: cn(
            "absolute right-2 top-2 rounded-md p-1",
            "text-foreground/50 hover:text-foreground",
            "transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2",
            "disabled:pointer-events-none disabled:opacity-50",
          ),
          error:
            "group-[.toaster]:bg-red-50 dark:group-[.toaster]:bg-red-950/10",
          success:
            "group-[.toaster]:bg-green-50 dark:group-[.toaster]:bg-green-950/10",
          warning:
            "group-[.toaster]:bg-yellow-50 dark:group-[.toaster]:bg-yellow-950/10",
          info: "group-[.toaster]:bg-blue-50 dark:group-[.toaster]:bg-blue-950/10",
          loading: "group-[.toaster]:bg-muted",
        },
      }}
      expand={false}
      richColors
      closeButton
      icons={{
        success: (
          <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400" />
        ),
        error: <XCircle className="h-5 w-5 text-red-600 dark:text-red-400" />,
        warning: (
          <AlertCircle className="h-5 w-5 text-yellow-600 dark:text-yellow-400" />
        ),
        info: <Info className="h-5 w-5 text-blue-600 dark:text-blue-400" />,
        loading: (
          <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
        ),
        close: <X className="h-4 w-4" />,
      }}
      {...props}
    />
  );
};

interface ToastOptions {
  title?: string;
  description?: string;
  duration?: number;
  action?: {
    label: string;
    onClick: () => void;
  };
}

const toast = {
  success: (message: string, options?: ToastOptions) => {
    return sonnerToast.success(options?.title || message, {
      description: options?.description,
      duration: options?.duration || 4000,
      action: options?.action,
    });
  },
  error: (message: string, options?: ToastOptions) => {
    return sonnerToast.error(options?.title || message, {
      description: options?.description,
      duration: options?.duration || 5000,
      action: options?.action,
    });
  },
  warning: (message: string, options?: ToastOptions) => {
    return sonnerToast.warning(options?.title || message, {
      description: options?.description,
      duration: options?.duration || 4500,
      action: options?.action,
    });
  },
  info: (message: string, options?: ToastOptions) => {
    return sonnerToast.info(options?.title || message, {
      description: options?.description,
      duration: options?.duration || 4000,
      action: options?.action,
    });
  },
  loading: (message: string, options?: ToastOptions) => {
    return sonnerToast.loading(options?.title || message, {
      description: options?.description,
      duration: options?.duration || Infinity,
      action: options?.action,
    });
  },
  promise: <T,>(
    promise: Promise<T>,
    options: {
      loading: string;
      success: string | ((data: T) => string);
      error: string | ((error: unknown) => string);
      description?: string;
    },
  ) => {
    return sonnerToast.promise(promise, options);
  },
  dismiss: (id?: string | number) => {
    return sonnerToast.dismiss(id);
  },
};

export { Toaster, toast };
