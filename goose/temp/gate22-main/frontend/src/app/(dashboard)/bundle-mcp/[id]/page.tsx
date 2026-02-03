"use client";

import { useParams, useRouter } from "next/navigation";
import { useMCPServerBundle } from "@/features/bundle-mcp/hooks/use-bundle-mcp";
import { getMcpBaseUrl } from "@/lib/api-client";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  ArrowLeft,
  Loader2,
  Copy,
  Check,
  AlertCircle,
  Package,
  Terminal,
  Eye,
  EyeOff,
  Shield,
} from "lucide-react";
import Image from "next/image";
import { useState, useMemo } from "react";
import { toast } from "sonner";
import { PERMISSIONS } from "@/lib/rbac/permissions";
import { PermissionGuard } from "@/components/rbac/permission-guard";

export default function BundleDetailPage() {
  const params = useParams();
  const router = useRouter();
  const bundleId = params.id as string;
  const [copiedField, setCopiedField] = useState<string | null>(null);
  const [isUrlVisible, setIsUrlVisible] = useState(false);

  const { data: bundle, isLoading, error } = useMCPServerBundle(bundleId);

  // Generate MCP URL using configured base URL
  const mcpUrl = useMemo(() => {
    if (bundle) {
      const baseUrl = getMcpBaseUrl();
      return `${baseUrl}/mcp?bundle_key=${bundle.bundle_key}`;
    }
    return "";
  }, [bundle]);

  // Generate masked MCP URL for display
  const maskedMcpUrl = useMemo(() => {
    if (bundle) {
      const baseUrl = getMcpBaseUrl();
      return `${baseUrl}/mcp?bundle_key=••••••••••••••••••••••••••••••••••••••••`;
    }
    return "";
  }, [bundle]);

  // Generate configuration for different editors
  const generateConfig = (url: string, bundleName: string, editor: string) => {
    const configKey = editor === "vscode" ? "servers" : "mcpServers";
    return JSON.stringify(
      {
        [configKey]: {
          [bundleName]: {
            url: url,
          },
        },
      },
      null,
      2,
    );
  };

  // Editor configuration data
  const editorConfigs = [
    {
      id: "cursor",
      name: "Cursor",
      instructions: [
        "Add this configuration to your Cursor settings:",
        "Settings → Features → MCP → Edit Config",
      ],
    },
    {
      id: "windsurf",
      name: "Windsurf",
      instructions: [
        "Add this configuration to your Windsurf settings:",
        "Settings → AI → Manage MCP servers → Add custom server",
      ],
    },
    {
      id: "claude-code",
      name: "Claude Code",
      instructions: ["Add this configuration to your .mcp.json file in your project root:", ""],
    },
    {
      id: "vscode",
      name: "VS Code",
      instructions: [
        "Add this configuration to your VS Code settings:",
        "Settings → Extensions → MCP → Server Configuration",
      ],
    },
  ];

  const handleCopy = (text: string, fieldName: string) => {
    navigator.clipboard.writeText(text);
    setCopiedField(fieldName);
    toast.success(`Copied to clipboard`);
    setTimeout(() => setCopiedField(null), 2000);
  };

  const handleMcpUrlCopy = () => {
    if (!isUrlVisible) {
      toast.error("Please show the URL first to copy it");
      return;
    }
    handleCopy(mcpUrl, "mcp-url");
  };

  const handleConfigCopy = (editor: string) => {
    const config = generateConfig(mcpUrl, bundle!.name, editor);
    handleCopy(config, `${editor}-config`);
  };

  if (isLoading) {
    return (
      <div className="flex min-h-[400px] items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error || !bundle) {
    return (
      <div className="container mx-auto max-w-5xl p-6">
        <Button variant="outline" onClick={() => router.push("/bundle-mcp")} className="mb-4">
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Bundles
        </Button>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="h-5 w-5" />
              <p>Failed to load bundle details</p>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto max-w-5xl space-y-6 p-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <Button variant="outline" onClick={() => router.push("/bundle-mcp")}>
          <ArrowLeft className="mr-2 h-4 w-4" />
          Back to Bundles
        </Button>
      </div>

      {/* Bundle Information */}
      <Card>
        <CardHeader>
          <div className="flex items-center gap-4">
            <div className="rounded-lg bg-primary/10 p-3">
              <Package className="h-8 w-8 text-primary" />
            </div>
            <div className="flex-1">
              <CardTitle className="text-2xl font-bold">{bundle.name}</CardTitle>
              {bundle.description && (
                <p className="mt-1 text-sm text-muted-foreground">{bundle.description}</p>
              )}
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-6">
          <PermissionGuard permission={PERMISSIONS.BUNDLE_MCP_URL_VIEW}>
            {/* Security Warning */}
            <div className="flex items-start gap-3 rounded-lg border border-amber-200 bg-amber-50 p-4">
              <Shield className="mt-0.5 h-5 w-5 flex-shrink-0 text-amber-600" />
              <div className="space-y-1">
                <p className="text-sm font-medium text-amber-800">Security Notice</p>
                <p className="text-sm text-amber-700">
                  The MCP URL below contains a secret bundle ID that should be treated as a service
                  token.
                  <strong> Do not share this URL with others</strong> as it provides access to your
                  MCP server bundle.
                </p>
              </div>
            </div>

            {/* MCP URL - Primary Focus */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium text-muted-foreground">MCP URL</label>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => setIsUrlVisible(!isUrlVisible)}
                    className="flex-shrink-0"
                  >
                    {isUrlVisible ? (
                      <>
                        <EyeOff className="mr-2 h-4 w-4" />
                        Hide
                      </>
                    ) : (
                      <>
                        <Eye className="mr-2 h-4 w-4" />
                        Show
                      </>
                    )}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleMcpUrlCopy}
                    className="flex-shrink-0"
                  >
                    {copiedField === "mcp-url" ? (
                      <>
                        <Check className="mr-2 h-4 w-4 text-green-600" />
                        Copied
                      </>
                    ) : (
                      <>
                        <Copy className="mr-2 h-4 w-4" />
                        Copy
                      </>
                    )}
                  </Button>
                </div>
              </div>
              <code className="block w-full rounded border bg-muted/20 px-3 py-2 font-mono text-sm break-words whitespace-pre-wrap">
                {isUrlVisible ? mcpUrl : maskedMcpUrl}
              </code>
              {!isUrlVisible && (
                <p className="text-xs text-muted-foreground">
                  Click &quot;Show&quot; to reveal the MCP URL with the secret bundle ID
                </p>
              )}
            </div>
          </PermissionGuard>

          {/* Additional Details */}
          <div className="space-y-3 text-sm">
            <div className="flex items-center justify-between border-b py-2">
              <span className="text-muted-foreground">Created</span>
              <span>{new Date(bundle.created_at).toLocaleDateString()}</span>
            </div>
            <div className="flex items-center justify-between py-2">
              <span className="text-muted-foreground">Last Updated</span>
              <span>{new Date(bundle.updated_at).toLocaleDateString()}</span>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* MCP Configuration for Different Editors */}
      <Card>
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Terminal className="h-5 w-5 text-muted-foreground" />
              <CardTitle className="text-lg">Quick Setup for Code Editors</CardTitle>
            </div>
            <Badge variant="secondary" className="text-xs">
              HTTP Streaming MCP Server
            </Badge>
          </div>
          <p className="mt-2 text-sm text-muted-foreground">
            Copy and paste the configuration below into your preferred code editor to use this MCP
            server
          </p>
        </CardHeader>
        <CardContent>
          <Tabs defaultValue="cursor" className="w-full">
            <TabsList className="grid h-auto w-full grid-cols-4">
              {editorConfigs.map((editor) => (
                <TabsTrigger key={editor.id} value={editor.id}>
                  {editor.name}
                </TabsTrigger>
              ))}
            </TabsList>

            {editorConfigs.map((editor) => {
              const config = generateConfig(
                isUrlVisible ? mcpUrl : maskedMcpUrl,
                bundle.name,
                editor.id,
              );
              const configId = `${editor.id}-config`;

              return (
                <TabsContent key={editor.id} value={editor.id} className="space-y-3">
                  <div className="text-sm text-muted-foreground">
                    <p>
                      {editor.id === "claude-code" ? (
                        <>
                          Add this configuration to your{" "}
                          <code className="rounded bg-muted px-1 py-0.5 text-xs">.mcp.json</code>{" "}
                          file in your project root:
                        </>
                      ) : (
                        editor.instructions[0]
                      )}
                    </p>
                    {editor.instructions[1] && <p className="mt-1">{editor.instructions[1]}</p>}
                  </div>
                  <div className="relative">
                    <pre className="overflow-x-auto rounded-lg border bg-muted/50 p-4 text-xs">
                      <code>{config}</code>
                    </pre>
                    <Button
                      variant="outline"
                      size="sm"
                      className="absolute top-2 right-2"
                      onClick={() => handleConfigCopy(editor.id)}
                    >
                      {copiedField === configId ? (
                        <>
                          <Check className="mr-1 h-3 w-3 text-green-600" />
                          Copied
                        </>
                      ) : (
                        <>
                          <Copy className="mr-1 h-3 w-3" />
                          Copy
                        </>
                      )}
                    </Button>
                  </div>
                </TabsContent>
              );
            })}
          </Tabs>
        </CardContent>
      </Card>

      {/* MCP Server Configurations */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-lg">MCP Server Configurations</CardTitle>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="divide-y">
            {bundle.mcp_server_configurations.map((config) => (
              <div key={config.id} className="flex items-center justify-between py-3">
                <div className="flex items-center gap-3">
                  {config.mcp_server.logo ? (
                    <Image
                      src={config.mcp_server.logo}
                      alt={config.mcp_server.name}
                      width={24}
                      height={24}
                      className="rounded"
                    />
                  ) : (
                    <div className="flex h-6 w-6 items-center justify-center rounded bg-muted">
                      <Package className="h-3 w-3 text-muted-foreground" />
                    </div>
                  )}
                  <div>
                    <h3 className="text-sm font-medium">{config.name || config.mcp_server.name}</h3>
                    <p className="text-xs text-muted-foreground">{config.mcp_server.name}</p>
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => router.push(`/mcp-configuration/${config.id}`)}
                >
                  View →
                </Button>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
