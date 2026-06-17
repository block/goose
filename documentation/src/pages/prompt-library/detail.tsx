import Layout from "@theme/Layout";
import { ArrowLeft, Terminal, Download, Code } from "lucide-react";
import Admonition from "@theme/Admonition";
import CodeBlock from "@theme/CodeBlock";
import { Button } from "@site/src/components/ui/button";
import { Badge } from "@site/src/components/ui/badge";
import { useLocation } from "@docusaurus/router";
import { useEffect, useState } from "react";
import Link from "@docusaurus/Link";
import { motion, AnimatePresence } from "framer-motion";
import type { MCPServer } from "@site/src/types/server";
import { getPromptById } from "@site/src/utils/prompts";
import ReactMarkdown from "react-markdown";

import type { Prompt, Extension } from "@site/src/types/prompt";
import { getGooseInstallLink } from "@site/src/utils/install-links";

function extensionToMCPServer(extension: Extension): MCPServer {
  return {
    id: extension.command,
    name: extension.name,
    command: extension.command,
    url: extension.url,
    description: extension.name,
    is_builtin: extension.is_builtin,
    link: extension.link || "",
    installation_notes: extension.installation_notes || "",
    endorsed: false,
    environmentVariables: extension.environmentVariables || [],
  };
}

function ExtensionList({ extensions }: { extensions: Extension[] }) {
  const [expandedExtension, setExpandedExtension] = useState<string | null>(
    null
  );

  const hasExpandedExtension = expandedExtension !== null;

  return (
    <div
      className={`flex gap-3 ${
        hasExpandedExtension ? "flex-col" : "flex-wrap"
      }`}
    >
      {extensions.map((extension) => (
        <ExtensionDetails
          key={extension.name}
          extension={extension}
          isExpanded={expandedExtension === extension.name}
          onToggle={(expanded) => {
            setExpandedExtension(expanded ? extension.name : null);
          }}
        />
      ))}
    </div>
  );
}

