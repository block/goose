"use client";

import { useState, useEffect } from "react";
import { defineStepper } from "@stepperize/react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Plus, ChevronRight, ChevronLeft, Loader2, Trash2 } from "lucide-react";
import Image from "next/image";
import { CreateMCPServerBundleInput } from "@/features/bundle-mcp/types/bundle-mcp.types";
import {
  MCPServerConfigurationPublicBasic,
  ConnectedAccountOwnership,
} from "@/features/mcp/types/mcp.types";
import { ConnectedAccount } from "@/features/connected-accounts/types/connectedaccount.types";
import { ManageMCPConfigurationDialog } from "@/features/mcp/components/manage-mcp-configuration-dialog";
import { getOwnershipLabel } from "@/utils/configuration-labels";

interface ConfigurationSelection {
  configurationId: string;
}

interface BundleMCPStepperProps {
  isOpen: boolean;
  onClose: () => void;
  availableConfigurations: MCPServerConfigurationPublicBasic[];
  connectedAccounts?: ConnectedAccount[];
  onSubmit: (values: CreateMCPServerBundleInput) => Promise<void>;
  selectedIds?: string[];
  onSelectionChange?: (ids: string[]) => void;
}

// Define the stepper with two steps
const { useStepper, steps } = defineStepper(
  { id: "details", label: "Bundle Details" },
  { id: "configurations", label: "MCP Configurations" },
);

