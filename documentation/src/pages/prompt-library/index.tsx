import { PromptCard } from "@site/src/components/prompt-card";
import { useState, useEffect } from "react";
import { motion } from "framer-motion";
import Layout from "@theme/Layout";
import Admonition from '@theme/Admonition';
import type { Prompt, Category } from "@site/src/types/prompt";
import { searchPrompts } from "@site/src/utils/prompts";
import { Button } from "@site/src/components/ui/button";
import { PillFilter, type PillFilterOption } from "@site/src/components/ui/pill-filter";
import { SidebarFilter, type SidebarFilterGroup } from "@site/src/components/ui/sidebar-filter";
import { Menu, X } from 'lucide-react';

const categoryOptions: PillFilterOption[] = [
  { label: "All", value: "all" },
  { label: "Business", value: "business" },
  { label: "Technical", value: "technical" },
  { label: "Productivity", value: "productivity" },
];

const sidebarFilterGroups: SidebarFilterGroup[] = [
  {
    title: "Job",
    options: [
      { label: "Engineering", value: "engineering" },
      { label: "Design", value: "design" },
      { label: "Project Management", value: "project-management" },
      { label: "Marketing/Content", value: "marketing-content" },
      { label: "Data Analytics", value: "data-analytics" },
      { label: "Operations", value: "operations" }
    ],
  }
];

export default function HomePage() {
  const [prompts, setPrompts] = useState<Prompt[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [selectedCategory, setSelectedCategory] = useState("all");
  const [selectedFilters, setSelectedFilters] = useState<Record<string, string[]>>({});
  const [isMobileFilterOpen, setIsMobileFilterOpen] = useState(false);
  const promptsPerPage = 20;

  useEffect(() => {
    const loadPrompts = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const results = await searchPrompts(searchQuery);
        
        // Filter results based on category
        let filteredResults = results;
        if (selectedCategory !== "all") {
          filteredResults = results.filter(prompt => 
            prompt.category === selectedCategory
          );
        }

        // Apply job filters
        Object.entries(selectedFilters).forEach(([group, values]) => {
          if (values.length > 0) {
            filteredResults = filteredResults.filter(prompt => {
              if (group === "Job") {
                return values.includes(prompt.job);
              }
              return true;
            });
          }
        });

        setPrompts(filteredResults);
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
  }, [searchQuery, selectedCategory, selectedFilters]);

  // Close mobile filter when screen size changes to desktop
  useEffect(() => {
    const handleResize = () => {
      if (window.innerWidth >= 768) {
        setIsMobileFilterOpen(false);
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  return (
    <Layout>
      <div className="container mx-auto px-4 py-8 md:p-24">
        <div className="pb-8 md:pb-16">
          <h1 className="text-4xl md:text-[64px] font-medium text-textProminent">
            Prompt Library
          </h1>
          <p className="text-textProminent">
            Your central directory for discovering and using effective prompts with Goose.
          </p>
        </div>

        <div className="search-container mb-6 md:mb-8">
          <input
            className="bg-bgApp font-light text-textProminent placeholder-textPlaceholder w-full px-3 py-2 md:py-3 text-2xl md:text-[40px] leading-tight md:leading-[52px] border-b border-borderSubtle focus:outline-none focus:ring-purple-500 focus:border-borderProminent caret-[#FF4F00] pl-0"
            placeholder="Search for prompts by keywords"
            value={searchQuery}
            onChange={(e) => {
              setSearchQuery(e.target.value);
              setCurrentPage(1);
            }}
          />
        </div>

        <div className="mb-6 md:mb-8 overflow-x-auto">
          <PillFilter
            options={categoryOptions}
            selectedValue={selectedCategory}
            onChange={(category) => {
              setSelectedCategory(category);
              setCurrentPage(1);
            }}
          />
        </div>

        {error && (
          <Admonition type="danger" title="Error">
            <p>{error}</p>
          </Admonition>
        )}

        {/* Mobile Filter Toggle Button */}
        <div className="md:hidden mb-4">
          <Button
            onClick={() => setIsMobileFilterOpen(!isMobileFilterOpen)}
            className="w-full flex items-center justify-center gap-2 py-2 px-4 bg-surfaceHighlight text-textProminent"
          >
            {isMobileFilterOpen ? <X size={20} /> : <Menu size={20} />}
            {isMobileFilterOpen ? 'Close Filters' : 'Show Filters'}
          </Button>
        </div>

        <div className="flex flex-col md:flex-row gap-8">
          {/* Mobile Sidebar Filter */}
          <div
            className={`${
              isMobileFilterOpen ? 'block' : 'hidden'
            } md:block md:w-64 flex-shrink-0 transition-all duration-300 ease-in-out`}
          >
            <SidebarFilter
              groups={sidebarFilterGroups}
              selectedValues={selectedFilters}
              onChange={(groupTitle, values) => {
                setSelectedFilters(prev => ({
                  ...prev,
                  [groupTitle]: values
                }));
                setCurrentPage(1);
              }}
            />
          </div>

          <div className="flex-1">
            <div className={`${searchQuery ? "pb-2" : "pb-4 md:pb-8"}`}>
              <p className="text-gray-600">
                {searchQuery
                  ? `${prompts.length} result${prompts.length > 1 ? "s" : ""
                  } for "${searchQuery}"`
                  : ""}
              </p>
            </div>

            {isLoading ? (
              <div className="py-8 text-xl text-gray-600">Loading prompts...</div>
            ) : prompts.length === 0 ? (
              <Admonition type="info">
                <p>
                  {searchQuery
                    ? "No prompts found matching your search."
                    : "No prompts available in the library yet."}
                </p>
              </Admonition>
            ) : (
              <>
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 md:gap-6">
                  {prompts
                    .slice((currentPage - 1) * promptsPerPage, currentPage * promptsPerPage)
                    .map((prompt) => (
                      <motion.div
                        key={prompt.id}
                        initial={{
                          opacity: 0,
                        }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 0.6 }}
                      >
                        <PromptCard key={prompt.id} prompt={prompt} />
                      </motion.div>
                    ))}
                </div>

                {/* Pagination Controls */}
                {prompts.length > promptsPerPage && (
                  <div className="flex justify-center items-center gap-2 md:gap-4 mt-6 md:mt-8">
                    <Button
                      onClick={() => setCurrentPage(prev => Math.max(prev - 1, 1))}
                      disabled={currentPage === 1}
                      className="px-3 md:px-4 py-2 rounded-md border border-border bg-surfaceHighlight hover:bg-surface text-textProminent disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm md:text-base"
                    >
                      Previous
                    </Button>

                    <span className="text-textProminent text-sm md:text-base">
                      Page {currentPage} of {Math.ceil(prompts.length / promptsPerPage)}
                    </span>

                    <Button
                      onClick={() => setCurrentPage(prev => Math.min(Math.ceil(prompts.length / promptsPerPage), prev + 1))}
                      disabled={currentPage >= Math.ceil(prompts.length / promptsPerPage)}
                      className="px-3 md:px-4 py-2 rounded-md border border-border bg-surfaceHighlight hover:bg-surface text-textProminent disabled:opacity-50 disabled:cursor-not-allowed transition-colors text-sm md:text-base"
                    >
                      Next
                    </Button>
                  </div>
                )}
              </>
            )}
          </div>
        </div>
      </div>
    </Layout>
  );
}