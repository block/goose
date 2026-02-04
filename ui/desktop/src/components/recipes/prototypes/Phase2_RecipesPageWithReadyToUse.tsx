/**
 * Phase 2: Intent - Recipes Page with Ready to Use
 *
 * This is the evolved Recipes view that shows:
 * 1. "My Recipes" section - user's saved recipes
 * 2. "Ready to Use" section - pre-made templates they can use immediately
 * 3. [+ Create] button that leads to Phase 3 (Recipe Builder)
 *
 * Key differences from current RecipesView:
 * - Two distinct sections with headers
 * - Ready recipes are shown as cards user can start immediately
 * - Simplified action buttons (Start Chat, Edit, ⋮ menu)
 */

import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Play,
  Edit,
  Trash2,
  MoreVertical,
  Plus,
  FileText,
  Mail,
  Code,
  MessageSquare,
  Calendar,
  Sparkles,
} from 'lucide-react';

// Mock data for user's recipes
const mockMyRecipes = [
  {
    id: '1',
    title: 'Daily Standup Summary',
    description: 'Summarizes my daily standup notes and formats them for Slack',
    lastModified: '2 days ago',
    hasSchedule: true,
    slashCommand: '/standup',
  },
  {
    id: '2',
    title: 'Code Review Helper',
    description: 'Reviews code changes and suggests improvements',
    lastModified: '1 week ago',
    hasSchedule: false,
    slashCommand: null,
  },
];

// Mock data for ready-to-use templates
const mockReadyRecipes = [
  {
    id: 'ready-1',
    title: 'Email Writer',
    description: 'Professional email drafting assistant',
    icon: Mail,
    color: 'bg-blue-500',
  },
  {
    id: 'ready-2',
    title: 'Code Explainer',
    description: 'Explains code in simple terms',
    icon: Code,
    color: 'bg-green-500',
  },
  {
    id: 'ready-3',
    title: 'Meeting Notes',
    description: 'Organizes and summarizes meeting notes',
    icon: MessageSquare,
    color: 'bg-purple-500',
  },
  {
    id: 'ready-4',
    title: 'Task Planner',
    description: 'Breaks down tasks into actionable steps',
    icon: Calendar,
    color: 'bg-orange-500',
  },
];

