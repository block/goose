import { ServerCard } from "@site/src/components/server-card";
import { useState, useEffect } from "react";
import type { MCPServer } from "@site/src/types/server";
import { fetchMCPServers, searchMCPServers } from "@site/src/utils/mcp-servers";
import { motion } from "framer-motion";
import Layout from "@theme/Layout";
import Link from "@docusaurus/Link";
import { Wand2, Book, ArrowLeft } from "lucide-react";

export default function MCPServersPage() {
  const [servers, setServers] = useState<MCPServer[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Combined effect for initial load and search
  useEffect(() => {
    const loadServers = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const trimmedQuery = searchQuery.trim();
        const results = trimmedQuery
          ? await searchMCPServers(trimmedQuery)
          : await fetchMCPServers();

        // Filter to only show MCP servers (both built-in and external MCP servers)
        const mcpServers = results.filter(server => {
          // Include all servers since they're all MCP servers, but we could filter further if needed
          return true;
        });

        console.log("Loaded MCP servers:", mcpServers);
        setServers(mcpServers);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : "Unknown error";
        setError(`Failed to load MCP servers: ${errorMessage}`);
        console.error("Error loading MCP servers:", err);
      } finally {
        setIsLoading(false);
      }
    };

    // Debounce all server loads
    const timeoutId = setTimeout(loadServers, 300);
    return () => clearTimeout(timeoutId);
  }, [searchQuery]);

  // Separate built-in and external servers
  const builtInServers = servers.filter(server => server.is_builtin);
  const externalServers = servers.filter(server => !server.is_builtin);

  return (
    <Layout
      title="MCP Servers"
      description="Discover and install Model Context Protocol (MCP) servers to extend Goose's capabilities"
    >
      <div className="container mx-auto px-4 p-24">
        <div className="pb-16">
          <div className="flex items-center gap-3 mb-4">
            <Link to="/docs/category/getting-started" className="text-textSubtle hover:text-textProminent transition-colors no-underline">
              <ArrowLeft className="h-5 w-5" />
            </Link>
            <h1 className="text-[64px] font-medium text-textProminent m-0">
              MCP Servers
            </h1>
          </div>
          <p className="text-textProminent text-lg">
            Model Context Protocol (MCP) servers that extend Goose's capabilities. 
            Install these extensions to connect Goose to external services and tools.
          </p>
        </div>

        <div className="flex justify-between items-center mb-8">
          <div className="search-container flex-1">
            <input
              className="bg-bgApp font-light text-textProminent placeholder-textPlaceholder w-full px-3 py-3 text-[40px] leading-[52px] border-b border-borderSubtle focus:outline-none focus:ring-purple-500 focus:border-borderProminent caret-[#FF4F00] pl-0"
              placeholder="Search MCP servers"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
            />
          </div>
          <div className="flex gap-3 ml-4">
            <Link to="/extensions" className="no-underline">
              <div className="flex items-center gap-2 bg-bgAppInverse text-textProminentInverse px-4 py-3 rounded-lg hover:bg-opacity-90 transition-all">
                <span>All Extensions</span>
              </div>
            </Link>
            <Link to="/recipe-generator" className="no-underline">
              <div className="flex items-center gap-2 bg-bgAppInverse text-textProminentInverse px-4 py-3 rounded-lg hover:bg-opacity-90 transition-all">
                <Wand2 className="h-5 w-5" />
                <span>Recipe Generator</span>
              </div>
            </Link>
          </div>
        </div>

        {error && (
          <div className="p-4 bg-red-50 text-red-600 rounded-md mb-8">{error}</div>
        )}

        <div className={`${searchQuery ? "pb-2" : "pb-8"}`}>
          <p className="text-gray-600">
            {searchQuery
              ? `${servers.length} result${
                  servers.length > 1 ? "s" : ""
                } for "${searchQuery}"`
              : `${servers.length} MCP servers available`}
          </p>
        </div>

        {isLoading ? (
          <div className="py-8 text-xl text-gray-600">Loading MCP servers...</div>
        ) : servers.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            {searchQuery
              ? "No MCP servers found matching your search."
              : "No MCP servers available."}
          </div>
        ) : (
          <>
            {/* Built-in MCP Servers Section */}
            {builtInServers.length > 0 && !searchQuery && (
              <section className="mb-12">
                <h2 className="text-2xl font-semibold text-textProminent mb-6">
                  Built-in MCP Extensions
                </h2>
                <div className="cards-grid">
                  {builtInServers.map((server) => (
                    <motion.div
                      key={server.id}
                      initial={{ opacity: 0 }}
                      animate={{ opacity: 1 }}
                      exit={{ opacity: 0 }}
                      transition={{ duration: 0.6 }}
                    >
                      <ServerCard server={server} />
                    </motion.div>
                  ))}
                </div>
              </section>
            )}

            {/* External MCP Servers Section */}
            {externalServers.length > 0 && (
              <section>
                {!searchQuery && builtInServers.length > 0 && (
                  <h2 className="text-2xl font-semibold text-textProminent mb-6">
                    Community MCP Servers
                  </h2>
                )}
                <div className="cards-grid">
                  {(searchQuery ? servers : externalServers)
                    .sort((a, b) => {
                      // Sort endorsed servers first, then alphabetically
                      if (a.endorsed && !b.endorsed) return -1;
                      if (!a.endorsed && b.endorsed) return 1;
                      return a.name.localeCompare(b.name);
                    })
                    .map((server) => (
                      <motion.div
                        key={server.id}
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 0.6 }}
                      >
                        <ServerCard server={server} />
                      </motion.div>
                    ))}
                </div>
              </section>
            )}
          </>
        )}

        {/* Documentation Links */}
        <div className="mt-16 p-6 bg-gray-50 dark:bg-gray-800 rounded-lg">
          <h3 className="text-lg font-semibold mb-4 flex items-center gap-2">
            <Book className="h-5 w-5" />
            Learn More About MCP Servers
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <Link to="/docs/tutorials/custom-extensions" className="block p-4 bg-white dark:bg-gray-700 rounded border hover:shadow-md transition-shadow no-underline">
              <h4 className="font-medium text-textProminent mb-2">Create Custom Extensions</h4>
              <p className="text-sm text-textSubtle">Learn how to build your own MCP servers</p>
            </Link>
            <Link to="/docs/getting-started/using-extensions" className="block p-4 bg-white dark:bg-gray-700 rounded border hover:shadow-md transition-shadow no-underline">
              <h4 className="font-medium text-textProminent mb-2">Using Extensions</h4>
              <p className="text-sm text-textSubtle">Guide to installing and configuring extensions</p>
            </Link>
          </div>
        </div>
      </div>
    </Layout>
  );
}
