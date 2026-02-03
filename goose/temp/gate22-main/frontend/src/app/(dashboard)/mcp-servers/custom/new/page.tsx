"use client";

import { useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ArrowLeft, HelpCircle, Loader2, Plus } from "lucide-react";
import { useMetaInfo } from "@/components/context/metainfo";
import { toast } from "sonner";
import {
  useOAuth2Discovery,
  useOAuth2ClientRegistration,
  useCreateCustomMCPServer,
} from "@/features/mcp/hooks/use-custom-mcp-server";
import { useOperationalMCPServerConfigurations } from "@/features/mcp/hooks/use-mcp-servers";
import { OperationalAccountDialog } from "@/features/mcp/components/operational-account-dialog";
import {
  OAuth2DiscoveryResponse,
  OAuth2DCRResponse,
  AuthConfig,
} from "@/features/mcp/api/custom-mcp.service";
import { AuthType, MCPServerPublic } from "@/features/mcp/types/mcp.types";

export default function AddCustomMCPServerPage() {
  const router = useRouter();
  const [currentStep, setCurrentStep] = useState(1);
  const [name, setName] = useState("");
  const [authMethods, setAuthMethods] = useState<{
    no_auth: boolean;
    api_key: boolean;
    oauth2: boolean;
  }>({
    no_auth: false,
    api_key: false,
    oauth2: false,
  });
  const [url, setUrl] = useState("");
  const [description, setDescription] = useState("");
  const [logoUrl, setLogoUrl] = useState("");
  const [categories] = useState<string[]>([]);
  // const [setCategoryInput] = useState("");

  // Step 2 fields - OAuth2
  const [oauth2Config, setOauth2Config] = useState<OAuth2DiscoveryResponse>({});
  const [authorizeUrl, setAuthorizeUrl] = useState("");
  const [tokenUrl, setTokenUrl] = useState("");
  const [dcrResult, setDcrResult] = useState<OAuth2DCRResponse | null>(null);
  const [oauth2RegistrationMode, setOauth2RegistrationMode] = useState<string>("");

  // Manual OAuth2 registration fields
  const [manualEndpointAuthMethod, setManualEndpointAuthMethod] = useState<string>("");
  const [manualClientId, setManualClientId] = useState("");
  const [manualClientSecret, setManualClientSecret] = useState("");
  const [manualScope, setManualScope] = useState("");

  // Step 2 fields - API Key
  const [apiKeyLocation, setApiKeyLocation] = useState<string>("");
  const [apiKeyName, setApiKeyName] = useState("");
  const [apiKeyPrefix, setApiKeyPrefix] = useState("");

  // Step 3 fields - Operational Account
  const [operationalAccountAuthType, setOperationalAccountAuthType] = useState<string>("");

  // Step 4 fields - Created server info
  const [createdServer, setCreatedServer] = useState<MCPServerPublic | null>(null);
  const [serverCreated, setServerCreated] = useState<boolean>(false);

  const { accessToken } = useMetaInfo();

  // Hooks for API calls
  const oAuth2Discovery = useOAuth2Discovery();
  const oAuth2ClientRegistration = useOAuth2ClientRegistration();
  const createCustomMCPServer = useCreateCustomMCPServer();
  const { data: operationalConfigs, refetch: refetchOperationalConfigs } =
    useOperationalMCPServerConfigurations();

  // Step 4 state
  const [isOperationalDialogOpen, setIsOperationalDialogOpen] = useState(false);

  // Check if there's an operational account for the created server
  const operationalConfig = operationalConfigs?.data?.find(
    (config) => config.mcp_server.id === createdServer?.id,
  );
  const hasOperationalAccount = operationalConfig?.has_operational_connected_account || false;

  const isAutomaticRegistrationAvailable = useCallback(() => {
    return !!(
      oauth2Config.registration_url &&
      oauth2Config.token_endpoint_auth_method_supported &&
      oauth2Config.token_endpoint_auth_method_supported.length > 0
    );
  }, [oauth2Config.registration_url, oauth2Config.token_endpoint_auth_method_supported]);

  // Auth method management functions
  const handleAuthMethodChange = (method: keyof typeof authMethods, checked: boolean) => {
    setAuthMethods((prev) => ({
      ...prev,
      [method]: checked,
    }));
  };

  const hasSelectedAuthMethod = () => {
    return Object.values(authMethods).some((method) => method);
  };

  const needsStep2 = () => {
    return authMethods.api_key || authMethods.oauth2;
  };

  const needsStep3 = () => {
    return hasSelectedAuthMethod(); // Always show step 3 if any auth method is selected
  };

  // Remove unused needsStep4 function as we directly check serverCreated

  const getSelectedAuthMethods = () => {
    return Object.entries(authMethods)
      .filter(([, selected]) => selected)
      .map(([method]) => method);
  };

  // Check if Step 2 form is valid
  const isStep2Valid = () => {
    // API Key validation
    if (authMethods.api_key) {
      if (!apiKeyLocation || !apiKeyName.trim()) {
        return false;
      }
    }

    // OAuth2 validation
    if (authMethods.oauth2) {
      if (!authorizeUrl.trim() || !tokenUrl.trim()) {
        return false;
      }

      if (oauth2RegistrationMode === "automatic") {
        if (!dcrResult) {
          return false;
        }
      } else if (oauth2RegistrationMode === "manual") {
        if (!manualEndpointAuthMethod || !manualClientId.trim()) {
          return false;
        }
        // Check if client secret is required and provided
        if (manualEndpointAuthMethod.includes("client_secret_") && !manualClientSecret.trim()) {
          return false;
        }
      }
    }

    return true;
  };

  // Category management functions
  // const addCategory = (category: string) => {
  //   const trimmedCategory = category.trim();
  //   if (trimmedCategory && !categories.includes(trimmedCategory)) {
  //     setCategories([...categories, trimmedCategory]);
  //     setCategoryInput("");
  //   }
  // };

  // const removeCategory = (categoryToRemove: string) => {
  //   setCategories(categories.filter((cat) => cat !== categoryToRemove));
  // };

  // const handleCategoryKeyPress = (e: React.KeyboardEvent) => {
  //   if (e.key === "Enter") {
  //     e.preventDefault();
  //     addCategory(categoryInput);
  //   }
  // };

  // Name validation function
  const validateName = (name: string): string | null => {
    if (!name.trim()) {
      return "Server name is required";
    }

    // Check for valid characters: uppercase letters, numbers, underscores only
    if (!/^[A-Z0-9_]+$/.test(name)) {
      return "Name must contain only uppercase letters, numbers, and underscores";
    }

    // Check for consecutive underscores
    if (/__/.test(name)) {
      return "Name cannot contain consecutive underscores";
    }

    return null;
  };

  // URL validation function
  const validateUrl = (url: string): string | null => {
    if (!url.trim()) {
      return "URL is required";
    }

    try {
      new URL(url);
      return null;
    } catch {
      return "Please enter a valid URL";
    }
  };

  const handleStep1Submit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validate name
    const nameError = validateName(name);
    if (nameError) {
      toast.error(nameError);
      return;
    }

    // Validate auth methods
    if (!hasSelectedAuthMethod()) {
      toast.error("Please select at least one authentication method");
      return;
    }

    // Validate URL
    const urlError = validateUrl(url);
    if (urlError) {
      toast.error(urlError);
      return;
    }

    if (!accessToken) {
      toast.error("Authentication required. Please log in.");
      return;
    }

    // If needs step 2 (API Key or OAuth2), proceed to step 2
    if (needsStep2()) {
      // If OAuth2 is selected, perform discovery
      if (authMethods.oauth2) {
        try {
          const response = await oAuth2Discovery.mutateAsync({
            mcp_server_url: url.trim(),
          });

          setOauth2Config(response);
          // Pre-populate fields with discovered values
          setAuthorizeUrl(response.authorize_url || "");
          setTokenUrl(response.access_token_url || "");

          // Set default registration mode based on discovery results
          if (
            response.registration_url &&
            response.token_endpoint_auth_method_supported &&
            response.token_endpoint_auth_method_supported.length > 0
          ) {
            setOauth2RegistrationMode("automatic");
          } else {
            setOauth2RegistrationMode("manual");
          }

          setCurrentStep(2);
        } catch {
          // Error handling is done in the hook
          // Set manual mode as fallback when discovery fails
          setOauth2RegistrationMode("manual");
          setCurrentStep(2);
        }
      } else {
        // For API Key only, go directly to step 2
        setCurrentStep(2);
      }
    } else {
      // For No Auth only, go to step 3 (operational account)
      setCurrentStep(3);
    }
  };

  const createServer = async (): Promise<void> => {
    const authConfigs: AuthConfig[] = [];

    // Build auth configs for each selected auth method
    if (authMethods.no_auth) {
      authConfigs.push({
        type: "no_auth",
      });
    }

    if (authMethods.api_key) {
      authConfigs.push({
        type: "api_key",
        location: apiKeyLocation,
        name: apiKeyName.trim(),
        prefix: apiKeyPrefix.trim() || undefined,
      });
    }

    if (authMethods.oauth2) {
      const oauth2AuthConfig: AuthConfig = {
        type: "oauth2",
        location: "header",
        name: "Authorization",
        prefix: "Bearer",
        authorize_url: authorizeUrl.trim(),
        access_token_url: tokenUrl.trim(),
        refresh_token_url: oauth2Config.refresh_token_url || undefined,
        scope: "",
      };

      if (oauth2RegistrationMode === "automatic" && dcrResult) {
        oauth2AuthConfig.client_id = dcrResult.client_id;
        oauth2AuthConfig.client_secret = dcrResult.client_secret;
        oauth2AuthConfig.token_endpoint_auth_method = dcrResult.token_endpoint_auth_method;
      } else if (oauth2RegistrationMode === "manual") {
        oauth2AuthConfig.client_id = manualClientId.trim();
        oauth2AuthConfig.client_secret = manualClientSecret.trim() || undefined;
        oauth2AuthConfig.scope = manualScope.trim() || "";
        oauth2AuthConfig.token_endpoint_auth_method = manualEndpointAuthMethod;
      }

      authConfigs.push(oauth2AuthConfig);
    }

    const payload = {
      name: name.trim(),
      url: url.trim(),
      description: description.trim(),
      categories: categories,
      auth_configs: authConfigs,
      server_metadata: {},
      operational_account_auth_type: operationalAccountAuthType,
      ...(logoUrl.trim() && { logo: logoUrl.trim() }),
    };

    const response = await createCustomMCPServer.mutateAsync(payload);

    setCreatedServer(response);
    setServerCreated(true);
    setCurrentStep(4);

    // Store the created server info and proceed to step 4
  };

  const handleAutoRegisterClient = async () => {
    if (!oauth2Config.registration_url) {
      toast.error("Registration URL not available from OAuth2 discovery");
      return;
    }

    try {
      const response = await oAuth2ClientRegistration.mutateAsync({
        mcp_server_url: url.trim(),
        registration_url: oauth2Config.registration_url,
        token_endpoint_auth_method_supported:
          oauth2Config.token_endpoint_auth_method_supported || [],
      });

      setDcrResult(response);
    } catch {
      // Error handling is done in the hook
    }
  };

  const handleStep2Submit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validate API Key fields if API Key is selected
    if (authMethods.api_key) {
      if (!apiKeyLocation) {
        toast.error("Please select an API key location");
        return;
      }
      if (!apiKeyName.trim()) {
        toast.error("Please enter an API key name");
        return;
      }
    }

    // Validate OAuth2 fields if OAuth2 is selected
    if (authMethods.oauth2) {
      if (!authorizeUrl.trim()) {
        toast.error("Please enter an authorization URL");
        return;
      }
      if (!tokenUrl.trim()) {
        toast.error("Please enter a token URL");
        return;
      }

      if (oauth2RegistrationMode === "automatic") {
        if (!dcrResult) {
          toast.error("Please complete auto client registration before proceeding");
          return;
        }
      } else if (oauth2RegistrationMode === "manual") {
        if (!manualEndpointAuthMethod) {
          toast.error("Please select an endpoint auth method");
          return;
        }
        if (!manualClientId.trim()) {
          toast.error("Please enter a client ID");
          return;
        }
        // Validate client secret is provided when required by auth method
        if (manualEndpointAuthMethod.includes("client_secret_") && !manualClientSecret.trim()) {
          toast.error("Please enter a client secret for the selected auth method");
          return;
        }
      }
    }

    setCurrentStep(3);
  };

  const handleStep3Submit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validate operational account auth type
    if (!operationalAccountAuthType) {
      toast.error("Please select an operational account authentication method");
      return;
    }

    await createServer();
  };

  return (
    <div className="mx-auto max-w-4xl p-6">
      {/* Back Button */}
      <Button variant="outline" onClick={() => router.push("/mcp-servers")} className="mb-6">
        <ArrowLeft className="mr-2 h-4 w-4" />
        Back to MCP Servers
      </Button>

      {/* Header */}
      <div className="mb-6 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Add Custom MCP Server</h1>
          <p className="mt-1 text-muted-foreground">
            {currentStep === 1
              ? needsStep3()
                ? `Step ${currentStep} of 4: Server Details`
                : "Create a new custom MCP server configuration"
              : currentStep === 2
                ? `Step ${currentStep} of 4: Setup Auth Method`
                : currentStep === 3
                  ? `Step ${currentStep} of 4: Operational Account`
                  : currentStep === 4
                    ? `Step ${currentStep} of 4: Setup Operational Account (Optional)`
                    : "Create a new custom MCP server configuration"}
          </p>
        </div>
      </div>

      <Separator className="mb-6" />

      {/* Step 1: Server Details */}
      {currentStep === 1 && (
        <div className="mb-8">
          <h2 className="mb-4 text-lg font-semibold">Server Details</h2>
          <form onSubmit={handleStep1Submit} className="space-y-6">
            <div className="space-y-2">
              <Label htmlFor="name">
                Server Name <span className="text-red-500">*</span>
              </Label>
              <Input
                id="name"
                type="text"
                placeholder="MY_CUSTOM_SERVER"
                value={name}
                onChange={(e) => {
                  // Only allow uppercase letters, digits, and underscores
                  const filteredValue = e.target.value.toUpperCase().replace(/[^A-Z0-9_]/g, "");
                  setName(filteredValue);
                }}
                disabled={oAuth2Discovery.isPending}
                required
                className="max-w-md"
              />
              <p className="text-sm text-muted-foreground">
                Use uppercase letters, numbers, and underscores only. No consecutive underscores.
              </p>
            </div>

            <div className="space-y-2">
              <Label>
                Authentication Methods <span className="text-red-500">*</span>
              </Label>
              <div className="space-y-3">
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="no_auth"
                    checked={authMethods.no_auth}
                    onCheckedChange={(checked) =>
                      handleAuthMethodChange("no_auth", checked as boolean)
                    }
                    disabled={oAuth2Discovery.isPending}
                  />
                  <Label htmlFor="no_auth" className="text-sm font-normal">
                    No Auth
                  </Label>
                </div>
                {/* Commenting out API Key for now as it would bring in more complexity for users to setup */}
                {/* <div className="flex items-center space-x-2">
                  <Checkbox
                    id="api_key"
                    checked={authMethods.api_key}
                    onCheckedChange={(checked) =>
                      handleAuthMethodChange("api_key", checked as boolean)
                    }
                    disabled={oAuth2Discovery.isPending}
                  />
                  <Label htmlFor="api_key" className="text-sm font-normal">
                    API Key
                  </Label>
                </div> */}
                <div className="flex items-center space-x-2">
                  <Checkbox
                    id="oauth2"
                    checked={authMethods.oauth2}
                    onCheckedChange={(checked) =>
                      handleAuthMethodChange("oauth2", checked as boolean)
                    }
                    disabled={oAuth2Discovery.isPending}
                  />
                  <Label htmlFor="oauth2" className="text-sm font-normal">
                    OAuth2
                  </Label>
                </div>
              </div>
              <p className="text-sm text-muted-foreground">
                Select one or more authentication methods supported by your server.
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="url">
                Server URL <span className="text-red-500">*</span>
              </Label>
              <Input
                id="url"
                type="url"
                placeholder="http://mcp.example.com/mcp"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                disabled={oAuth2Discovery.isPending}
                required
                className="max-w-md"
              />
              <p className="text-sm text-muted-foreground">
                Enter the full URL to your MCP server endpoint.
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Textarea
                id="description"
                placeholder="Enter a description for your MCP server..."
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                disabled={oAuth2Discovery.isPending}
                className="max-w-md"
                rows={3}
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="logoUrl">Logo URL</Label>
              <Input
                id="logoUrl"
                type="url"
                placeholder="https://example.com/logo.png"
                value={logoUrl}
                onChange={(e) => setLogoUrl(e.target.value)}
                disabled={oAuth2Discovery.isPending}
                className="max-w-md"
              />
            </div>

            {/* <div className="space-y-2">
              <Label htmlFor="categories">Categories</Label>
              <div className="space-y-2">
                <div className="flex flex-wrap gap-2 mb-2">
                  {categories.map((category, index) => (
                    <Badge
                      key={index}
                      variant="secondary"
                      className="flex items-center gap-1"
                    >
                      {category}
                      <button
                        type="button"
                        onClick={() => removeCategory(category)}
                        className="ml-1 hover:bg-muted rounded-full p-0.5"
                        disabled={oAuth2Discovery.isPending}
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </Badge>
                  ))}
                </div>
                <div className="flex gap-2">
                  <Input
                    id="categories"
                    type="text"
                    placeholder="Add a category and press Enter..."
                    value={categoryInput}
                    onChange={(e) => setCategoryInput(e.target.value)}
                    onKeyPress={handleCategoryKeyPress}
                    disabled={oAuth2Discovery.isPending}
                    className="max-w-md"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => addCategory(categoryInput)}
                    disabled={
                      oAuth2Discovery.isPending || !categoryInput.trim()
                    }
                  >
                    Add
                  </Button>
                </div>
              </div>
            </div> */}

            <div className="flex gap-2 pt-4">
              <Button
                type="submit"
                disabled={
                  oAuth2Discovery.isPending ||
                  createCustomMCPServer.isPending ||
                  !name.trim() ||
                  !hasSelectedAuthMethod() ||
                  !url.trim()
                }
                className="flex items-center gap-2"
              >
                {oAuth2Discovery.isPending || createCustomMCPServer.isPending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Plus className="h-4 w-4" />
                )}
                {oAuth2Discovery.isPending
                  ? "Discovering..."
                  : createCustomMCPServer.isPending
                    ? "Registering..."
                    : needsStep2()
                      ? "Next: Setup Auth Method"
                      : "Next: Operational Account"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => router.push("/mcp-servers")}
                disabled={oAuth2Discovery.isPending || createCustomMCPServer.isPending}
              >
                Cancel
              </Button>
            </div>
          </form>
        </div>
      )}

      {/* Step 2: Setup Auth Method */}
      {currentStep === 2 && (
        <div className="mb-8">
          <h2 className="mb-4 text-lg font-semibold">Setup Auth Method</h2>
          <form onSubmit={handleStep2Submit} className="space-y-6">
            {authMethods.api_key && (
              <div className="space-y-4 rounded-lg border border-gray-200 p-4">
                <h3 className="text-md font-medium">API Key Configuration</h3>

                <div className="space-y-2">
                  <Label htmlFor="apiKeyLocation">
                    Location <span className="text-red-500">*</span>
                  </Label>
                  <Select
                    value={apiKeyLocation}
                    onValueChange={setApiKeyLocation}
                    disabled={createCustomMCPServer.isPending}
                  >
                    <SelectTrigger className="max-w-md">
                      <SelectValue placeholder="Select API key location" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="path">PATH</SelectItem>
                      <SelectItem value="query">QUERY</SelectItem>
                      <SelectItem value="header">HEADER</SelectItem>
                      <SelectItem value="cookie">COOKIE</SelectItem>
                      <SelectItem value="body">BODY</SelectItem>
                    </SelectContent>
                  </Select>
                  <p className="text-sm text-muted-foreground">
                    The location of the API key in the request.
                  </p>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="apiKeyName">
                    Name <span className="text-red-500">*</span>
                  </Label>
                  <Input
                    id="apiKeyName"
                    type="text"
                    placeholder="X-Subscription-Token"
                    value={apiKeyName}
                    onChange={(e) => setApiKeyName(e.target.value)}
                    disabled={createCustomMCPServer.isPending}
                    className="max-w-md"
                  />
                  <p className="text-sm text-muted-foreground">
                    The name of the API key in the request, e.g., &apos;X-Subscription-Token&apos;.
                  </p>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="apiKeyPrefix">Prefix</Label>
                  <Input
                    id="apiKeyPrefix"
                    type="text"
                    placeholder="Bearer"
                    value={apiKeyPrefix}
                    onChange={(e) => setApiKeyPrefix(e.target.value)}
                    disabled={createCustomMCPServer.isPending}
                    className="max-w-md"
                  />
                  <p className="text-sm text-muted-foreground">
                    The prefix of the API key in the request, e.g., &apos;Bearer&apos;.
                  </p>
                </div>
              </div>
            )}

            {authMethods.oauth2 && (
              <div className="space-y-4 rounded-lg border border-gray-200 p-4">
                <h3 className="text-md font-medium">OAuth2 Configuration</h3>

                <div className="space-y-2">
                  <Label htmlFor="authorizeUrl">
                    Authorization URL <span className="text-red-500">*</span>
                  </Label>
                  <Input
                    id="authorizeUrl"
                    type="url"
                    placeholder="https://example.com/oauth/authorize"
                    value={authorizeUrl}
                    onChange={(e) => setAuthorizeUrl(e.target.value)}
                    disabled={createCustomMCPServer.isPending}
                    className="max-w-md"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="tokenUrl">
                    Token URL <span className="text-red-500">*</span>
                  </Label>
                  <Input
                    id="tokenUrl"
                    type="url"
                    placeholder="https://example.com/oauth/token"
                    value={tokenUrl}
                    onChange={(e) => setTokenUrl(e.target.value)}
                    disabled={createCustomMCPServer.isPending}
                    className="max-w-md"
                  />
                </div>

                <div className="space-y-2">
                  <Label htmlFor="authMethodsSupported">
                    Token Endpoint Auth Methods Supported
                  </Label>
                  <Input
                    id="authMethodsSupported"
                    type="text"
                    value={oauth2Config.token_endpoint_auth_method_supported?.join(", ") || "None"}
                    disabled
                    readOnly
                    className="max-w-md bg-muted"
                  />
                  <p className="text-sm text-muted-foreground">
                    Supported authentication methods discovered from the server.
                  </p>
                </div>

                {/* Client Registration Mode */}
                <div className="space-y-4">
                  <Label>Client Registration</Label>
                  <RadioGroup
                    value={oauth2RegistrationMode}
                    onValueChange={setOauth2RegistrationMode}
                    disabled={createCustomMCPServer.isPending}
                    className="flex flex-row space-x-6"
                  >
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem
                        value="automatic"
                        id="automatic"
                        disabled={
                          !isAutomaticRegistrationAvailable() || createCustomMCPServer.isPending
                        }
                      />
                      <Label
                        htmlFor="automatic"
                        className={`text-sm font-normal ${
                          !isAutomaticRegistrationAvailable() ? "text-muted-foreground" : ""
                        }`}
                      >
                        Automatic
                        {!isAutomaticRegistrationAvailable() && (
                          <span className="ml-1 text-xs text-muted-foreground">
                            (Not available)
                          </span>
                        )}
                      </Label>
                    </div>
                    <div className="flex items-center space-x-2">
                      <RadioGroupItem value="manual" id="manual" />
                      <Label htmlFor="manual" className="text-sm font-normal">
                        Manual
                      </Label>
                    </div>
                  </RadioGroup>
                  {!isAutomaticRegistrationAvailable() && (
                    <p className="text-sm text-muted-foreground">
                      Automatic registration is not available because the server did not provide a
                      registration URL or supported authentication methods during discovery.
                    </p>
                  )}
                </div>

                {oauth2RegistrationMode === "automatic" && oauth2Config.registration_url && (
                  <div className="space-y-4">
                    <div className="flex items-center gap-4">
                      <Button
                        type="button"
                        variant="outline"
                        onClick={handleAutoRegisterClient}
                        disabled={
                          oAuth2ClientRegistration.isPending || createCustomMCPServer.isPending
                        }
                        className="flex items-center gap-2"
                      >
                        {oAuth2ClientRegistration.isPending ? (
                          <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                          <Plus className="h-4 w-4" />
                        )}
                        {oAuth2ClientRegistration.isPending
                          ? "Registering..."
                          : "Auto Register Client"}
                      </Button>
                      {dcrResult && (
                        <span className="text-sm font-medium text-green-600">
                          ✓ Client registered successfully
                        </span>
                      )}
                    </div>

                    {dcrResult && (
                      <div className="space-y-3 rounded-lg border border-green-200 bg-green-50 p-4">
                        <h4 className="text-sm font-medium text-green-800">Registration Results</h4>
                        <div className="grid grid-cols-1 gap-3 text-sm">
                          <div>
                            <span className="font-medium text-green-700">Auth Method:</span>{" "}
                            <span className="text-green-600">
                              {dcrResult.token_endpoint_auth_method}
                            </span>
                          </div>
                          {dcrResult.client_id && (
                            <div>
                              <span className="font-medium text-green-700">Client ID:</span>{" "}
                              <span className="font-mono text-xs text-green-600">
                                {dcrResult.client_id}
                              </span>
                            </div>
                          )}
                          {dcrResult.client_secret && (
                            <div>
                              <span className="font-medium text-green-700">Client Secret:</span>{" "}
                              <span className="font-mono text-xs text-green-600">
                                {"•".repeat(8)}...
                                {dcrResult.client_secret.slice(-4)}
                              </span>
                            </div>
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Manual Registration */}
                {oauth2RegistrationMode === "manual" && (
                  <div className="space-y-4">
                    <div className="space-y-2">
                      <Label htmlFor="manualEndpointAuthMethod">
                        Endpoint Auth Method <span className="text-red-500">*</span>
                      </Label>
                      <Select
                        value={manualEndpointAuthMethod}
                        onValueChange={setManualEndpointAuthMethod}
                        disabled={createCustomMCPServer.isPending}
                      >
                        <SelectTrigger className="max-w-md">
                          <SelectValue placeholder="Select auth method" />
                        </SelectTrigger>
                        <SelectContent>
                          {oauth2Config.token_endpoint_auth_method_supported?.map((method) => (
                            <SelectItem key={method} value={method}>
                              {method}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>

                    <div className="space-y-2">
                      <div className="flex items-center gap-2">
                        <Label htmlFor="manualClientId">
                          Client ID <span className="text-red-500">*</span>
                        </Label>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <HelpCircle className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground" />
                          </TooltipTrigger>
                          <TooltipContent className="max-w-xs">
                            <p className="text-sm">
                              Enter the Client ID from your OAuth2 provider registration results.
                              This is the unique identifier for your application.
                            </p>
                          </TooltipContent>
                        </Tooltip>
                      </div>
                      <Input
                        id="manualClientId"
                        type="text"
                        placeholder="Paste client ID from registration results"
                        value={manualClientId}
                        onChange={(e) => setManualClientId(e.target.value)}
                        disabled={createCustomMCPServer.isPending}
                        className="max-w-md"
                      />
                    </div>

                    {/* Client Secret - only show when auth method requires it */}
                    {manualEndpointAuthMethod.includes("client_secret_") && (
                      <div className="space-y-2">
                        <div className="flex items-center gap-2">
                          <Label htmlFor="manualClientSecret">
                            Client Secret <span className="text-red-500">*</span>
                          </Label>
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <HelpCircle className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground" />
                            </TooltipTrigger>
                            <TooltipContent className="max-w-xs">
                              <p className="text-sm">
                                Enter the Client Secret from your OAuth2 provider registration
                                results. This confidential key is required for client_secret_*
                                authentication methods.
                              </p>
                            </TooltipContent>
                          </Tooltip>
                        </div>
                        <Input
                          id="manualClientSecret"
                          type="password"
                          placeholder="Paste client secret from registration results"
                          value={manualClientSecret}
                          onChange={(e) => setManualClientSecret(e.target.value)}
                          disabled={createCustomMCPServer.isPending}
                          className="max-w-md"
                        />
                      </div>
                    )}

                    <div className="space-y-2">
                      <div className="flex items-center gap-2">
                        <Label htmlFor="manualScope">Scope</Label>
                        <Tooltip>
                          <TooltipTrigger asChild>
                            <HelpCircle className="h-3.5 w-3.5 cursor-help text-muted-foreground transition-colors hover:text-foreground" />
                          </TooltipTrigger>
                          <TooltipContent className="max-w-xs">
                            <p className="text-sm">
                              Enter the OAuth2 scope you used to register the client, or copy the
                              scope from the registration results.
                            </p>
                          </TooltipContent>
                        </Tooltip>
                      </div>
                      <Input
                        id="manualScope"
                        type="text"
                        placeholder="Enter the scope you used to register the client."
                        value={manualScope}
                        onChange={(e) => setManualScope(e.target.value)}
                        disabled={createCustomMCPServer.isPending}
                        className="max-w-md"
                      />
                    </div>
                  </div>
                )}
              </div>
            )}

            <div className="flex gap-2 pt-4">
              <Button
                type="button"
                variant="outline"
                onClick={() => setCurrentStep(1)}
                disabled={createCustomMCPServer.isPending}
              >
                Back
              </Button>
              <Button
                type="submit"
                disabled={createCustomMCPServer.isPending || !isStep2Valid()}
                className="flex items-center gap-2"
              >
                {createCustomMCPServer.isPending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Plus className="h-4 w-4" />
                )}
                {createCustomMCPServer.isPending ? "Registering..." : "Next: Operational Account"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => router.push("/mcp-servers")}
                disabled={createCustomMCPServer.isPending}
              >
                Cancel
              </Button>
            </div>
          </form>
        </div>
      )}

      {/* Step 3: Operational Account */}
      {currentStep === 3 && (
        <div className="mb-8">
          <h2 className="mb-4 text-lg font-semibold">Operational Account</h2>
          <form onSubmit={handleStep3Submit} className="space-y-6">
            <div className="space-y-4">
              <div className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  The operational account is exclusively used by the system for administrative
                  purposes such as fetching MCP server metadata and monitoring server status. It
                  will never be used by any users.
                </p>
              </div>

              <p className="text-sm text-muted-foreground">
                Please select the authentication method used to connect Operational Account.
              </p>

              <div className="space-y-2">
                <Label htmlFor="operationalAccountAuthType">
                  Operational Account Auth Method <span className="text-red-500">*</span>
                </Label>
                <Select
                  value={operationalAccountAuthType}
                  onValueChange={setOperationalAccountAuthType}
                  disabled={createCustomMCPServer.isPending}
                  required
                >
                  <SelectTrigger className="max-w-md">
                    <SelectValue placeholder="Select auth method" />
                  </SelectTrigger>
                  <SelectContent>
                    {getSelectedAuthMethods().map((method) => (
                      <SelectItem key={method} value={method}>
                        {method === "no_auth"
                          ? "No Auth"
                          : method === "api_key"
                            ? "API Key"
                            : method === "oauth2"
                              ? "OAuth2"
                              : method}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>

            <div className="flex gap-2 pt-4">
              <Button
                type="button"
                variant="outline"
                onClick={() => setCurrentStep(needsStep2() ? 2 : 1)}
                disabled={createCustomMCPServer.isPending}
              >
                Back
              </Button>
              <Button
                type="submit"
                disabled={createCustomMCPServer.isPending || !operationalAccountAuthType}
                className="flex items-center gap-2"
              >
                {createCustomMCPServer.isPending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Plus className="h-4 w-4" />
                )}
                {createCustomMCPServer.isPending ? "Registering..." : "Register Server"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => router.push("/mcp-servers")}
                disabled={createCustomMCPServer.isPending}
              >
                Cancel
              </Button>
            </div>
          </form>
        </div>
      )}

      {/* Step 4: Setup Operational Account (Optional) */}
      {currentStep === 4 && serverCreated && createdServer && (
        <div className="mb-8">
          <div className="mb-4 rounded-lg border border-green-200 bg-green-50 p-4">
            <div className="mb-2 flex items-center gap-2">
              <div className="h-2 w-2 rounded-full bg-green-500"></div>
              <span className="text-sm font-medium text-green-800">
                Server Created Successfully
              </span>
            </div>
            <p className="text-sm text-green-700">
              Your MCP server &ldquo;{createdServer.name}&rdquo; has been created and is ready to
              use.
            </p>
          </div>
          <h2 className="mb-4 text-lg font-semibold">Setup Operational Account (Optional)</h2>

          <div className="space-y-6">
            <div className="space-y-4">
              <div className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  The operational account is exclusively used by the system for administrative
                  purposes such as fetching MCP server metadata and monitoring server status. It
                  will never be used by any users.
                </p>
                <p className="text-sm text-muted-foreground">
                  You can set this up now or skip and configure it later from the server details
                  page.
                </p>
              </div>

              <div className="flex gap-2">
                {hasOperationalAccount ? (
                  <>
                    <div className="flex items-center gap-2 text-green-600">
                      <div className="flex h-4 w-4 items-center justify-center rounded-full bg-green-500">
                        <div className="h-2 w-2 rounded-full bg-white"></div>
                      </div>
                      <span className="text-sm">Operational account is configured</span>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="flex items-center gap-2 text-red-600">
                      <div className="h-4 w-4 rounded-full border-2 border-red-500"></div>
                      <span className="text-sm">No operational account configured</span>
                    </div>
                  </>
                )}
              </div>
            </div>

            <div className="flex gap-2 pt-4">
              <Button
                variant="outline"
                onClick={() => router.push(`/mcp-servers/${createdServer.id}`)}
                className="flex items-center gap-2"
              >
                Go to MCP Server
              </Button>
              {!hasOperationalAccount && (
                <Button
                  onClick={() => setIsOperationalDialogOpen(true)}
                  className="flex items-center gap-2"
                >
                  <Plus className="h-4 w-4" />
                  Setup Operational Account
                </Button>
              )}
            </div>
          </div>
        </div>
      )}

      {/* Operational Account Dialog for Step 4 */}
      {serverCreated && createdServer && operationalConfig && (
        <OperationalAccountDialog
          open={isOperationalDialogOpen}
          onOpenChange={setIsOperationalDialogOpen}
          server={{
            id: operationalConfig.mcp_server_id,
            name: createdServer?.name,
            auth_type: operationalAccountAuthType as AuthType,
          }}
          operationalConfigId={operationalConfig.id}
          onSuccess={() => {
            // Refresh operational configs to update the status
            refetchOperationalConfigs();
            setIsOperationalDialogOpen(false);
          }}
        />
      )}
    </div>
  );
}