export function BundleMCPStepperForm({
  isOpen,
  onClose,
  availableConfigurations,
  connectedAccounts = [],
  onSubmit,
  selectedIds,
  onSelectionChange,
}: BundleMCPStepperProps) {
  const stepper = useStepper();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [name, setName] = useState("");
  const [nameError, setNameError] = useState("");
  const [description, setDescription] = useState("");
  const [selections, setSelections] = useState<ConfigurationSelection[]>([]);
  const [showConfigSelectionDialog, setShowConfigSelectionDialog] = useState(false);

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen) {
      stepper.reset();
      setName("");
      setNameError("");
      setDescription("");

      // Initialize selections with configuration IDs only
      const initialSelections =
        selectedIds?.map((id) => ({
          configurationId: id,
        })) || [];

      setSelections(initialSelections);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  useEffect(() => {
    if (!onSelectionChange) {
      return;
    }

    onSelectionChange(selections.map((s) => s.configurationId));
  }, [selections, onSelectionChange]);

  // Handle managing configurations (adding/removing)
  const handleManageConfigurations = (additions: string[], removals: string[]) => {
    setSelections((prev) => [
      ...prev.filter((selection) => !removals.includes(selection.configurationId)),
      ...additions.map((id) => ({ configurationId: id })),
    ]);
  };

  // Handle removing a configuration from the bundle
  const handleRemoveConfiguration = (configId: string) => {
    setSelections((prev) => prev.filter((s) => s.configurationId !== configId));
  };

  // Get available accounts for a configuration
  const getAccountsForConfiguration = (configId: string) => {
    return connectedAccounts.filter((account) => account.mcp_server_configuration_id === configId);
  };

  // Check if a shared account configuration has a valid shared account
  const hasValidSharedAccount = (configId: string) => {
    const config = availableConfigurations.find((c) => c.id === configId);
    if (config?.connected_account_ownership === ConnectedAccountOwnership.SHARED) {
      // Check if there's any connected account that is shared for this configuration
      // Shared accounts should have a specific indicator in the connected accounts
      // If no accounts exist for this shared configuration, it means the shared account is not set up
      const sharedAccounts = connectedAccounts.filter(
        (account) => account.mcp_server_configuration_id === configId,
      );
      // If there are no accounts at all for a shared configuration,
      // it means the shared account is not properly set up
      return sharedAccounts.length > 0;
    }
    return false;
  };

  // Step validation
  const isStepValid = (stepId: string) => {
    switch (stepId) {
      case "details":
        return !!name.trim();
      case "configurations":
        return selections.length > 0;
      default:
        return false;
    }
  };

  const canProceed = isStepValid(stepper.current.id);
  const currentStepIndex = steps.findIndex((s) => s.id === stepper.current.id);

  // Handle form submission
  const handleSubmit = async () => {
    // Final validation
    if (!name.trim()) {
      setNameError("Bundle name is required");
      stepper.goTo("details");
      return;
    }

    if (selections.length === 0) {
      stepper.goTo("configurations");
      return;
    }

    setIsSubmitting(true);
    try {
      await onSubmit({
        name: name.trim(),
        description: description.trim() || undefined,
        mcp_server_configuration_ids: selections.map((s) => s.configurationId),
      });
      onClose();
      // Reset form
      setName("");
      setDescription("");
      setSelections([]);
    } catch (error) {
      console.error("Error creating bundle:", error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <>
      <Dialog open={isOpen} onOpenChange={onClose}>
        <DialogContent className="flex max-h-[90vh] flex-col p-0 sm:max-w-3xl">
          <DialogHeader className="flex-shrink-0 border-b px-6 pt-6 pb-4">
            <DialogTitle>Create MCP Bundle</DialogTitle>
            <DialogDescription>
              Create a new MCP server bundle with selected configurations
            </DialogDescription>
          </DialogHeader>

          {/* Main Content */}
          <div className="flex-1 overflow-y-auto px-6 py-4">
            <div className="space-y-6">
              {stepper.current.id === "details" && (
                <div className="space-y-4">
                  <div className="px-1">
                    <h3 className="mb-1 text-sm font-medium">Bundle Information</h3>
                    <p className="text-xs text-muted-foreground">
                      Provide a name and description for your bundle
                    </p>
                  </div>
                  <div className="space-y-3">
                    <div className="space-y-2 px-1">
                      <Label htmlFor="bundle-name">Bundle Name *</Label>
                      <Input
                        id="bundle-name"
                        value={name}
                        onChange={(e) => {
                          setName(e.target.value);
                          if (nameError) setNameError("");
                        }}
                        placeholder="Enter bundle name"
                        className={nameError ? "border-red-500" : ""}
                        required
                      />
                      {nameError && <p className="text-xs text-red-500">{nameError}</p>}
                    </div>
                    <div className="space-y-2 px-1">
                      <Label htmlFor="bundle-description">Description</Label>
                      <Textarea
                        id="bundle-description"
                        value={description}
                        onChange={(e) => setDescription(e.target.value)}
                        placeholder="Enter bundle description (optional)"
                        rows={3}
                        className="resize-none"
                      />
                    </div>
                  </div>
                </div>
              )}

              {stepper.current.id === "configurations" && (
                <div className="space-y-4">
                  <div className="px-1">
                    <h3 className="mb-1 text-sm font-medium">MCP Server Configurations</h3>
                    <p className="text-xs text-muted-foreground">
                      Select configurations to include in your bundle
                    </p>
                  </div>

                  {/* Configuration Selection Table */}
                  <div className="overflow-x-auto rounded-lg border">
                    {selections.length === 0 ? (
                      <div className="p-8 text-center">
                        <p className="mb-4 text-muted-foreground">No configurations added yet</p>
                        <Button
                          type="button"
                          variant="outline"
                          onClick={() => setShowConfigSelectionDialog(true)}
                        >
                          <Plus className="mr-2 h-4 w-4" />
                          Manage MCP Configuration
                        </Button>
                      </div>
                    ) : (
                      <>
                        <Table>
                          <TableHeader>
                            <TableRow>
                              <TableHead className="w-[300px]">Configuration</TableHead>
                              <TableHead className="w-[200px]">Connected Account</TableHead>
                              <TableHead className="w-[80px]">Actions</TableHead>
                            </TableRow>
                          </TableHeader>
                          <TableBody>
                            {selections.map((selection) => {
                              const config = availableConfigurations.find(
                                (c) => c.id === selection.configurationId,
                              );

                              if (!config) return null;

                              const accounts = getAccountsForConfiguration(config.id);
                              const isShared =
                                config.connected_account_ownership ===
                                ConnectedAccountOwnership.SHARED;

                              return (
                                <TableRow key={config.id}>
                                  <TableCell>
                                    <div className="flex items-center space-x-3">
                                      <div className="flex-1 space-y-1">
                                        <div className="flex items-center gap-2">
                                          {config.mcp_server?.logo && (
                                            <div className="relative h-5 w-5 shrink-0">
                                              <Image
                                                src={config.mcp_server.logo}
                                                alt=""
                                                fill
                                                className="rounded-sm object-contain"
                                              />
                                            </div>
                                          )}
                                          <span className="font-medium">{config.name}</span>
                                        </div>
                                        {config.connected_account_ownership && (
                                          <div className="text-sm text-muted-foreground">
                                            Type:{" "}
                                            {getOwnershipLabel(config.connected_account_ownership)}
                                          </div>
                                        )}
                                      </div>
                                    </div>
                                  </TableCell>
                                  <TableCell>
                                    {isShared ? (
                                      <Badge variant="secondary">Shared Account</Badge>
                                    ) : (
                                      <div className="text-sm">
                                        {accounts.length > 0 ? (
                                          <span>
                                            {accounts[0].user?.email ||
                                              accounts[0].user?.name ||
                                              "Connected"}
                                          </span>
                                        ) : (
                                          <span className="text-muted-foreground">
                                            No account connected
                                          </span>
                                        )}
                                      </div>
                                    )}
                                  </TableCell>
                                  <TableCell>
                                    <Button
                                      type="button"
                                      size="sm"
                                      variant="ghost"
                                      onClick={() => handleRemoveConfiguration(config.id)}
                                    >
                                      <Trash2 className="h-4 w-4 text-destructive" />
                                    </Button>
                                  </TableCell>
                                </TableRow>
                              );
                            })}
                          </TableBody>
                        </Table>

                        {/* Add More Configuration Button */}
                        <div className="border-t bg-muted/50 p-3">
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => setShowConfigSelectionDialog(true)}
                            className="w-full"
                          >
                            <Plus className="mr-2 h-4 w-4" />
                            Manage MCP Configuration
                          </Button>
                        </div>
                      </>
                    )}
                  </div>

                  {/* Selection Summary */}
                  {selections.length > 0 && (
                    <div className="px-1 text-sm text-muted-foreground">
                      {selections.length} configuration
                      {selections.length !== 1 ? "s" : ""} selected
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>

          {/* Footer with navigation */}
          <div className="flex-shrink-0 border-t px-6 py-3">
            <div className="flex items-center justify-between">
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  if (stepper.isFirst) {
                    onClose();
                  } else {
                    stepper.prev();
                  }
                }}
                disabled={isSubmitting}
              >
                <ChevronLeft className="mr-1 h-4 w-4" />
                {stepper.isFirst ? "Cancel" : "Back"}
              </Button>

              <div className="flex items-center gap-3">
                <span className="text-xs text-muted-foreground">
                  Step {currentStepIndex + 1} of {steps.length}
                </span>
                {!stepper.isLast ? (
                  <Button size="sm" onClick={() => stepper.next()} disabled={!canProceed}>
                    Next
                    <ChevronRight className="ml-1 h-4 w-4" />
                  </Button>
                ) : (
                  <Button size="sm" onClick={handleSubmit} disabled={isSubmitting || !canProceed}>
                    {isSubmitting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                    Create Bundle
                  </Button>
                )}
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Configuration Management Dialog */}
      <ManageMCPConfigurationDialog
        isOpen={showConfigSelectionDialog}
        onClose={() => setShowConfigSelectionDialog(false)}
        availableConfigurations={availableConfigurations}
        connectedAccounts={connectedAccounts}
        alreadySelectedIds={selections.map((s) => s.configurationId)}
        onConfirm={handleManageConfigurations}
        hasValidSharedAccount={hasValidSharedAccount}
      />
    </>
  );
}
