import { Input } from "../components/ui/input";
import { PromptCard } from "../components/prompt-card";
import { FilterPills } from "../components/filter-pills";
import { FilterSidebar } from "../components/filter-sidebar";
import { useState, useEffect } from "react";
import type { Prompt } from "../types/prompt";
import type { FilterCategories } from "../types/filters";
import { fetchPrompts, searchPrompts } from "../prompt-data";
import { fetchFilterCategories } from "../utils/filters";
import { motion } from "framer-motion";

// Define the fixed list of extensions
const AVAILABLE_EXTENSIONS = [
  "developer",
  "computer controller",
  "memory",
  "jetbrains",
  "git",
  "figma",
  "google drive",
  "google maps", 
  "tavily web search" 
];

const ITEMS_PER_PAGE = 6; // Show 6 cards per page (3x2 grid)

export default function HomePage() {
  const [prompts, setPrompts] = useState<Prompt[]>([]);
  const [filterCategories, setFilterCategories] = useState<FilterCategories | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [filters, setFilters] = useState({
    functions: [] as string[],
    extensions: [] as string[],
    verified: false
  });
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Get unique categories from prompts
  const categories = [...new Set(prompts.map(p => p.category))];

  // Load filter categories
  useEffect(() => {
    const loadFilters = async () => {
      try {
        const categories = await fetchFilterCategories();
        setFilterCategories(categories);
      } catch (err) {
        console.error("Error loading filter categories:", err);
      }
    };
    loadFilters();
  }, []);

  // Load prompts
  useEffect(() => {
    const loadPrompts = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const trimmedQuery = searchQuery.trim();
        const results = trimmedQuery
          ? await searchPrompts(trimmedQuery)
          : await fetchPrompts();

        setPrompts(results);
        setCurrentPage(1); // Reset to first page when search/filters change
      } catch (err) {
        const errorMessage =
          err instanceof Error ? err.message : "Unknown error";
        setError(`Failed to load prompts: ${errorMessage}`);
        console.error("Error loading prompts:", err);
      } finally {
        setIsLoading(false);
      }
    };

    const timeoutId = setTimeout(loadPrompts, 300);
    return () => clearTimeout(timeoutId);
  }, [searchQuery]);

  // Helper function to check if an extension matches
  const extensionMatches = (extension: string, filterExtension: string) => {
    // Handle special cases for extensions with spaces or different naming conventions
    switch (filterExtension.toLowerCase()) {
      case "computer controller":
        return extension.startsWith("computercontroller");
      case "google drive":
        return extension.startsWith("googledrive") || extension.startsWith("google_drive");
      case "google maps":
        return extension.startsWith("googlemaps") || extension.startsWith("google_maps");
      case "developer":
        return extension.startsWith("developer");
      case "tavily web search":
         return extension.startsWith("tavilywebsearch") || extension.startsWith("tavily_web_search");
      default:
        return extension.toLowerCase().includes(filterExtension.toLowerCase());
    }
  };

  // Filter prompts based on all criteria
  const filteredPrompts = prompts.filter(prompt => {
    const matchesCategory = !selectedCategory || prompt.category === selectedCategory;
    const matchesFunctions = filters.functions.length === 0 || filters.functions.includes(prompt.function);
    const matchesExtensions = filters.extensions.length === 0 || 
      prompt.extensions.some(extension => 
        filters.extensions.some(filterExt => extensionMatches(extension.toLowerCase(), filterExt))
      );
    const matchesVerified = !filters.verified || prompt.verified;
    const matchesSearch = !searchQuery || 
      prompt.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
      prompt.description.toLowerCase().includes(searchQuery.toLowerCase());

    return matchesCategory && matchesFunctions && matchesExtensions && matchesVerified && matchesSearch;
  });

  // Pagination calculations
  const totalPages = Math.ceil(filteredPrompts.length / ITEMS_PER_PAGE);
  const startIndex = (currentPage - 1) * ITEMS_PER_PAGE;
  const endIndex = startIndex + ITEMS_PER_PAGE;
  const currentPrompts = filteredPrompts.slice(startIndex, endIndex);

  // Handle filter changes
  const handleFilterChange = (type: keyof typeof filters, value: string) => {
    setFilters(prev => ({
      ...prev,
      [type]: prev[type].includes(value) 
        ? prev[type].filter(v => v !== value)
        : [...prev[type], value]
    }));
    setCurrentPage(1); // Reset to first page when filters change
  };

  // Pagination controls component
  const PaginationControls = () => {
    if (totalPages <= 1) return null;

    return (
      <div className="flex justify-center items-center gap-4 mt-8">
        <button
          onClick={() => setCurrentPage(prev => Math.max(prev - 1, 1))}
          disabled={currentPage === 1}
          className="px-4 py-2 rounded-md border border-border bg-surfaceHighlight hover:bg-surface text-textProminent disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Previous
        </button>
        <span className="text-textProminent">
          Page {currentPage} of {totalPages}
        </span>
        <button
          onClick={() => setCurrentPage(prev => Math.min(prev + 1, totalPages))}
          disabled={currentPage === totalPages}
          className="px-4 py-2 rounded-md border border-border bg-surfaceHighlight hover:bg-surface text-textProminent disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          Next
        </button>
      </div>
    );
  };

  return (
    <div className="pb-24">
      <div className="pb-16">
        <h1 className="text-[64px] font-medium text-textProminent">
          Prompt Library
        </h1>
        <p className="text-textProminent">
          Your central directory for discovering and using effective prompts with Goose.
        </p>
      </div>

      <div className="relative mb-6">
        <Input
          className="pl-0"
          placeholder="Search prompts by category, function, or keyword"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
      </div>

      <div className="mb-8">
        <FilterPills
          categories={categories}
          selectedCategory={selectedCategory}
          onSelectCategory={(category) => {
            setSelectedCategory(category);
            setCurrentPage(1); // Reset to first page when category changes
          }}
        />
      </div>

      <div className="flex gap-8">
        <FilterSidebar
          functions={filterCategories?.functions || []}
          selected={filters}
          onFilterChange={handleFilterChange}
          extensions={AVAILABLE_EXTENSIONS}
        />

        <div className="flex-1">
          {error && (
            <div className="p-4 bg-red-50 text-red-600 rounded-md">{error}</div>
          )}

          {isLoading ? (
            <div className="py-8 text-xl text-textSubtle">Loading prompts...</div>
          ) : filteredPrompts.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              {searchQuery
                ? "No prompts found matching your search."
                : "No prompts available."}
            </div>
          ) : (
            <>
              <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
                {currentPrompts.map((prompt) => (
                  <motion.div
                    key={prompt.id}
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.6 }}
                  >
                    <PromptCard prompt={prompt} />
                  </motion.div>
                ))}
              </div>
              <PaginationControls />
            </>
          )}
        </div>
      </div>
    </div>
  );
}