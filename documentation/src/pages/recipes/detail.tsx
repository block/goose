import Layout from "@theme/Layout";
import { ArrowLeft } from "lucide-react";
import { useLocation } from "@docusaurus/router";
import { useEffect, useState } from "react";
import Link from "@docusaurus/Link";
import Admonition from "@theme/Admonition";
import CodeBlock from "@theme/CodeBlock";
import { Button } from "@site/src/components/ui/button";
import { getRecipeById } from "@site/src/utils/recipes";
import type { Recipe } from "@site/src/components/recipe-card";

const colorMap: { [key: string]: string } = {
  "GitHub MCP": "bg-yellow-100 text-yellow-800 border-yellow-200",
  "Context7 MCP": "bg-purple-100 text-purple-800 border-purple-200",
  "Memory": "bg-blue-100 text-blue-800 border-blue-200",
};

export default function RecipeDetailPage(): JSX.Element {
  const location = useLocation();
  const [recipe, setRecipe] = useState<Recipe | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadRecipe = async () => {
      try {
        setLoading(true);
        setError(null);

        const params = new URLSearchParams(location.search);
        const id = params.get("id");
        if (!id) {
          setError("No recipe ID provided");
          return;
        }

        const recipeData = await getRecipeById(id);
        if (recipeData) {
          setRecipe(recipeData);
        } else {
          setError("Recipe not found");
        }
      } catch (err) {
        setError("Failed to load recipe details");
        console.error(err);
      } finally {
        setLoading(false);
      }
    };

    loadRecipe();
  }, [location]);

  if (loading) {
    return (
      <Layout>
        <div className="min-h-screen flex items-start justify-center py-16">
          <div className="container max-w-5xl mx-auto px-4 animate-pulse">
            <div className="h-12 w-48 bg-bgSubtle dark:bg-zinc-800 rounded-lg mb-4"></div>
            <div className="h-6 w-full bg-bgSubtle dark:bg-zinc-800 rounded-lg mb-2"></div>
            <div className="h-6 w-2/3 bg-bgSubtle dark:bg-zinc-800 rounded-lg"></div>
          </div>
        </div>
      </Layout>
    );
  }

  if (error || !recipe) {
    return (
      <Layout>
        <div className="min-h-screen flex items-start justify-center py-16">
          <div className="container max-w-5xl mx-auto px-4 text-red-500">
            {error || "Recipe not found"}
          </div>
        </div>
      </Layout>
    );
  }

  return (
    <Layout>
      <div className="min-h-screen py-12">
        <div className="max-w-4xl mx-auto px-4">
          <div className="mb-8 flex justify-between items-start">
            <Link to="/recipes">
              <Button className="flex items-center gap-2">
                <ArrowLeft className="h-4 w-4" />
                Back
              </Button>
            </Link>
            {recipe.author && (
              <a
                href={`https://github.com/${recipe.author}`}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-sm text-textSubtle hover:underline"
              >
                <img
                  src={`https://github.com/${recipe.author}.png`}
                  alt={recipe.author}
                  className="w-6 h-6 rounded-full"
                />
                @{recipe.author}
              </a>
            )}
          </div>

          <div className="bg-white dark:bg-[#1A1A1A] border border-borderSubtle dark:border-zinc-700 rounded-xl p-8 shadow-md">
            <h1 className="text-4xl font-semibold mb-2 text-textProminent dark:text-white">
              {recipe.title}
            </h1>
            <p className="text-textSubtle dark:text-zinc-400 text-lg mb-6">{recipe.description}</p>

            {/* Activities */}
            {recipe.activities?.length > 0 && (
              <div className="mb-6 border-t border-borderSubtle dark:border-zinc-700 pt-6">
                <h2 className="text-2xl font-medium mb-2 text-textProminent dark:text-white">Activities</h2>
                <div className="flex flex-wrap gap-2">
                  {recipe.activities.map((activity, index) => (
                    <span
                      key={index}
                      className="bg-surfaceHighlight dark:bg-zinc-900 border border-border dark:border-zinc-700 rounded-full px-3 py-1 text-sm text-textProminent dark:text-zinc-300"
                    >
                      {activity}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {/* Extensions */}
            {recipe.extensions?.length > 0 && (
              <div className="mb-6 border-t border-borderSubtle dark:border-zinc-700 pt-6">
                <h2 className="text-2xl font-medium mb-2 text-textProminent dark:text-white">Extensions</h2>
                <div className="flex flex-wrap gap-2">
                  {recipe.extensions.map((ext, index) => (
                    <span
                      key={index}
                      className={`border rounded-full px-3 py-1 text-sm ${
                        colorMap[ext] || 
                        "bg-gray-100 text-gray-800 border-gray-200 dark:bg-zinc-900 dark:text-zinc-300 dark:border-zinc-700"
                      }`}
                    >
                      {ext}
                    </span>
                  ))}
                </div>
              </div>
            )}

            {/* Instructions */}
            {recipe.instructions && (
              <div className="mb-6 border-t border-borderSubtle dark:border-zinc-700 pt-6">
                <h2 className="text-2xl font-medium mb-2 text-textProminent dark:text-white">Instructions</h2>
                <p className="text-textSubtle dark:text-zinc-400 whitespace-pre-line">
                  {recipe.instructions}
                </p>
              </div>
            )}

            {/* Prompt */}
            {recipe.prompt && (
              <div className="mb-6 border-t border-borderSubtle dark:border-zinc-700 pt-6">
                <h2 className="text-2xl font-medium mb-4 text-textProminent dark:text-white">Initial Prompt</h2>
                <Admonition type="info" className="mb-4">
                  This prompt auto-starts the recipe when launched in Goose.
                </Admonition>
                <CodeBlock language="markdown">{recipe.prompt}</CodeBlock>
              </div>
            )}

            {/* Launch Button */}
            {recipe.recipeUrl && (
              <div className="pt-8 border-t border-borderSubtle dark:border-zinc-700 mt-6">
                <Link
                  to={recipe.recipeUrl}
                  target="_blank"
                  className="inline-block text-white bg-black dark:bg-white dark:text-black px-6 py-2 rounded-full text-sm font-medium hover:bg-gray-900 dark:hover:bg-gray-100 transition-colors"
                >
                  Launch Recipe →
                </Link>
              </div>
            )}
          </div>
        </div>
      </div>
    </Layout>
  );
}