interface MyRecipeCardProps {
  recipe: (typeof mockMyRecipes)[0];
  onStartChat: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

function MyRecipeCard({ recipe, onStartChat, onEdit, onDelete }: MyRecipeCardProps) {
  const [showMenu, setShowMenu] = useState(false);

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4 hover:border-gray-300 dark:hover:border-gray-600 transition-colors">
      <div className="flex justify-between items-start gap-4">
        <div className="flex-1 min-w-0">
          <h3 className="font-medium truncate">{recipe.title}</h3>
          <p className="text-sm text-gray-500 dark:text-gray-400 line-clamp-2 mt-1">
            {recipe.description}
          </p>
          <div className="flex items-center gap-3 mt-2 text-xs text-gray-400">
            <span>{recipe.lastModified}</span>
            {recipe.hasSchedule && (
              <span className="text-blue-500 flex items-center gap-1">
                <Calendar className="w-3 h-3" />
                Scheduled
              </span>
            )}
            {recipe.slashCommand && (
              <span className="text-purple-500 font-mono">{recipe.slashCommand}</span>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          {/* Primary: Start Chat */}
          <button
            onClick={onStartChat}
            className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-md flex items-center gap-1.5"
          >
            <Play className="w-3.5 h-3.5" />
            Start Chat
          </button>

          {/* Secondary: Edit */}
          <button
            onClick={onEdit}
            className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md"
            title="Edit"
          >
            <Edit className="w-4 h-4 text-gray-500" />
          </button>

          {/* More menu */}
          <div className="relative">
            <button
              onClick={() => setShowMenu(!showMenu)}
              className="p-1.5 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md"
            >
              <MoreVertical className="w-4 h-4 text-gray-500" />
            </button>

            {showMenu && (
              <>
                <div className="fixed inset-0" onClick={() => setShowMenu(false)} />
                <div className="absolute right-0 top-full mt-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-md shadow-lg py-1 z-10 min-w-[160px]">
                  <button
                    onClick={() => {
                      setShowMenu(false);
                      alert('Copy deeplink');
                    }}
                    className="w-full px-3 py-2 text-sm text-left hover:bg-gray-100 dark:hover:bg-gray-700"
                  >
                    Copy Deeplink
                  </button>
                  <button
                    onClick={() => {
                      setShowMenu(false);
                      alert('Copy YAML');
                    }}
                    className="w-full px-3 py-2 text-sm text-left hover:bg-gray-100 dark:hover:bg-gray-700"
                  >
                    Copy YAML
                  </button>
                  <button
                    onClick={() => {
                      setShowMenu(false);
                      alert('Export to file');
                    }}
                    className="w-full px-3 py-2 text-sm text-left hover:bg-gray-100 dark:hover:bg-gray-700"
                  >
                    Export to File
                  </button>
                  <div className="border-t border-gray-200 dark:border-gray-700 my-1" />
                  <button
                    onClick={() => {
                      setShowMenu(false);
                      onDelete();
                    }}
                    className="w-full px-3 py-2 text-sm text-left text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    Delete
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

interface ReadyRecipeCardProps {
  recipe: (typeof mockReadyRecipes)[0];
  onStartChat: () => void;
}

function ReadyRecipeCard({ recipe, onStartChat }: ReadyRecipeCardProps) {
  const Icon = recipe.icon;

  return (
    <div
      onClick={onStartChat}
      className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4 hover:border-blue-300 dark:hover:border-blue-600 hover:shadow-md transition-all cursor-pointer group"
    >
      <div className="flex items-start gap-3">
        <div className={`${recipe.color} p-2 rounded-lg`}>
          <Icon className="w-5 h-5 text-white" />
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-medium group-hover:text-blue-600 dark:group-hover:text-blue-400 transition-colors">
            {recipe.title}
          </h3>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">{recipe.description}</p>
        </div>
        <Play className="w-5 h-5 text-gray-300 group-hover:text-blue-500 transition-colors" />
      </div>
    </div>
  );
}

export default function Phase2_RecipesPageWithReadyToUse() {
  const navigate = useNavigate();

  const handleStartChat = (recipeName: string) => {
    alert(`Starting chat with recipe: ${recipeName}`);
  };

  const handleEdit = (recipeName: string) => {
    alert(`Opening editor for: ${recipeName}`);
  };

  const handleDelete = (recipeName: string) => {
    alert(`Delete recipe: ${recipeName}`);
  };

  const handleCreateNew = () => {
    alert('Opening Recipe Builder (Phase 3)...');
    // In real implementation: navigate('/prototype-phase3')
  };

  return (
    <div className="min-h-screen bg-gray-50 dark:bg-gray-900">
      {/* Header */}
      <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
        <div className="max-w-4xl mx-auto px-6 py-6">
          <div className="flex justify-between items-center">
            <div>
              <h1 className="text-2xl font-semibold">Recipes</h1>
              <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                Start with a ready recipe or create your own
              </p>
            </div>
            <button
              onClick={handleCreateNew}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md flex items-center gap-2"
            >
              <Sparkles className="w-4 h-4" />
              Create Recipe
            </button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="max-w-4xl mx-auto px-6 py-8 space-y-8">
        {/* My Recipes Section */}
        <section>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-medium flex items-center gap-2">
              <FileText className="w-5 h-5 text-gray-400" />
              My Recipes
            </h2>
            <span className="text-sm text-gray-400">{mockMyRecipes.length} recipes</span>
          </div>

          {mockMyRecipes.length === 0 ? (
            <div className="bg-white dark:bg-gray-800 rounded-lg border border-dashed border-gray-300 dark:border-gray-600 p-8 text-center">
              <FileText className="w-12 h-12 text-gray-300 mx-auto mb-3" />
              <p className="text-gray-500 dark:text-gray-400">No recipes yet</p>
              <p className="text-sm text-gray-400 mt-1">
                Create a recipe or try one from Ready to Use below
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {mockMyRecipes.map((recipe) => (
                <MyRecipeCard
                  key={recipe.id}
                  recipe={recipe}
                  onStartChat={() => handleStartChat(recipe.title)}
                  onEdit={() => handleEdit(recipe.title)}
                  onDelete={() => handleDelete(recipe.title)}
                />
              ))}
            </div>
          )}
        </section>

        {/* Ready to Use Section */}
        <section>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-medium flex items-center gap-2">
              <Sparkles className="w-5 h-5 text-purple-500" />
              Ready to Use
            </h2>
            <span className="text-sm text-gray-400">Click to start</span>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            {mockReadyRecipes.map((recipe) => (
              <ReadyRecipeCard
                key={recipe.id}
                recipe={recipe}
                onStartChat={() => handleStartChat(recipe.title)}
              />
            ))}
          </div>
        </section>
      </div>

      {/* Back button */}
      <div className="fixed bottom-6 left-6">
        <button
          onClick={() => navigate('/recipes')}
          className="px-4 py-2 bg-gray-800 text-white rounded-md hover:bg-gray-700"
        >
          ← Back to Recipes
        </button>
      </div>
    </div>
  );
}

/**
 * Demo wrapper
 */
export function Phase2_Demo() {
  return <Phase2_RecipesPageWithReadyToUse />;
}
