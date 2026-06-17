import { Download, Terminal, Info } from "lucide-react";
import Link from "@docusaurus/Link";
import { useState } from "react";
import CodeBlock from '@theme/CodeBlock';
import { motion, AnimatePresence } from "framer-motion";
import { getGooseInstallLink } from "@site/src/utils/install-links";
import type { MCPServer } from "@site/src/types/server";
import type { Prompt, Extension } from "@site/src/types/prompt";

function extensionToMCPServer(extension: Extension): MCPServer {
  return {
    id: extension.command,
    name: extension.name,
    command: extension.command,
    url: extension.url,
    description: extension.name,
    is_builtin: extension.is_builtin,
    link: extension.link || '',
    installation_notes: extension.installation_notes || '',
    endorsed: false,
    environmentVariables: extension.environmentVariables || [],
  };
}

export function PromptCard({ prompt }: { prompt: Prompt }) {
  const [expandedExtension, setExpandedExtension] = useState<string | null>(null);

  return (
    <Link 
      to={`/prompt-library/detail?id=${prompt.id}`} 
      className="block no-underline hover:no-underline"
    >
      <div className="extension-title h-full">
        <div className="server-card interactive w-full h-full">
          <div className="card-glow"></div>
          <div className="prompt-card">
            <div className="card-header">
              <div className="card-header-content">
                <span className="home-page-server-name">
                  {prompt.title}
                </span>
              </div>
            </div>
            <div className="prompt-card-content">
              <div>
                <div>
                  <p className="card-description">{prompt.description}</p>
                </div>

                <div className="mt-6">
                  <div className="border-t border-borderSubtle pt-4">
                    <div className="flex flex-wrap gap-3">
                      {prompt.extensions.map((extension, index) => (
                        <div 
                          key={index}
                          className="flex flex-col"
                        >
                          <div 
                            className={`
                              inline-flex items-center h-9 px-4 rounded-full 
                              bg-background-subtle border border-borderSubtle
                              transition-all duration-150 ease-in-out
                              hover:bg-background-standard hover:border-borderStandard
                              group ${extension.is_builtin ? 'cursor-help' : 'cursor-pointer'}
                              ${expandedExtension === extension.command ? 'bg-background-standard border-borderStandard' : ''}
                            `}
                            onClick={(e) => {
                              e.preventDefault();
                              e.stopPropagation();
                              if (!extension.is_builtin) {
                                setExpandedExtension(expandedExtension === extension.command ? null : extension.command);
                              }
                            }}
                            title={extension.is_builtin ? "Built-in extension - can be enabled in settings" : "Click to see installation options"}
                          >
                            <span className="text-sm text-textStandard group-hover:text-textProminent">
                              {extension.name}
                            </span>
                            {extension.is_builtin ? (
                              <div className="inline-flex items-center ml-2">
                                <span className="text-sm text-textSubtle">
                                  Built-in
                                </span>
                              </div>
                            ) : (
                              <span className="ml-2 text-textSubtle">
                                <Download className="h-4 w-4" />
                              </span>
                            )}
                          </div>

                          {/* Inline Expansion */}
                          <AnimatePresence>
                            {!extension.is_builtin && expandedExtension === extension.command && (
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
                                    <span className="text-sm">Install via Desktop</span>
                                  </a>
                                  
                                  <div className="border-t border-borderSubtle" />
                                  
                                  <button
                                    className="command-toggle"
                                  >
                                    <Terminal className="h-4 w-4" />
                                    <h4 className="mx-2">Command</h4>
                                  </button>
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
                              </motion.div>
                            )}
                          </AnimatePresence>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Link>
  );
}