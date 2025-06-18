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

// 🎨 Assign a consistent color per extension name
function getColorClass(name: string): string {
  const colors = [
    "bg-purple-100 text-purple-800 border-purple-200",
    "bg-green-100 text-green-800 border-green-200",
    "bg-blue-100 text-blue-800 border-blue-200",
    "bg-yellow-100 text-yellow-800 border-yellow-200",
    "bg-pink-100 text-pink-800 border-pink-200",
    "bg-orange-100 text-orange-800 border-orange-200",
    "bg-teal-100 text-teal-800 border-teal-200",
    "bg-red-100 text-red-800 border-red-200",
    "bg-indigo-100 text-indigo-800 border-indigo-200",
    "bg-gray-100 text-gray-800 border-gray-200",
  ];

  let hash = 0;
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash);
  }
  const index = Math.abs(hash % colors.length);
  return colors[index];
}

export function RecipeCard({ recipe }: { recipe: Recipe }) {
  return (
    <Link
      to={`/recipes/detail?id=${recipe.id}`}
      className="block no-underline hover:no-underline h-full"
    >
      <div className="w-full h-full transition-shadow duration-200 ease-in-out hover:shadow-[0_0_0_2px_rgba(99,102,241,0.4),_0_4px_20px_rgba(99,102,241,0.1)] rounded-2xl border border-borderSubtle bg-white flex flex-col justify-between p-6">
        <div className="space-y-4">
          {/* Title & Description */}
          <div>
            <h3 className="font-semibold text-lg text-textProminent">
              {recipe.title}
            </h3>
            <p className="text-sm text-textStandard">{recipe.description}</p>
          </div>

         {/* Extensions */}
            <div className="flex flex-wrap gap-2">
            {recipe.extensions.map((ext, index) => {
                const cleanedLabel = ext.replace(/MCP/i, "").trim();
                return (
                <span
                    key={index}
                    className={`rounded-full px-3 py-1 text-sm border ${getColorClass(ext)}`}
                >
                    {cleanedLabel}
                </span>
                );
            })}
            </div>


          {/* Separator */}
          {recipe.activities?.length > 0 && (
            <div className="border-t border-borderSubtle my-2" />
          )}

          {/* Activities */}
          <div className="flex flex-wrap gap-2">
            {recipe.activities.map((activity, index) => (
              <span
                key={index}
                className="bg-surfaceHighlight border border-border rounded-full px-3 py-1 text-sm text-textProminent"
              >
                {activity}
              </span>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-between items-center pt-6">
          <a
            href={recipe.recipeUrl}
            className="text-sm font-medium text-purple-600 hover:underline"
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
              className="flex items-center gap-2 text-sm text-textSubtle hover:underline"
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
    </Link>
  );
}
