import React from "react";
import Link from "@docusaurus/Link";

export type Recipe = {
  id: string;
  title: string;
  description: string;
  extensions: string[];
  activities: string[];
  recipeUrl: string;
  action?: string;
  author?: string;
  persona?: string;
};

export function RecipeCard({ recipe }: { recipe: Recipe }) {
  return (
    <Link
      to={`/recipes/detail?id=${recipe.id}`}
      className="block no-underline hover:no-underline h-full"
    >
      <div className="relative w-full h-full">
        {/* Optional Glow */}
        <div className="absolute inset-0 rounded-2xl bg-purple-500 opacity-10 blur-2xl" />

        {/* Card Container */}
        <div className="relative z-10 w-full h-full rounded-2xl border border-zinc-200 dark:border-zinc-700 bg-white dark:bg-[#1A1A1A] flex flex-col justify-between p-6 transition-shadow duration-200 ease-in-out hover:shadow-[0_0_0_2px_rgba(99,102,241,0.4),_0_4px_20px_rgba(99,102,241,0.1)]">
          <div className="space-y-4">
            {/* Title & Description */}
            <div>
              <h3 className="font-semibold text-base text-zinc-900 dark:text-white leading-snug">
                {recipe.title}
              </h3>
              <p className="text-sm text-zinc-600 dark:text-zinc-400 mt-1">
                {recipe.description}
              </p>
            </div>

            {/* Extensions */}
            {recipe.extensions.length > 0 && (
              <div className="flex flex-wrap gap-2 mt-2">
                {recipe.extensions.map((ext, index) => {
                  const cleanedLabel = ext.replace(/MCP/i, "").trim();
                  return (
                    <span
                      key={index}
                      className="inline-flex items-center h-7 px-3 rounded-full 
                                 border border-zinc-300 bg-zinc-100 text-zinc-700 
                                 dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300 
                                 text-xs font-medium"
                    >
                      {cleanedLabel}
                    </span>
                  );
                })}
              </div>
            )}

            {/* Activities */}
            {recipe.activities?.length > 0 && (
              <div className="border-t border-zinc-200 dark:border-zinc-700 pt-2 mt-2 flex flex-wrap gap-2">
                {recipe.activities.map((activity, index) => (
                  <span
                    key={index}
                    className="inline-flex items-center h-7 px-3 rounded-full 
                               border border-zinc-300 bg-zinc-100 text-zinc-700 
                               dark:border-zinc-700 dark:bg-zinc-900 dark:text-zinc-300 
                               text-xs font-medium"
                  >
                    {activity}
                  </span>
                ))}
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="flex justify-between items-center pt-6 mt-2">
            <a
              href={recipe.recipeUrl}
              className="text-sm font-medium text-purple-600 hover:underline dark:text-purple-400"
              target="_blank"
              onClick={(e) => e.stopPropagation()}
            >
              Launch Recipe →
            </a>
            {recipe.author && (
              <a
                href={`https://github.com/${recipe.author}`}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-sm text-zinc-500 hover:underline dark:text-zinc-300"
                title="Recipe author"
                onClick={(e) => e.stopPropagation()}
              >
                <img
                  src={`https://github.com/${recipe.author}.png`}
                  alt={recipe.author}
                  className="w-5 h-5 rounded-full"
                />
                @{recipe.author}
              </a>
            )}
          </div>
        </div>
      </div>
    </Link>
  );
}