function ExtensionDetails({
  extension,
  isExpanded,
  onToggle,
}: {
  extension: Extension;
  isExpanded: boolean;
  onToggle: (expanded: boolean) => void;
}) {
  return (
    <div className="flex flex-col">
      <div
        className={`
          inline-flex items-center h-9 px-4 rounded-full 
          bg-background-subtle border border-borderSubtle
          transition-all duration-150 ease-in-out
          hover:bg-background-standard hover:border-borderStandard
          group ${extension.is_builtin ? "cursor-help" : "cursor-pointer"}
          ${isExpanded ? "bg-background-standard border-borderStandard" : ""}
        `}
        onClick={() => {
          if (!extension.is_builtin) {
            onToggle(!isExpanded);
          }
        }}
        title={
          extension.is_builtin
            ? "Built-in extension - can be enabled in settings"
            : "Click to see installation options"
        }
      >
        <span className="text-sm text-textStandard group-hover:text-textProminent">
          {extension.name}
        </span>
        {extension.is_builtin ? (
          <div className="inline-flex items-center ml-2">
            <span className="text-sm text-textSubtle">Built-in</span>
          </div>
        ) : (
          <span className="ml-2 text-textSubtle">
            <Download className="h-4 w-4" />
          </span>
        )}
      </div>

      {/* Inline Expansion */}
      <AnimatePresence>
        {!extension.is_builtin && isExpanded && (
          <motion.div
            initial={{ height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={{ height: 0, opacity: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            <div className="mt-2 bg-background-subtle rounded-md p-3 border border-borderSubtle space-y-3">
              <a
                href={getGooseInstallLink(extensionToMCPServer(extension))}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-textStandard hover:text-textProminent"
                onClick={(e) => e.stopPropagation()}
              >
                <Download className="h-4 w-4" />
                <span className="mx-2">Install via Desktop</span>
              </a>

              <div className="border-t border-borderSubtle" />
              <button className="command-toggle w-full flex items-center">
                <Terminal className="h-4 w-4" />
                <h4 className="mx-2">Command</h4>
              </button>
              <div>
                {extension.url ? (
                  <CodeBlock language="bash">
                    goose session --with-remote-extension "{extension.url}"
                  </CodeBlock>
                ) : (
                  <CodeBlock language="bash">
                    goose session --with-extension "{extension.command}"
                  </CodeBlock>
                )}
              </div>

              {extension.installation_notes && (
                <>
                  <div className="border-t border-borderSubtle" />
                  <div>
                    <div className="text-sm font-medium mb-2">
                      Installation Notes
                    </div>
                    <div className="text-sm text-textSubtle">
                      <ReactMarkdown>
                        {extension.installation_notes}
                      </ReactMarkdown>
                    </div>
                  </div>
                </>
              )}

              {extension.environmentVariables &&
                extension.environmentVariables.length > 0 && (
                  <>
                    <div className="border-t border-borderSubtle" />
                    <div>
                      <div className="text-sm font-medium mb-2">
                        Environment Variables
                      </div>
                      {extension.environmentVariables.map((env) => (
                        <div key={env.name} className="mb-2 last:mb-0">
                          <code className="text-sm">{env.name}</code>
                          <div className="text-sm text-textSubtle mt-1">
                            {env.description}
                            {env.required && (
                              <Badge
                                variant="secondary"
                                className="ml-2 text-xs"
                              >
                                Required
                              </Badge>
                            )}
                          </div>
                        </div>
                      ))}
                    </div>
                  </>
                )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function PromptDetail({ prompt }: { prompt: Prompt }) {
  return (
    <Layout>
      <div className="min-h-screen flex items-start justify-center py-16">
        <div className="container max-w-5xl mx-auto px-4">
          <div className="flex gap-8">
            <div>
              <Link to="/prompt-library" className="no-underline">
                <Button className="flex items-center gap-2 hover:cursor-pointer">
                  <ArrowLeft className="h-4 w-4" />
                  Back
                </Button>
              </Link>
            </div>

            <div className="prompt-card flex-1">
              <div className="card p-8">
                <div className="card-header mb-6">
                  <h1 className="font-medium text-5xl text-textProminent m-0">
                    {prompt.title}
                  </h1>
                </div>

                <div className="card-content space-y-8">
                  <div>
                    <p className="text-xl text-textSubtle m-0">
                      {prompt.description}
                    </p>
                  </div>

                  <Admonition type="info">
                    Results may vary depending on the model and context.
                  </Admonition>

                  <div>
                    <h2 className="text-2xl font-medium mb-4">
                      Example Prompt
                    </h2>
                    <CodeBlock language="markdown">
                      {prompt.example_prompt}
                    </CodeBlock>
                  </div>

                  {prompt.example_result && (
                    <div>
                      <h2 className="text-2xl font-medium mb-4">
                        Example Result
                      </h2>
                      <CodeBlock language="markdown">
                        {prompt.example_result}
                      </CodeBlock>
                    </div>
                  )}

                  <div>
                    <h2 className="text-2xl font-medium mb-4">
                      Recommended Extensions
                    </h2>
                    <ExtensionList extensions={prompt.extensions} />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Layout>
  );
}

export default function DetailPage(): JSX.Element {
  const location = useLocation();
  const [prompt, setPrompt] = useState<Prompt | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadPrompt = async () => {
      try {
        setLoading(true);
        setError(null);

        // Get the ID from the query parameter
        const params = new URLSearchParams(location.search);
        const id = params.get("id");
        if (!id) {
          setError("No prompt ID provided");
          return;
        }

        const promptData = await getPromptById(id);
        if (promptData) {
          setPrompt(promptData);
        } else {
          setError("Prompt not found");
        }
      } catch (err) {
        setError("Failed to load prompt details");
        console.error(err);
      } finally {
        setLoading(false);
      }
    };

    loadPrompt();
  }, [location]);

  if (loading) {
    return (
      <Layout>
        <div className="min-h-screen flex items-start justify-center py-16">
          <div className="container max-w-5xl mx-auto px-4">
            <div className="flex gap-8">
              <div>
                <Link to="/prompt-library" className="no-underline">
                  <Button className="flex items-center gap-2 hover:text-textProminent hover:cursor-pointer">
                    <ArrowLeft className="h-4 w-4" />
                    Back
                  </Button>
                </Link>
              </div>
              <div className="prompt-card flex-1">
                <div className="card p-8">
                  <div className="animate-pulse">
                    <div className="h-12 w-48 bg-bgSubtle rounded-lg mb-4"></div>
                    <div className="h-6 w-full bg-bgSubtle rounded-lg mb-2"></div>
                    <div className="h-6 w-2/3 bg-bgSubtle rounded-lg"></div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Layout>
    );
  }

  if (error || !prompt) {
    return (
      <Layout>
        <div className="min-h-screen flex items-start justify-center py-16">
          <div className="container max-w-5xl mx-auto px-4">
            <div className="flex gap-8">
              <div>
                <Link to="/prompt-library" className="no-underline">
                  <Button
                    variant="ghost"
                    className="flex items-center gap-2 hover:text-textProminent hover:cursor-pointer"
                  >
                    <ArrowLeft className="h-4 w-4" />
                    Back
                  </Button>
                </Link>
              </div>
              <div className="prompt-card flex-1">
                <div className="card p-8">
                  <div className="text-red-500">
                    {error || "Prompt not found"}
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Layout>
    );
  }

  return <PromptDetail prompt={prompt} />;
}
