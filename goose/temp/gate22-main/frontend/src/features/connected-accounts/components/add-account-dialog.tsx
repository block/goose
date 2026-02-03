"use client";

import { useState, useMemo, useEffect } from "react";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm } from "react-hook-form";
import * as z from "zod";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
  FormDescription,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Plus, ExternalLink, Check, ChevronsUpDown, Key, ShieldCheck, Loader2 } from "lucide-react";
import { toast } from "sonner";
import Image from "next/image";
import { cn } from "@/lib/utils";
import {
  useMCPServerConfigurations,
  useMCPServerConfiguration,
} from "@/features/mcp/hooks/use-mcp-servers";
import { useCreateConnectedAccount } from "@/features/connected-accounts/hooks/use-connected-account";
import { MCPServerConfigurationPublicBasic } from "@/features/mcp/types/mcp.types";
import { AuthType, ConnectedAccountOwnership } from "@/features/mcp/types/mcp.types";
import { OAuth2ConnectedAccountResponse } from "@/features/connected-accounts/api/connectedaccount";
import { useRole } from "@/hooks/use-permissions";

const formSchema = z.object({
  mcpServerConfigurationId: z.string().min(1, "MCP Server configuration is required"),
  apiKey: z.string().optional(),
});

type FormValues = z.infer<typeof formSchema>;

interface AddAccountDialogProps {
  onSuccess?: () => void;
}

