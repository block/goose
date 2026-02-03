"use client";

import { useState, useEffect } from "react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Checkbox } from "@/components/ui/checkbox";
import { ScrollArea } from "@/components/ui/scroll-area";
import { toast } from "sonner";
import { Plus, Loader2, HelpCircle, ChevronRight, ChevronLeft, Check } from "lucide-react";
import {
  useMCPServers,
  useMCPServer,
  useCreateMCPServerConfiguration,
} from "../hooks/use-mcp-servers";
import { useTeams } from "@/features/teams/hooks/use-teams";
import {
  AuthType,
  MCPServerConfigurationCreate,
  ConnectedAccountOwnership,
} from "../types/mcp.types";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { getConfigurationTypeDetailedInfo } from "@/utils/configuration-labels";
import { getAuthTypeLabel, getAuthTypeDescription } from "@/utils/auth-labels";
import { cn } from "@/lib/utils";

enum ConfigurationStep {
  BASIC_INFO = 1,
  AUTHENTICATION = 2,
  CONFIGURATION_TYPE = 3,
  TOOLS = 4,
  TEAMS = 5,
}

interface StepIndicatorProps {
  currentStep: ConfigurationStep;
  onClick?: (step: ConfigurationStep) => void;
  completedSteps: Set<ConfigurationStep>;
}

function StepIndicator({ currentStep, onClick, completedSteps }: StepIndicatorProps) {
  const steps = [
    { id: ConfigurationStep.BASIC_INFO, label: "Basic Info" },
    { id: ConfigurationStep.AUTHENTICATION, label: "Authentication" },
    { id: ConfigurationStep.CONFIGURATION_TYPE, label: "Type" },
    { id: ConfigurationStep.TOOLS, label: "Tools" },
    { id: ConfigurationStep.TEAMS, label: "Teams" },
  ];

  return (
    <div className="mb-6 flex items-center justify-between">
      {steps.map((step, index) => (
        <div key={step.id} className="flex flex-1 items-center">
          <button
            onClick={() => onClick?.(step.id)}
            disabled={!onClick || (!completedSteps.has(step.id) && step.id > currentStep)}
            className={cn(
              "flex h-8 w-8 items-center justify-center rounded-full border-2 transition-colors",
              "disabled:cursor-not-allowed",
              currentStep === step.id
                ? "border-primary bg-primary text-primary-foreground"
                : completedSteps.has(step.id)
                  ? "border-primary bg-primary/10 text-primary"
                  : "border-muted-foreground/30 bg-background text-muted-foreground",
            )}
          >
            {completedSteps.has(step.id) && currentStep !== step.id ? (
              <Check className="h-4 w-4" />
            ) : (
              <span className="text-sm font-medium">{step.id}</span>
            )}
          </button>
          <div className="ml-2 flex-1">
            <p
              className={cn(
                "hidden text-xs font-medium sm:block",
                currentStep === step.id
                  ? "text-foreground"
                  : completedSteps.has(step.id)
                    ? "text-primary"
                    : "text-muted-foreground",
              )}
            >
              {step.label}
            </p>
          </div>
          {index < steps.length - 1 && (
            <div
              className={cn(
                "mx-2 h-0.5 flex-1",
                completedSteps.has(step.id) ? "bg-primary" : "bg-muted-foreground/30",
              )}
            />
          )}
        </div>
      ))}
    </div>
  );
}

