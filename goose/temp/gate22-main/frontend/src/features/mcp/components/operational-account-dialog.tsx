"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { ExternalLink, Key, ShieldCheck, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { AuthType } from "../types/mcp.types";
import { useCreateConnectedAccount } from "@/features/connected-accounts/hooks/use-connected-account";
import { OAuth2ConnectedAccountResponse } from "@/features/connected-accounts/api/connectedaccount";

const formSchema = z.object({
  apiKey: z.string().optional(),
});

type FormValues = z.infer<typeof formSchema>;

interface OperationalAccountDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  server: {
    id: string;
    name: string;
    auth_type?: AuthType;
  };
  operationalConfigId?: string;
  onSuccess?: () => void;
}

export function OperationalAccountDialog({
  open,
  onOpenChange,
  server,
  operationalConfigId,
  onSuccess,
}: OperationalAccountDialogProps) {
  const { mutateAsync: createAccount, isPending: isCreating } = useCreateConnectedAccount();

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      apiKey: "",
    },
  });

  // Get the auth type from server's supported auth types (assuming first one for now)
  const authType = server.auth_type;

  const handleSubmit = async (values: FormValues) => {
    try {
      if (!authType) {
        toast.error("Unable to determine authentication type");
        return;
      }

      if (!operationalConfigId) {
        toast.error("No operational configuration found for this server");
        return;
      }

      // Prepare request based on auth type
      let result;

      if (authType === AuthType.OAUTH) {
        // For OAuth, include redirect URL
        const currentOrigin = window.location.origin;
        const redirectUrl = `${currentOrigin}/mcp-servers/${server.id}`;

        result = (await createAccount({
          mcpServerConfigurationId: operationalConfigId,
          redirectUrl: redirectUrl,
        })) as OAuth2ConnectedAccountResponse;

        if (result.authorization_url) {
          // Redirect to OAuth provider
          window.location.href = result.authorization_url;
        }
      } else if (authType === AuthType.API_KEY) {
        // For API Key, validate and include the key
        if (!values.apiKey || values.apiKey.trim() === "") {
          toast.error("API key is required");
          return;
        }

        result = await createAccount({
          mcpServerConfigurationId: operationalConfigId,
          apiKey: values.apiKey,
        });

        toast.success("Operational account configured successfully");
        onOpenChange(false);
        form.reset();
        onSuccess?.();
      } else if (authType === AuthType.NO_AUTH) {
        // For No Auth, just create the account
        result = await createAccount({
          mcpServerConfigurationId: operationalConfigId,
        });

        toast.success("Operational account configured successfully");
        onOpenChange(false);
        form.reset();
        onSuccess?.();
      }
    } catch (error) {
      console.error("Error configuring operational account:", error);
      toast.error(
        error instanceof Error ? error.message : "Failed to configure operational account",
      );
    }
  };

  const getSubmitButtonText = () => {
    if (isCreating) return "Processing...";
    if (!authType) return "Unknown Auth Type";

    switch (authType) {
      case AuthType.OAUTH:
        return "Connect Account";
      case AuthType.API_KEY:
        return "Save API Key";
      case AuthType.NO_AUTH:
        return "Configure";
      default:
        return "Submit";
    }
  };

  const getSubmitButtonIcon = () => {
    if (isCreating) return <Loader2 className="mr-2 h-4 w-4 animate-spin" />;
    if (!authType) return null;

    switch (authType) {
      case AuthType.OAUTH:
        return <ExternalLink className="mr-2 h-4 w-4" />;
      case AuthType.API_KEY:
        return <Key className="mr-2 h-4 w-4" />;
      case AuthType.NO_AUTH:
        return <ShieldCheck className="mr-2 h-4 w-4" />;
      default:
        return null;
    }
  };

  const getAuthMethodLabel = () => {
    switch (authType) {
      case AuthType.OAUTH:
        return "OAuth 2.0";
      case AuthType.API_KEY:
        return "API Key";
      case AuthType.NO_AUTH:
        return "No Authentication";
      default:
        return "Unknown";
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Setup Operational Account</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* Server Info */}
          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              Setting up operational account for <strong>{server.name}</strong>
            </p>
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium">Operational Account Auth Method:</span>
              <span className="text-sm text-muted-foreground">{getAuthMethodLabel()}</span>
            </div>
          </div>

          <Form {...form}>
            <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
              {/* API Key Input (only for API_KEY auth type) */}
              {authType === AuthType.API_KEY && (
                <FormField
                  control={form.control}
                  name="apiKey"
                  render={({ field }) => (
                    <FormItem>
                      <FormLabel>API Key</FormLabel>
                      <FormControl>
                        <Input type="password" placeholder="Enter your API key" {...field} />
                      </FormControl>
                      <FormMessage />
                    </FormItem>
                  )}
                />
              )}

              {/* OAuth Info */}
              {authType === AuthType.OAUTH && (
                <div className="rounded-lg bg-muted/50 p-4">
                  <p className="text-sm text-muted-foreground">
                    You will be redirected to the authentication provider to authorize access.
                  </p>
                </div>
              )}

              {/* No Auth Info */}
              {authType === AuthType.NO_AUTH && (
                <div className="rounded-lg bg-muted/50 p-4">
                  <p className="text-sm text-muted-foreground">
                    This server requires no authentication. Click configure to set up the
                    operational account.
                  </p>
                </div>
              )}

              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                  disabled={isCreating}
                >
                  Cancel
                </Button>
                <Button type="submit" disabled={isCreating || !authType}>
                  {getSubmitButtonIcon()}
                  {getSubmitButtonText()}
                </Button>
              </DialogFooter>
            </form>
          </Form>
        </div>
      </DialogContent>
    </Dialog>
  );
}
