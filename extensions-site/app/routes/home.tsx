import { Search } from "lucide-react";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { ServerCard } from "../components/server-card";
import { useState, useEffect } from "react";
import type { MCPServer } from "../types/server";
import { fetchMCPServers, searchMCPServers } from "../mcp-servers";
import { motion, AnimatePresence } from "framer-motion";

export default function HomePage() {
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

        setServers(results);
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : "Unknown error";
        setError(`Failed to load servers: ${errorMessage}`);
        console.error("Error loading servers:", err);
      } finally {
        setIsLoading(false);
      }
    };

    // Debounce all server loads
    const timeoutId = setTimeout(loadServers, 300);
    return () => clearTimeout(timeoutId);
  }, [searchQuery]);

  return (
    <div className="pb-24">
      <div className="pb-16">
        <h1 className="text-[64px] font-medium text-textProminent">
          Browse Extensions
        </h1>
        <p className="text-textProminent">
          Your central directory for discovering and installing extensions.
        </p>
      </div>

      <div className="relative">
        {/* <Search className="absolute left-3 top-3 h-4 w-4 text-gray-500" /> */}
        <Input
          className="pl-0"
          placeholder="Search for extensions"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      {error && (
        <div className="p-4 bg-red-50 text-red-600 rounded-md">{error}</div>
      )}

      <section className="pt-8">
        <div className={`${searchQuery ? "pb-2" : "pb-8"}`}>
          {/* <h2 className="text-xl">{searchQuery ? "Search Results" : ""}</h2> */}
          <p className="text-textSubtle">
            {searchQuery
              ? `${servers.length} result${
                  servers.length > 1 ? "s" : ""
                } for "${searchQuery}"`
              : ""}
          </p>
        </div>

        {isLoading ? (
          <div className="py-8 text-xl text-textSubtle">Loading servers...</div>
        ) : servers.length === 0 ? (
          <div className="text-center py-8 text-gray-500">
            {searchQuery
              ? "No servers found matching your search."
              : "No servers available."}
          </div>
        ) : (
          <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
            {/* <AnimatePresence> */}
            {servers
              .sort((a, b) => {
                // Sort built-in servers first
                if (a.is_builtin && !b.is_builtin) return -1;
                if (!a.is_builtin && b.is_builtin) return 1;
                return 0;
              })
              .map((server, index) => (
                <motion.div
                  key={server.id}
                  initial={{
                    opacity: 0,
                  }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{ duration: 0.6 }}
                >
                  <ServerCard key={server.id} server={server} />
                </motion.div>
              ))}
            {/* </AnimatePresence> */}
          </div>
        )}
      </section>
    </div>
  );
}