export function CreateMCPConfigurationDialog() {
  const [open, setOpen] = useState(false);
  const [currentStep, setCurrentStep] = useState<ConfigurationStep>(ConfigurationStep.BASIC_INFO);
  const [completedSteps, setCompletedSteps] = useState<Set<ConfigurationStep>>(new Set());

  // Form state
  const [name, setName] = useState<string>("");
  const [nameError, setNameError] = useState<string>("");
  const [description, setDescription] = useState<string>("");
  const [selectedServerId, setSelectedServerId] = useState<string>("");
  const [selectedAuthType, setSelectedAuthType] = useState<AuthType | "">("");
  const [configurationType, setConfigurationType] = useState<ConnectedAccountOwnership>(
    ConnectedAccountOwnership.INDIVIDUAL,
  );
  const [allToolsEnabled, setAllToolsEnabled] = useState(true);
  const [selectedTools, setSelectedTools] = useState<string[]>([]);
  const [selectedTeams, setSelectedTeams] = useState<string[]>([]);

  const { data: serversResponse } = useMCPServers({ limit: 100 });
  const { data: selectedServer } = useMCPServer(selectedServerId);
  const { data: teams } = useTeams();
  const createConfiguration = useCreateMCPServerConfiguration();

  // Validation for each step
  const validateStep = (step: ConfigurationStep): boolean => {
    switch (step) {
      case ConfigurationStep.BASIC_INFO:
        return name.trim() !== "" && selectedServerId !== "";
      case ConfigurationStep.AUTHENTICATION:
        return selectedAuthType !== "";
      case ConfigurationStep.CONFIGURATION_TYPE:
        return true; // Always valid as we have a default value
      case ConfigurationStep.TOOLS:
        return true; // Optional settings
      case ConfigurationStep.TEAMS:
        return true; // Optional settings
      default:
        return false;
    }
  };

  const handleNextStep = () => {
    if (validateStep(currentStep)) {
      setCompletedSteps((prev) => new Set([...prev, currentStep]));
      if (currentStep < ConfigurationStep.TEAMS) {
        setCurrentStep(currentStep + 1);
      }
    } else {
      if (currentStep === ConfigurationStep.BASIC_INFO && !name.trim()) {
        setNameError("Configuration name is required");
      }
      if (currentStep === ConfigurationStep.BASIC_INFO && !selectedServerId) {
        toast.error("Please select an MCP server");
      }
      if (currentStep === ConfigurationStep.AUTHENTICATION && !selectedAuthType) {
        toast.error("Please select an authentication type");
      }
    }
  };

  const handlePreviousStep = () => {
    if (currentStep > ConfigurationStep.BASIC_INFO) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleStepClick = (step: ConfigurationStep) => {
    if (completedSteps.has(step) || step <= currentStep) {
      setCurrentStep(step);
    }
  };

  const handleSubmit = async () => {
    // Validate all required steps regardless of current step
    if (!validateStep(ConfigurationStep.BASIC_INFO)) {
      toast.error("Please complete Basic Info step");
      return;
    }

    if (!validateStep(ConfigurationStep.AUTHENTICATION)) {
      toast.error("Please complete Authentication step");
      return;
    }

    const configData: MCPServerConfigurationCreate = {
      mcp_server_id: selectedServerId,
      name: name.trim(),
      description: description.trim() || undefined,
      auth_type: selectedAuthType as AuthType,
      connected_account_ownership: configurationType,
      all_tools_enabled: allToolsEnabled,
      enabled_tools: allToolsEnabled ? [] : selectedTools,
      allowed_teams: selectedTeams,
    };

    try {
      await createConfiguration.mutateAsync(configData);
      toast.success("MCP configuration created successfully");
      setOpen(false);
      resetForm();
    } catch (error) {
      console.error("Failed to create configuration:", error);
      toast.error("Failed to create MCP configuration");
    }
  };

  const resetForm = () => {
    setCurrentStep(ConfigurationStep.BASIC_INFO);
    setCompletedSteps(new Set());
    setName("");
    setNameError("");
    setDescription("");
    setSelectedServerId("");
    setSelectedAuthType("");
    setConfigurationType(ConnectedAccountOwnership.INDIVIDUAL);
    setAllToolsEnabled(true);
    setSelectedTools([]);
    setSelectedTeams([]);
  };

  const handleToolToggle = (toolId: string) => {
    setSelectedTools((prev) =>
      prev.includes(toolId) ? prev.filter((id) => id !== toolId) : [...prev, toolId],
    );
  };

  const handleTeamToggle = (teamId: string) => {
    setSelectedTeams((prev) =>
      prev.includes(teamId) ? prev.filter((id) => id !== teamId) : [...prev, teamId],
    );
  };

  // When server changes, reset auth type if it's not supported
  useEffect(() => {
    if (
      selectedServer &&
      selectedAuthType &&
      !selectedServer.supported_auth_types.includes(selectedAuthType as AuthType)
    ) {
      setSelectedAuthType("");
    }
  }, [selectedServer, selectedAuthType]);

  // Set default auth type when server is selected
  useEffect(() => {
    if (selectedServer && !selectedAuthType && selectedServer.supported_auth_types.length > 0) {
      setSelectedAuthType(selectedServer.supported_auth_types[0]);
    }
  }, [selectedServer, selectedAuthType]);

  // Reconcile tool selection when server changes
  useEffect(() => {
    if (selectedServer) {
      // Filter out tools that don't exist on the new server
      setSelectedTools((prev) =>
        prev.filter((id) => (selectedServer.tools ?? []).some((t) => t.id === id)),
      );
      // If the server has no tools, force-enable "all tools"
      if ((selectedServer.tools?.length ?? 0) === 0) {
        setAllToolsEnabled(true);
      }
    }
  }, [selectedServer]);

  const renderStepContent = () => {
    switch (currentStep) {
      case ConfigurationStep.BASIC_INFO:
        return (
          <div className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Configuration Name *</Label>
              <Input
                id="name"
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

            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                placeholder="Optional description for this configuration"
                rows={3}
                maxLength={500}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="server">MCP Server *</Label>
              <Select value={selectedServerId} onValueChange={setSelectedServerId}>
                <SelectTrigger id="server">
                  <SelectValue placeholder="Select an MCP server" />
                </SelectTrigger>
                <SelectContent>
                  {serversResponse?.data.map((server) => (
                    <SelectItem key={server.id} value={server.id}>
                      <div className="flex items-center gap-2">
                        <span>{server.name}</span>
                        <span className="text-sm text-muted-foreground">
                          ({server.description})
                        </span>
                      </div>
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>
        );

      case ConfigurationStep.AUTHENTICATION:
        if (!selectedServer) {
          return (
            <div className="py-8 text-center text-muted-foreground">
              Please select a server first
            </div>
          );
        }
        return (
          <div className="space-y-3">
            <div className="mb-4 flex items-center gap-2">
              <Label>Authentication Type *</Label>
              <Tooltip>
                <TooltipTrigger asChild>
                  <HelpCircle className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground" />
                </TooltipTrigger>
                <TooltipContent className="max-w-xs">
                  <p className="text-sm">
                    Choose the authentication method for accessing this MCP server.
                  </p>
                </TooltipContent>
              </Tooltip>
            </div>
            <RadioGroup
              value={selectedAuthType}
              onValueChange={(value) => setSelectedAuthType(value as AuthType)}
              className="grid gap-3"
            >
              {selectedServer.supported_auth_types.map((authType) => (
                <label
                  key={authType}
                  htmlFor={`auth-${authType}`}
                  className="flex cursor-pointer items-start space-x-3 rounded-lg border p-4 transition-colors hover:bg-muted/50"
                >
                  <RadioGroupItem value={authType} id={`auth-${authType}`} className="mt-0.5" />
                  <div className="flex-1 space-y-1">
                    <div className="text-sm font-medium">{getAuthTypeLabel(authType)}</div>
                    <p className="text-xs text-muted-foreground">
                      {getAuthTypeDescription(authType)}
                    </p>
                  </div>
                </label>
              ))}
            </RadioGroup>
          </div>
        );

      case ConfigurationStep.CONFIGURATION_TYPE:
        return (
          <div className="space-y-3">
            <div className="mb-4 flex items-center gap-2">
              <Label>Connected Account Type *</Label>
              <Tooltip>
                <TooltipTrigger asChild>
                  <HelpCircle className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground" />
                </TooltipTrigger>
                <TooltipContent className="max-w-xs">
                  <p className="text-sm">
                    Choose whether this configuration is for individual use or shared across the
                    team.
                  </p>
                </TooltipContent>
              </Tooltip>
            </div>
            <RadioGroup
              value={configurationType}
              onValueChange={(value) => setConfigurationType(value as ConnectedAccountOwnership)}
              className="grid gap-3"
            >
              <label
                htmlFor="config-individual"
                className="flex cursor-pointer items-start space-x-3 rounded-lg border p-4 transition-colors hover:bg-muted/50"
              >
                <RadioGroupItem
                  value={ConnectedAccountOwnership.INDIVIDUAL}
                  id="config-individual"
                  className="mt-0.5"
                />
                <div className="flex-1 space-y-1">
                  <div className="text-sm font-medium">Individual</div>
                  <p className="text-xs text-muted-foreground">
                    {getConfigurationTypeDetailedInfo(ConnectedAccountOwnership.INDIVIDUAL)}
                  </p>
                </div>
              </label>
              <label
                htmlFor="config-shared"
                className="flex cursor-pointer items-start space-x-3 rounded-lg border p-4 transition-colors hover:bg-muted/50"
              >
                <RadioGroupItem
                  value={ConnectedAccountOwnership.SHARED}
                  id="config-shared"
                  className="mt-0.5"
                />
                <div className="flex-1 space-y-1">
                  <div className="text-sm font-medium">Shared</div>
                  <p className="text-xs text-muted-foreground">
                    {getConfigurationTypeDetailedInfo(ConnectedAccountOwnership.SHARED)}
                  </p>
                </div>
              </label>
            </RadioGroup>
          </div>
        );

      case ConfigurationStep.TOOLS:
        if (!selectedServer) {
          return (
            <div className="py-8 text-center text-muted-foreground">
              Please select a server first
            </div>
          );
        }
        return (
          <div className="space-y-2">
            <div className="mb-4 flex items-center justify-between">
              <Label>Tools Configuration</Label>
              <div className="flex items-center space-x-2">
                <Label htmlFor="all-tools" className="text-sm">
                  Enable all tools
                </Label>
                <Switch
                  id="all-tools"
                  checked={allToolsEnabled}
                  onCheckedChange={setAllToolsEnabled}
                />
              </div>
            </div>

            {!allToolsEnabled && (selectedServer.tools?.length ?? 0) > 0 && (
              <ScrollArea className="h-64 rounded-md border p-3">
                <div className="space-y-2">
                  {(selectedServer.tools ?? []).map((tool) => (
                    <div key={tool.id} className="flex items-start space-x-2">
                      <Checkbox
                        id={tool.id}
                        checked={selectedTools.includes(tool.id)}
                        onCheckedChange={() => handleToolToggle(tool.id)}
                      />
                      <div className="space-y-1">
                        <Label htmlFor={tool.id} className="cursor-pointer text-sm font-normal">
                          {tool.name}
                        </Label>
                        {tool.description && (
                          <p className="text-xs text-muted-foreground">{tool.description}</p>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}

            {(selectedServer.tools?.length ?? 0) === 0 && (
              <div className="py-8 text-center text-muted-foreground">
                No tools available for this server
              </div>
            )}
          </div>
        );

      case ConfigurationStep.TEAMS:
        if (!teams || teams.length === 0) {
          return (
            <div className="py-8 text-center text-muted-foreground">
              No teams available. You can skip this step.
            </div>
          );
        }
        return (
          <div className="space-y-2">
            <div className="mb-4">
              <Label>Allowed Teams</Label>
              <p className="mt-1 text-sm text-muted-foreground">
                Select which teams can access this configuration
              </p>
            </div>
            <ScrollArea className="h-64 rounded-md border p-3">
              <div className="space-y-2">
                {teams.map((team) => (
                  <div key={team.team_id} className="flex items-center space-x-2">
                    <Checkbox
                      id={team.team_id}
                      checked={selectedTeams.includes(team.team_id)}
                      onCheckedChange={() => handleTeamToggle(team.team_id)}
                    />
                    <Label htmlFor={team.team_id} className="cursor-pointer text-sm font-normal">
                      {team.name}
                    </Label>
                  </div>
                ))}
              </div>
            </ScrollArea>
          </div>
        );

      default:
        return null;
    }
  };

  const getStepTitle = () => {
    switch (currentStep) {
      case ConfigurationStep.BASIC_INFO:
        return "Basic Information";
      case ConfigurationStep.AUTHENTICATION:
        return "Authentication Setup";
      case ConfigurationStep.CONFIGURATION_TYPE:
        return "Connected Account Type";
      case ConfigurationStep.TOOLS:
        return "Tools Configuration";
      case ConfigurationStep.TEAMS:
        return "Team Access";
      default:
        return "";
    }
  };

  const getStepDescription = () => {
    switch (currentStep) {
      case ConfigurationStep.BASIC_INFO:
        return "Provide basic details about your configuration";
      case ConfigurationStep.AUTHENTICATION:
        return "Select how users will authenticate with this server";
      case ConfigurationStep.CONFIGURATION_TYPE:
        return "Choose how this configuration will be managed";
      case ConfigurationStep.TOOLS:
        return "Select which tools to enable for this configuration";
      case ConfigurationStep.TEAMS:
        return "Choose which teams can access this configuration";
      default:
        return "";
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(newOpen) => {
        setOpen(newOpen);
        if (!newOpen) {
          resetForm();
        }
      }}
    >
      <DialogTrigger asChild>
        <Button>
          <Plus className="mr-2 h-4 w-4" />
          Create Configuration
        </Button>
      </DialogTrigger>
      <DialogContent className="max-h-[80vh] max-w-2xl">
        <DialogHeader>
          <DialogTitle>{getStepTitle()}</DialogTitle>
          <DialogDescription>{getStepDescription()}</DialogDescription>
        </DialogHeader>

        <div className="py-4">
          <StepIndicator
            currentStep={currentStep}
            onClick={handleStepClick}
            completedSteps={completedSteps}
          />

          <ScrollArea className="h-[400px] pr-4">{renderStepContent()}</ScrollArea>
        </div>

        <DialogFooter className="flex items-center justify-between">
          <div className="flex gap-2">
            <Button
              variant="outline"
              onClick={() => {
                setOpen(false);
                resetForm();
              }}
            >
              Cancel
            </Button>
            {currentStep > ConfigurationStep.BASIC_INFO && (
              <Button variant="outline" onClick={handlePreviousStep}>
                <ChevronLeft className="mr-2 h-4 w-4" />
                Previous
              </Button>
            )}
          </div>

          <div className="flex gap-2">
            {currentStep < ConfigurationStep.TEAMS ? (
              <Button onClick={handleNextStep}>
                Next
                <ChevronRight className="ml-2 h-4 w-4" />
              </Button>
            ) : (
              <Button
                onClick={handleSubmit}
                disabled={
                  !validateStep(ConfigurationStep.BASIC_INFO) ||
                  !validateStep(ConfigurationStep.AUTHENTICATION) ||
                  createConfiguration.isPending
                }
              >
                {createConfiguration.isPending && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                Create Configuration
              </Button>
            )}
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
