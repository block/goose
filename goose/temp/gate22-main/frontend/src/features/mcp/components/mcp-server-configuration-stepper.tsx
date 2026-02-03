"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { defineStepper } from "@stepperize/react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Checkbox } from "@/components/ui/checkbox";
import { Switch } from "@/components/ui/switch";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Loader2, AlertCircle, ChevronRight, ChevronLeft, Search, HelpCircle } from "lucide-react";
import { MultiSelect } from "@/components/ui/multi-select";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import {
  MCPServerPublic,
  MCPServerConfigurationCreate,
  AuthType,
  ConnectedAccountOwnership,
} from "../types/mcp.types";
import { useCreateMCPServerConfiguration } from "../hooks/use-mcp-servers";
import { toast } from "sonner";
import { listTeams } from "@/features/teams/api/team";
import { useMetaInfo } from "@/components/context/metainfo";
import { Team } from "@/features/teams/types/team.types";
import { CreateTeamWithMembersDialog } from "@/features/teams/components/create-team-with-members-dialog";
import { Plus } from "lucide-react";
import {
  getAuthTypeLabel,
  getAuthTypeDescription,
  getAuthTypeDetailedInfo,
} from "@/utils/auth-labels";
import { getConfigurationTypeDetailedInfo } from "@/utils/configuration-labels";

interface MCPServerConfigurationStepperProps {
  isOpen: boolean;
  onClose: () => void;
  server: MCPServerPublic;
}

// Define the stepper outside the component
const { useStepper, steps } = defineStepper(
  { id: "general", label: "General" },
  { id: "authentication", label: "Authentication" },
  { id: "type", label: "Type" },
  { id: "tools", label: "Tools" },
  { id: "teams", label: "Teams" },
);