export function AddAccountDialog({ onSuccess }: AddAccountDialogProps) {
  const [open, setOpen] = useState(false);
  const [comboboxOpen, setComboboxOpen] = useState(false);
  const [selectedConfig, setSelectedConfig] = useState<MCPServerConfigurationPublicBasic | null>(
    null,
  );
  const [authType, setAuthType] = useState<AuthType | null>(null);

  const { data: mcpConfigurationsResponse, isLoading: isLoadingConfigurations } =
    useMCPServerConfigurations({ limit: 100 });

  // Fetch full configuration details when a config is selected
  const { data: fullConfigDetails, isLoading: isLoadingDetails } = useMCPServerConfiguration(
    selectedConfig?.id || "",
  );

  const { mutateAsync: createAccount, isPending: isCreating } = useCreateConnectedAccount();

  const { activeRole } = useRole();
  const isActingAsAdmin = activeRole === "admin";
  const isActingAsMember = activeRole === "member";

  const mcpConfigurations = useMemo(() => {
    const allConfigurations = mcpConfigurationsResponse?.data || [];

    // Filter configurations based on active role (considering "view as" functionality)
    return allConfigurations.filter((config) => {
      // Admin role: Can only create Shared ConnectedAccounts
      if (isActingAsAdmin) {
        return config.connected_account_ownership === ConnectedAccountOwnership.SHARED;
      }
      // Member role: Can only create Individual ConnectedAccounts
      if (isActingAsMember) {
        return config.connected_account_ownership === ConnectedAccountOwnership.INDIVIDUAL;
      }
      // Default: show all if role is not determined
      return true;
    });
  }, [mcpConfigurationsResponse, isActingAsAdmin, isActingAsMember]);

  const form = useForm<FormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      mcpServerConfigurationId: "",
      apiKey: "",
    },
  });

  // Update auth type when full config details are loaded
  useEffect(() => {
    if (fullConfigDetails?.auth_type) {
      setAuthType(fullConfigDetails.auth_type);
      // Clear API key field when switching configurations
      form.setValue("apiKey", "");
    }
  }, [fullConfigDetails, form]);

  const handleSubmit = async (values: FormValues) => {
    try {
      if (!authType) {
        toast.error("Unable to determine authentication type");
        return;
      }

      // Prepare request based on auth type
      let result;

      if (authType === AuthType.OAUTH) {
        // For OAuth, include redirect URL
        const currentOrigin = window.location.origin;
        const redirectUrl = `${currentOrigin}/connected-accounts`;

        result = (await createAccount({
          mcpServerConfigurationId: values.mcpServerConfigurationId,
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
          mcpServerConfigurationId: values.mcpServerConfigurationId,
          apiKey: values.apiKey,
        });

        toast.success("Connected account created successfully");
        setOpen(false);
        form.reset();
        setSelectedConfig(null);
        setAuthType(null);
        onSuccess?.();
      } else if (authType === AuthType.NO_AUTH) {
        // For No Auth, just create the account
        result = await createAccount({
          mcpServerConfigurationId: values.mcpServerConfigurationId,
        });

        toast.success("Connected account created successfully");
        setOpen(false);
        form.reset();
        setSelectedConfig(null);
        setAuthType(null);
        onSuccess?.();
      }
    } catch (error) {
      console.error("Error creating connected account:", error);
      toast.error(error instanceof Error ? error.message : "Failed to create connected account");
    }
  };

  const getSubmitButtonText = () => {
    if (isCreating) return "Processing...";
    if (!authType) return "Select Configuration";

    switch (authType) {
      case AuthType.OAUTH:
        return "Connect Account";
      case AuthType.API_KEY:
        return "Confirm";
      case AuthType.NO_AUTH:
        return "Confirm";
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

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        <Button>
          <Plus className="mr-2 h-4 w-4" />
          Add Account
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Add Connected Account</DialogTitle>
        </DialogHeader>
        <Form {...form}>
          <form onSubmit={form.handleSubmit(handleSubmit)} className="space-y-4">
            {/* Configuration Selection */}
            <FormField
              control={form.control}
              name="mcpServerConfigurationId"
              render={({ field }) => (
                <FormItem className="flex flex-col">
                  <FormLabel>MCP Server Configuration</FormLabel>
                  <Popover open={comboboxOpen} onOpenChange={setComboboxOpen}>
                    <PopoverTrigger asChild>
                      <FormControl>
                        <Button
                          variant="outline"
                          role="combobox"
                          aria-expanded={comboboxOpen}
                          className={cn(
                            "w-full justify-between",
                            !field.value && "text-muted-foreground",
                          )}
                          disabled={isLoadingConfigurations}
                        >
                          {selectedConfig ? (
                            <div className="flex items-center gap-2">
                              {selectedConfig.mcp_server?.logo && (
                                <div className="relative h-4 w-4 shrink-0">
                                  <Image
                                    src={selectedConfig.mcp_server.logo}
                                    alt=""
                                    fill
                                    className="rounded-sm object-contain"
                                  />
                                </div>
                              )}
                              <span>{selectedConfig.name || "Unknown Configuration"}</span>
                            </div>
                          ) : (
                            "Select configuration..."
                          )}
                          <ChevronsUpDown className="ml-2 h-4 w-4 shrink-0 opacity-50" />
                        </Button>
                      </FormControl>
                    </PopoverTrigger>
                    <PopoverContent className="w-full p-0" align="start">
                      <Command>
                        <CommandInput placeholder="Search configurations..." className="h-9" />
                        <CommandList>
                          <CommandEmpty>
                            {isActingAsAdmin
                              ? "No shared configurations found."
                              : isActingAsMember
                                ? "No individual configurations found."
                                : "No configuration found."}
                          </CommandEmpty>
                          <CommandGroup>
                            {mcpConfigurations?.map((config) => (
                              <CommandItem
                                key={config.id}
                                value={config.name || config.id}
                                onSelect={() => {
                                  field.onChange(config.id);
                                  setSelectedConfig(config);
                                  setComboboxOpen(false);
                                }}
                                className="cursor-pointer"
                              >
                                <div className="flex flex-1 items-center gap-2">
                                  {config.mcp_server?.logo && (
                                    <div className="relative h-4 w-4 shrink-0">
                                      <Image
                                        src={config.mcp_server.logo}
                                        alt=""
                                        fill
                                        className="rounded-sm object-contain"
                                      />
                                    </div>
                                  )}
                                  <span>{config.name || "Unknown Configuration"}</span>
                                </div>
                                <Check
                                  className={cn(
                                    "ml-auto h-4 w-4",
                                    field.value === config.id ? "opacity-100" : "opacity-0",
                                  )}
                                />
                              </CommandItem>
                            ))}
                          </CommandGroup>
                        </CommandList>
                      </Command>
                    </PopoverContent>
                  </Popover>
                  <FormDescription>
                    Select the MCP server configuration to link your account with.
                  </FormDescription>
                  <FormMessage />
                </FormItem>
              )}
            />

            {/* Show loading state while fetching config details */}
            {selectedConfig && isLoadingDetails && (
              <div className="flex items-center justify-center py-4">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                <span className="ml-2 text-sm text-muted-foreground">
                  Loading configuration details...
                </span>
              </div>
            )}

            {/* Dynamic Auth UI based on auth type */}
            {authType && !isLoadingDetails && (
              <>
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
                        <FormDescription>
                          Enter the API key for this service. It will be securely stored.
                        </FormDescription>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                )}

                {authType === AuthType.OAUTH && (
                  <div className="rounded-lg border bg-muted/50 p-4">
                    <div className="flex items-start space-x-2">
                      <ExternalLink className="mt-0.5 h-5 w-5 text-muted-foreground" />
                      <div className="space-y-1">
                        <p className="text-sm font-medium">OAuth Authentication</p>
                        <p className="text-sm text-muted-foreground">
                          You&apos;ll be redirected to authenticate with the service provider.
                        </p>
                      </div>
                    </div>
                  </div>
                )}

                {authType === AuthType.NO_AUTH && (
                  <div className="rounded-lg border bg-muted/50 p-4">
                    <div className="flex items-start space-x-2">
                      <ShieldCheck className="mt-0.5 h-5 w-5 text-muted-foreground" />
                      <div className="space-y-1">
                        <p className="text-sm font-medium">No Authentication Required</p>
                        <p className="text-sm text-muted-foreground">
                          This service doesn&apos;t require authentication. Click confirm to
                          connect.
                        </p>
                      </div>
                    </div>
                  </div>
                )}
              </>
            )}

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => {
                  setOpen(false);
                  form.reset();
                  setSelectedConfig(null);
                  setAuthType(null);
                }}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={
                  isCreating ||
                  !form.watch("mcpServerConfigurationId") ||
                  isLoadingDetails ||
                  (authType === AuthType.API_KEY && !form.watch("apiKey"))
                }
              >
                {getSubmitButtonIcon()}
                {getSubmitButtonText()}
              </Button>
            </DialogFooter>
          </form>
        </Form>
      </DialogContent>
    </Dialog>
  );
}
