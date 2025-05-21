import React, { useState, useCallback, useMemo } from 'react';
import Layout from "@theme/Layout";
import { ArrowLeft, Copy, Check, Plus, X } from "lucide-react";
import { Button } from "@site/src/components/ui/button";
import Link from "@docusaurus/Link";

export default function RecipeGenerator() {
  // State management
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [instructions, setInstructions] = useState('');
  const [activities, setActivities] = useState([]);
  const [newActivity, setNewActivity] = useState('');
  const [prompt, setPrompt] = useState('');
  const [copied, setCopied] = useState(false);
  const [errors, setErrors] = useState({});

  // Add activity handler
  const handleAddActivity = useCallback(() => {
    if (newActivity.trim()) {
      setActivities(prev => [...prev, newActivity.trim()]);
      setNewActivity('');
    }
  }, [newActivity]);

  // Remove activity handler
  const handleRemoveActivity = useCallback((index) => {
    setActivities(prev => prev.filter((_, i) => i !== index));
  }, []);

  // Form validation
  const validateForm = useCallback(() => {
    const newErrors = {};
    
    if (!title.trim()) {
      newErrors.title = 'Title is required';
    }
    if (!description.trim()) {
      newErrors.description = 'Description is required';
    }
    if (!instructions.trim()) {
      newErrors.instructions = 'Instructions are required';
    }
    
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  }, [title, description, instructions]);

  // Generate URL with useMemo to prevent re-renders
  const recipeUrl = useMemo(() => {
    // Only generate if we have the required fields
    if (!title.trim() || !description.trim() || !instructions.trim()) {
      return '';
    }

    try {
      const recipeConfig = {
        version: "1.0.0",
        title,
        description,
        instructions,
        prompt: prompt || undefined,
        activities: activities.length > 0 ? activities : undefined
      };

      // Filter out undefined values
      Object.keys(recipeConfig).forEach(key => {
        if (recipeConfig[key] === undefined) {
          delete recipeConfig[key];
        }
      });

      // Use window.btoa for browser compatibility
      return `goose://recipe?config=${window.btoa(JSON.stringify(recipeConfig))}`;
    } catch (error) {
      console.error('Error generating recipe URL:', error);
      return '';
    }
  }, [title, description, instructions, prompt, activities]);

  // Copy handler
  const handleCopy = useCallback(() => {
    if (validateForm() && recipeUrl) {
      navigator.clipboard.writeText(recipeUrl)
        .then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        })
        .catch(err => console.error('Failed to copy URL:', err));
    }
  }, [validateForm, recipeUrl]);

  return (
    <Layout>
      <div className="container mx-auto px-4 py-12 max-w-4xl">
        <div className="mb-6">
          <Link to="/extensions" className="no-underline">
            <Button className="flex items-center gap-2">
              <ArrowLeft className="h-4 w-4" />
              Back to Extensions
            </Button>
          </Link>
        </div>

        <div className="mb-8">
          <h1 className="text-4xl font-bold mb-4">Recipe Generator</h1>
          <p className="text-lg">
            Create a shareable Goose recipe URL that others can use to launch a session with your predefined settings.
          </p>
        </div>

        <div className="bg-white border rounded-lg p-6 mb-8 shadow-sm">
          <h2 className="text-2xl font-medium mb-6">Recipe Details</h2>
          
          <div className="space-y-6">
            {/* Title */}
            <div>
              <label htmlFor="title" className="block text-sm font-medium mb-2">
                Title <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                id="title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onBlur={validateForm}
                className={`w-full p-3 border rounded-lg ${
                  errors.title ? 'border-red-500' : 'border-gray-300'
                }`}
                placeholder="Enter a title for your recipe"
              />
              {errors.title && <div className="text-red-500 text-sm mt-1">{errors.title}</div>}
            </div>

            {/* Description */}
            <div>
              <label htmlFor="description" className="block text-sm font-medium mb-2">
                Description <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                onBlur={validateForm}
                className={`w-full p-3 border rounded-lg ${
                  errors.description ? 'border-red-500' : 'border-gray-300'
                }`}
                placeholder="Enter a description for your recipe"
              />
              {errors.description && <div className="text-red-500 text-sm mt-1">{errors.description}</div>}
            </div>

            {/* Instructions */}
            <div>
              <label htmlFor="instructions" className="block text-sm font-medium mb-2">
                Instructions <span className="text-red-500">*</span>
              </label>
              <textarea
                id="instructions"
                value={instructions}
                onChange={(e) => setInstructions(e.target.value)}
                onBlur={validateForm}
                className={`w-full p-3 border rounded-lg min-h-[150px] ${
                  errors.instructions ? 'border-red-500' : 'border-gray-300'
                }`}
                placeholder="Enter instructions for the AI (these will be added to the system prompt)"
              />
              {errors.instructions && <div className="text-red-500 text-sm mt-1">{errors.instructions}</div>}
            </div>

            {/* Initial Prompt */}
            <div>
              <label htmlFor="prompt" className="block text-sm font-medium mb-2">
                Initial Message (optional)
              </label>
              <textarea
                id="prompt"
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                className="w-full p-3 border border-gray-300 rounded-lg min-h-[100px]"
                placeholder="Enter an initial message to start the session with (optional)"
              />
            </div>

            {/* Activities */}
            <div>
              <label className="block text-sm font-medium mb-2">
                Activities (optional)
              </label>
              <div className="flex flex-wrap gap-2 mb-4">
                {activities.map((activity, index) => (
                  <div
                    key={index}
                    className="inline-flex items-center bg-gray-100 border border-gray-300 rounded-full px-4 py-2 text-sm"
                  >
                    <span>{activity}</span>
                    <button
                      onClick={() => handleRemoveActivity(index)}
                      className="ml-2 text-gray-500 hover:text-red-500 transition-colors"
                      aria-label="Remove activity"
                    >
                      <X className="h-4 w-4" />
                    </button>
                  </div>
                ))}
              </div>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={newActivity}
                  onChange={(e) => setNewActivity(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && e.preventDefault()}
                  onKeyPress={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleAddActivity();
                    }
                  }}
                  className="flex-1 p-3 border border-gray-300 rounded-lg"
                  placeholder="Enter an activity"
                />
                <Button
                  onClick={handleAddActivity}
                  className="flex items-center gap-2"
                  disabled={!newActivity.trim()}
                >
                  <Plus className="h-4 w-4" />
                  Add
                </Button>
              </div>
            </div>
          </div>
        </div>

        {/* Generated URL */}
        <div className="bg-white border rounded-lg p-6 shadow-sm">
          <h2 className="text-2xl font-medium mb-4">Generated Recipe URL</h2>
          
          <div className="bg-gray-100 rounded-lg p-4 mb-4 overflow-x-auto">
            <code className="text-sm font-mono break-all">
              {recipeUrl || 'Fill in the required fields to generate a URL'}
            </code>
          </div>
          
          <div className="flex justify-end">
            <Button
              onClick={handleCopy}
              className="flex items-center gap-2"
              disabled={!recipeUrl}
            >
              {copied ? (
                <>
                  <Check className="h-4 w-4" />
                  Copied!
                </>
              ) : (
                <>
                  <Copy className="h-4 w-4" />
                  Copy URL
                </>
              )}
            </Button>
          </div>
        </div>

        {/* Instructions for Use */}
        <div className="mt-8 bg-white border rounded-lg p-6 shadow-sm">
          <h2 className="text-2xl font-medium mb-4">How to Use</h2>
          <ol className="list-decimal pl-6 space-y-2">
            <li>Fill in the required fields above to generate a recipe URL.</li>
            <li>Copy the generated URL using the "Copy URL" button.</li>
            <li>Share the URL with others who have Goose Desktop installed.</li>
            <li>When someone clicks the URL, it will open Goose Desktop with your recipe configuration.</li>
            <li>Alternatively, users can paste the URL in their browser to launch Goose Desktop with your recipe.</li>
          </ol>
        </div>
      </div>
    </Layout>
  );
}