export function MCPServerConfigurationStepper({
  isOpen,
  onClose,
  server,
}: MCPServerConfigurationStepperProps) {
  const router = useRouter();
  const stepper = useStepper();
  const [name, setName] = useState<string>("");
  const [nameError, setNameError] = useState<string>("");
  const [description, setDescription] = useState<string>("");
  const [selectedAuthType, setSelectedAuthType] = useState<AuthType>(
    server?.supported_auth_types?.[0] || AuthType.NO_AUTH,
  );
  const [configurationType, setConfigurationType] = useState<ConnectedAccountOwnership>(
    ConnectedAccountOwnership.INDIVIDUAL,
  );
  const [allToolsEnabled, setAllToolsEnabled] = useState(true);
  const [selectedTools, setSelectedTools] = useState<Set<string>>(new Set());
  const [toolSearchQuery, setToolSearchQuery] = useState("");
  const [selectedTeams, setSelectedTeams] = useState<Set<string>>(new Set());
  const [teams, setTeams] = useState<Team[]>([]);
  const [teamsLoading, setTeamsLoading] = useState(false);
  const [createTeamDialogOpen, setCreateTeamDialogOpen] = useState(false);

  const { accessToken, activeOrg } = useMetaInfo();
  const createConfiguration = useCreateMCPServerConfiguration();

  // Reset state when dialog opens
  useEffect(() => {
    if (isOpen) {
      stepper.reset();
      setName("");
      setNameError("");
      setDescription("");
      setConfigurationType(ConnectedAccountOwnership.INDIVIDUAL);
      setSelectedTools(new Set());
      setToolSearchQuery("");
      setSelectedTeams(new Set());
      setAllToolsEnabled(true);
      setSelectedAuthType(server?.supported_auth_types?.[0] || AuthType.NO_AUTH);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  const loadTeams = async () => {
    setTeamsLoading(true);
    try {
      const fetchedTeams = await listTeams(accessToken, activeOrg.orgId);
      setTeams(fetchedTeams);
    } catch {
      toast.error("Failed to load teams");
    } finally {
      setTeamsLoading(false);
    }
  };

  useEffect(() => {
    if (isOpen && stepper.current.id === "teams") {
      loadTeams();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, stepper.current.id, accessToken, activeOrg.orgId]);

  const handleTeamCreated = (newTeam: Team) => {
    // Refresh teams list
    loadTeams();
    // Auto-select the newly created team
    setSelectedTeams((prev) => new Set([...prev, newTeam.team_id]));
    // Close the dialog
    setCreateTeamDialogOpen(false);
  };

  const handleToolToggle = (toolId: string) => {
    const newSelectedTools = new Set(selectedTools);
    if (newSelectedTools.has(toolId)) {
      newSelectedTools.delete(toolId);
    } else {
      newSelectedTools.add(toolId);
    }
    setSelectedTools(newSelectedTools);
  };

  const handleSubmit = async () => {
    // Final validation
    if (!name.trim()) {
      setNameError("Configuration name is required");
      stepper.goTo("general"); // Go to general step
      return;
    }

    const configurationData: MCPServerConfigurationCreate = {
      mcp_server_id: server.id,
      name: name.trim(),
      description: description.trim() || undefined,
      auth_type: selectedAuthType,
      connected_account_ownership: configurationType,
      all_tools_enabled: allToolsEnabled,
      enabled_tools: allToolsEnabled ? [] : Array.from(selectedTools),
      allowed_teams: Array.from(selectedTeams),
    };

    try {
      const result = await createConfiguration.mutateAsync(configurationData);
      toast.success("MCP server configured successfully");
      onClose();
      // Navigate to the configuration detail page
      router.push(`/mcp-configuration/${result.id}`);
    } catch (error: unknown) {
      const errorMessage =
        error instanceof Error ? error.message : "Failed to configure MCP server";
      toast.error(errorMessage);
    }
  };

  const isStepValid = (stepId: string) => {
    switch (stepId) {
      case "general":
        // Only name is required in general step
        return !!name.trim();
      case "authentication":
        // Authentication must be selected if available
        return (
          !server?.supported_auth_types ||
          server.supported_auth_types.length === 0 ||
          selectedAuthType !== undefined
        );
      case "type":
        // Configuration type is always valid (has default value)
        return !!configurationType;
      case "tools":
        return allToolsEnabled || selectedTools.size > 0;
      case "teams":
        return true; // Team selection is optional
      default:
        return false;
    }
  };

  const canProceed = isStepValid(stepper.current.id);
  const currentStepIndex = steps.findIndex((s) => s.id === stepper.current.id);

  return (
    <>
      <Dialog open={isOpen} onOpenChange={onClose}>
        <DialogContent className="flex h-[80vh] max-w-[95vw] flex-col p-0 sm:max-w-[500px]">
          <DialogHeader className="flex-shrink-0 border-b px-6 pt-6 pb-4">
            <DialogTitle>Configure {server?.name}</DialogTitle>
            <DialogDescription>
              Set up authentication and access permissions for this MCP server
            </DialogDescription>
          </DialogHeader>

          {/* Main Content */}
          <div className="min-h-0 flex-1 overflow-hidden">
            <ScrollArea className="h-full w-full">
              <div className="space-y-6 px-6 py-4">
                {stepper.current.id === "general" && (
                  <div className="space-y-4">
                    <div className="px-1">
                      <h3 className="mb-1 text-sm font-medium">General Information</h3>
                      <p className="text-xs text-muted-foreground">
                        Provide a name and description for this configuration
                      </p>
                    </div>
                    <div className="space-y-3">
                      <div className="space-y-2 px-1">
                        <Label htmlFor="config-name">Configuration Name *</Label>
                        <Input
                          id="config-name"
                          value={name}
                          onChange={(e) => {
                            setName(e.target.value);
                            if (nameError) setNameError("");
                          }}
                          placeholder="Enter a name for this configuration"
                          className={nameError ? "border-red-500" : ""}
                          maxLength={100}
                          required
                        />
                        {nameError && <p className="text-xs text-red-500">{nameError}</p>}
                      </div>
                      <div className="space-y-2 px-1">
                        <Label htmlFor="config-description">Description</Label>
                        <Textarea
                          id="config-description"
                          value={description}
                          onChange={(e) => setDescription(e.target.value)}
                          placeholder="Optional description for this configuration"
                          rows={3}
                          maxLength={500}
                        />
                      </div>
                    </div>
                  </div>
                )}

                {stepper.current.id === "authentication" && (
                  <div className="space-y-4">
                    <div className="px-1">
                      <h3 className="mb-1 text-sm font-medium">Authentication</h3>
                      <p className="text-xs text-muted-foreground">
                        Choose how users will authenticate with this server
                      </p>
                    </div>

                    {!server?.supported_auth_types || server.supported_auth_types.length === 0 ? (
                      <div className="px-1">
                        <Alert>
                          <AlertCircle className="h-4 w-4" />
                          <AlertDescription className="text-xs">
                            No authentication methods available for this server.
                          </AlertDescription>
                        </Alert>
                      </div>
                    ) : (
                      <RadioGroup
                        value={selectedAuthType}
                        onValueChange={(value) => setSelectedAuthType(value as AuthType)}
                        className="space-y-1.5 px-1"
                      >
                        {server.supported_auth_types.map((authType) => (
                          <label
                            key={authType}
                            htmlFor={`auth-${authType}`}
                            className="flex cursor-pointer items-center space-x-2 rounded border p-2 transition-colors hover:bg-accent/50"
                          >
                            <RadioGroupItem value={authType} id={`auth-${authType}`} />
                            <div className="flex-1">
                              <div className="flex items-center gap-2">
                                <span className="text-sm font-medium">
                                  {getAuthTypeLabel(authType)}
                                </span>
                                <Tooltip>
                                  <TooltipTrigger asChild>
                                    <button
                                      type="button"
                                      className="inline-flex items-center justify-center rounded-sm hover:opacity-80 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
                                      aria-label={`More information about ${getAuthTypeLabel(authType)} authentication`}
                                    >
                                      <HelpCircle className="h-3.5 w-3.5 text-muted-foreground transition-colors hover:text-foreground" />
                                    </button>
                                  </TooltipTrigger>
                                  <TooltipContent className="max-w-xs">
                                    <p>{getAuthTypeDetailedInfo(authType)}</p>
                                  </TooltipContent>
                                </Tooltip>
                              </div>
                              <p className="text-xs text-muted-foreground">
                                {getAuthTypeDescription(authType)}
                              </p>
                            </div>
                          </label>
                        ))}
                      </RadioGroup>
                    )}
                  </div>
                )}

                {stepper.current.id === "type" && (
                  <div className="space-y-4">
                    <div className="px-1">
                      <h3 className="mb-1 text-sm font-medium">Connected Account Type</h3>
                      <p className="text-xs text-muted-foreground">
                        Choose how this configuration will be used in your organization
                      </p>
                    </div>

                    <RadioGroup
                      value={configurationType}
                      onValueChange={(value) =>
                        setConfigurationType(value as ConnectedAccountOwnership)
                      }
                      className="space-y-1.5 px-1"
                    >
                      <label
                        htmlFor="stepper-individual"
                        className="flex cursor-pointer items-center space-x-2 rounded border p-2 transition-colors hover:bg-accent/50"
                      >
                        <RadioGroupItem
                          value={ConnectedAccountOwnership.INDIVIDUAL}
                          id="stepper-individual"
                        />
                        <div className="flex-1">
                          <div className="text-sm font-medium">Individual</div>
                          <p className="text-xs text-muted-foreground">
                            {getConfigurationTypeDetailedInfo(ConnectedAccountOwnership.INDIVIDUAL)}
                          </p>
                        </div>
                      </label>
                      <label
                        htmlFor="stepper-shared"
                        className="flex cursor-pointer items-center space-x-2 rounded border p-2 transition-colors hover:bg-accent/50"
                      >
                        <RadioGroupItem
                          value={ConnectedAccountOwnership.SHARED}
                          id="stepper-shared"
                        />
                        <div className="flex-1">
                          <div className="text-sm font-medium">Shared</div>
                          <p className="text-xs text-muted-foreground">
                            {getConfigurationTypeDetailedInfo(ConnectedAccountOwnership.SHARED)}
                          </p>
                        </div>
                      </label>
                    </RadioGroup>
                  </div>
                )}

                {stepper.current.id === "tools" && (
                  <div className="space-y-4">
                    <div className="px-1">
                      <h3 className="mb-1 text-sm font-medium">Select Tools</h3>
                      <p className="text-xs text-muted-foreground">
                        Choose which tools should be available
                      </p>
                    </div>

                    <div className="px-1">
                      <div className="flex items-center justify-between rounded-lg border p-3">
                        <div className="flex-1 space-y-0.5">
                          <Label htmlFor="all-tools" className="text-sm font-medium">
                            Enable all tools
                          </Label>
                          <p className="text-xs text-muted-foreground">
                            Grant access to all {server?.tools?.length || 0} available tools
                          </p>
                        </div>
                        <Switch
                          id="all-tools"
                          checked={allToolsEnabled}
                          onCheckedChange={setAllToolsEnabled}
                        />
                      </div>
                    </div>

                    {!allToolsEnabled && server?.tools && server.tools.length > 0 && (
                      <div className="space-y-3 px-1">
                        {/* Search Input */}
                        <div className="relative">
                          <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                          <Input
                            placeholder="Search tools..."
                            value={toolSearchQuery}
                            onChange={(e) => setToolSearchQuery(e.target.value)}
                            className="pl-9"
                          />
                        </div>

                        <div className="space-y-2">
                          <Label className="text-xs font-medium text-muted-foreground">
                            Select specific tools:
                          </Label>
                          <div className="grid max-h-[400px] gap-1.5 overflow-y-auto rounded-lg border p-2">
                            {(() => {
                              const filteredTools = server.tools.filter(
                                (tool) =>
                                  tool.name.toLowerCase().includes(toolSearchQuery.toLowerCase()) ||
                                  tool.description
                                    ?.toLowerCase()
                                    .includes(toolSearchQuery.toLowerCase()),
                              );

                              if (filteredTools.length === 0) {
                                return (
                                  <p className="py-8 text-center text-sm text-muted-foreground">
                                    {toolSearchQuery
                                      ? "No tools match your search"
                                      : "No tools available"}
                                  </p>
                                );
                              }

                              return filteredTools.map((tool) => (
                                <label
                                  key={tool.id}
                                  className="flex cursor-pointer items-start space-x-2 rounded border p-2 transition-colors hover:bg-accent/50"
                                >
                                  <Checkbox
                                    id={tool.id}
                                    checked={selectedTools.has(tool.id)}
                                    onCheckedChange={() => handleToolToggle(tool.id)}
                                    className="mt-0.5"
                                  />
                                  <div className="min-w-0 flex-1">
                                    <div className="text-sm font-medium">{tool.name}</div>
                                    {tool.description && (
                                      <p className="line-clamp-1 text-xs text-muted-foreground">
                                        {tool.description}
                                      </p>
                                    )}
                                  </div>
                                </label>
                              ));
                            })()}
                          </div>
                        </div>
                      </div>
                    )}

                    {!allToolsEnabled && selectedTools.size === 0 && (
                      <div className="px-1">
                        <Alert variant="destructive">
                          <AlertCircle className="h-4 w-4" />
                          <AlertDescription className="text-xs">
                            Please select at least one tool or enable all tools
                          </AlertDescription>
                        </Alert>
                      </div>
                    )}
                  </div>
                )}

                {stepper.current.id === "teams" && (
                  <div className="space-y-4">
                    <div className="flex items-start justify-between px-1">
                      <div>
                        <h3 className="mb-1 text-sm font-medium">Assign Teams</h3>
                        <p className="text-xs text-muted-foreground">
                          Select which teams can access this server
                        </p>
                      </div>
                      <Button
                        type="button"
                        size="sm"
                        variant="outline"
                        onClick={() => setCreateTeamDialogOpen(true)}
                        className="gap-1"
                      >
                        <Plus className="h-3 w-3" />
                        Create Team
                      </Button>
                    </div>

                    {teamsLoading ? (
                      <div className="flex items-center justify-center py-8">
                        <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                        <span className="text-sm text-muted-foreground">Loading teams...</span>
                      </div>
                    ) : teams.length === 0 ? (
                      <div className="px-1">
                        <Alert>
                          <AlertCircle className="h-4 w-4" />
                          <AlertDescription className="text-xs">
                            No teams found. Create teams in your organization settings first.
                          </AlertDescription>
                        </Alert>
                      </div>
                    ) : (
                      <div className="space-y-3 px-1">
                        <MultiSelect
                          options={teams.map((team) => ({
                            value: team.team_id,
                            label: `${team.name}${team.member_count !== undefined ? ` (${team.member_count} member${team.member_count !== 1 ? "s" : ""})` : ""}`,
                          }))}
                          selected={Array.from(selectedTeams)}
                          onChange={(selected) => setSelectedTeams(new Set(selected))}
                          placeholder="Select teams..."
                          searchPlaceholder="Search teams..."
                          emptyText="No teams found."
                          className="w-full"
                        />
                        {selectedTeams.size > 0 && (
                          <p className="text-xs text-muted-foreground">
                            {selectedTeams.size} team
                            {selectedTeams.size !== 1 ? "s" : ""} selected
                          </p>
                        )}
                      </div>
                    )}
                  </div>
                )}
              </div>
            </ScrollArea>
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
                  <Button
                    size="sm"
                    onClick={handleSubmit}
                    disabled={createConfiguration.isPending || !canProceed}
                  >
                    {createConfiguration.isPending && (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    )}
                    Complete
                  </Button>
                )}
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* Create Team Dialog */}
      <CreateTeamWithMembersDialog
        open={createTeamDialogOpen}
        onOpenChange={setCreateTeamDialogOpen}
        onSuccess={handleTeamCreated}
      />
    </>
  );
}
