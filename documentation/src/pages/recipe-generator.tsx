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
  const [copied, setCopied] = useState(false);
  const [errors, setErrors] = useState<{[key: string]: string}>({});
  const [outputFormat, setOutputFormat] = useState('url'); // 'url' or 'yaml'
  const [authorContact, setAuthorContact] = useState('');
  const [extensionsList, setExtensionsList] = useState([
    { type: 'builtin', name: 'developer', display_name: 'Developer', timeout: 300, bundled: true, enabled: false },
    { type: 'builtin', name: 'googledrive', display_name: 'Google Drive', timeout: 300, bundled: true, enabled: false },
    { type: 'builtin', name: 'computercontroller', display_name: 'Computer Controller', timeout: 300, bundled: true, enabled: false },
    { type: 'builtin', name: 'jetbrains', display_name: 'JetBrains', timeout: 300, bundled: true, enabled: false },
    { type: 'builtin', name: 'memory', display_name: 'Memory', timeout: 300, bundled: true, enabled: false },
    { 
      type: 'stdio', 
      name: 'pdf-reader', 
      cmd: 'uvx', 
      args: ['mcp-read-pdf@latest'], 
      envs: {}, 
      env_keys: [], 
      timeout: null, 
      description: "Read and analyze PDF documents", 
      enabled: false 
    }
  ]);
  const [prompt, setPrompt] = useState('');

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

  // Toggle extension handler
  const toggleExtension = useCallback((index) => {
    setExtensionsList(prev => {
      const updated = [...prev];
      updated[index] = { ...updated[index], enabled: !updated[index].enabled };
      return updated;
    });
  }, []);

  // Form validation
  const validateForm = useCallback(() => {
    const newErrors: {[key: string]: string} = {};
    
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

  // Generate output with useMemo to prevent re-renders
  const recipeOutput = useMemo(() => {
    // Only generate if we have the required fields
    if (!title.trim() || !description.trim() || !instructions.trim()) {
      return '';
    }

    try {
      if (outputFormat === 'url') {
        const recipeConfig = {
          version: "1.0.0",
          title,
          description,
          instructions,
          prompt: prompt.trim() || undefined,
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
      } else {
        // Generate YAML format
        const enabledExtensions = extensionsList.filter(ext => ext.enabled);
        
        let yaml = `version: 1.0.0
title: ${title}
description: ${description}
instructions: ${instructions}
`;

        if (authorContact) {
          yaml += `author:
  contact: ${authorContact}
`;
        }

        if (enabledExtensions.length > 0) {
          yaml += `extensions:
`;
          for (const ext of enabledExtensions) {
            if (ext.type === 'builtin') {
              yaml += `- type: ${ext.type}
  name: ${ext.name}
  display_name: ${ext.display_name}
  timeout: ${ext.timeout}
  bundled: ${ext.bundled}
`;
            } else if (ext.type === 'stdio') {
              yaml += `- type: ${ext.type}
  name: ${ext.name}
  cmd: ${ext.cmd}
  args:
  - ${ext.args.join('\n  - ')}
  envs: {}
  env_keys: []
  timeout: ${ext.timeout === null ? 'null' : ext.timeout}
  description: ${ext.description}
`;
            }
          }
        }

        if (prompt) {
          yaml += `prompt: ${prompt}
`;
        }

        return yaml;
      }
    } catch (error) {
      console.error('Error generating recipe output:', error);
      return '';
    }
  }, [title, description, instructions, activities, outputFormat, authorContact, extensionsList, prompt]);

  // Copy handler
  const handleCopy = useCallback(() => {
    if (validateForm() && recipeOutput) {
      navigator.clipboard.writeText(recipeOutput)
        .then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 2000);
        })
        .catch(err => console.error('Failed to copy output:', err));
    }
  }, [validateForm, recipeOutput]);

  return (
    <Layout>
      <div className="container mx-auto px-4 py-8 md:p-24">

        <div className="pb-8 md:pb-16">
          <h1 className="text-4xl md:text-[64px] font-medium text-textProminent">
            Recipe Generator
          </h1>
          <p className="text-textProminent">
            Create a shareable Goose recipe URL that others can use to launch a session with your predefined settings.
          </p>
        </div>

        <div className="bg-bgApp border border-borderSubtle rounded-lg p-6 mb-8 shadow-sm">
          <h2 className="text-2xl font-medium mb-6 text-textProminent">Recipe Details</h2>
          
          {/* Format Selection */}
          <div className="mb-6">
            <label className="block text-sm font-medium text-textStandard mb-2">
              Output Format
            </label>
            <div className="flex space-x-4">
              <label className="flex items-center">
                <input
                  type="radio"
                  name="format"
                  value="url"
                  checked={outputFormat === 'url'}
                  onChange={() => setOutputFormat('url')}
                  className="mr-2"
                />
                <span className="text-textStandard">URL Format</span>
              </label>
              <label className="flex items-center">
                <input
                  type="radio"
                  name="format"
                  value="yaml"
                  checked={outputFormat === 'yaml'}
                  onChange={() => setOutputFormat('yaml')}
                  className="mr-2"
                />
                <span className="text-textStandard">YAML Format</span>
              </label>
            </div>
          </div>
          
          <div className="space-y-6">
            {/* Title */}
            <div>
              <label htmlFor="title" className="block text-sm font-medium text-textStandard mb-2">
                Title <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                id="title"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onBlur={validateForm}
                className={`w-full p-3 border rounded-lg bg-bgSubtle text-textStandard ${
                  errors.title ? 'border-red-500' : 'border-borderSubtle'
                }`}
                placeholder="Enter a title for your recipe"
              />
              {errors.title && <div className="text-red-500 text-sm mt-1">{errors.title}</div>}
            </div>

            {/* Description */}
            <div>
              <label htmlFor="description" className="block text-sm font-medium text-textStandard mb-2">
                Description <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                id="description"
                value={description}
                onChange={(e) => setDescription(e.target.value)}
                onBlur={validateForm}
                className={`w-full p-3 border rounded-lg bg-bgSubtle text-textStandard ${
                  errors.description ? 'border-red-500' : 'border-borderSubtle'
                }`}
                placeholder="Enter a description for your recipe"
              />
              {errors.description && <div className="text-red-500 text-sm mt-1">{errors.description}</div>}
            </div>

            {/* Instructions */}
            <div>
              <label htmlFor="instructions" className="block text-sm font-medium text-textStandard mb-2">
                Instructions <span className="text-red-500">*</span>
              </label>
              <textarea
                id="instructions"
                value={instructions}
                onChange={(e) => setInstructions(e.target.value)}
                onBlur={validateForm}
                className={`w-full p-3 border rounded-lg bg-bgSubtle text-textStandard min-h-[150px] ${
                  errors.instructions ? 'border-red-500' : 'border-borderSubtle'
                }`}
                placeholder="Enter instructions for the AI (these will be added to the system prompt)"
              />
              {errors.instructions && <div className="text-red-500 text-sm mt-1">{errors.instructions}</div>}
            </div>

            {/* Initial Prompt */}
            <div>
              <label htmlFor="prompt" className="block text-sm font-medium text-textStandard mb-2">
                Initial Prompt (optional)
              </label>
              <textarea
                id="prompt"
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                className="w-full p-3 border border-borderSubtle rounded-lg bg-bgSubtle text-textStandard min-h-[100px]"
                placeholder="Enter an initial prompt to start the conversation (optional)"
              />
              <div className="text-sm text-textSubtle mt-1">
                If provided, this message will automatically start the conversation when the recipe is launched.
              </div>
            </div>

            {/* YAML-specific fields */}
            {outputFormat === 'yaml' && (
              <>
                <div>
                  <label htmlFor="authorContact" className="block text-sm font-medium text-textStandard mb-2">
                    Author Contact (optional)
                  </label>
                  <input
                    type="text"
                    id="authorContact"
                    value={authorContact}
                    onChange={(e) => setAuthorContact(e.target.value)}
                    className="w-full p-3 border border-borderSubtle rounded-lg bg-bgSubtle text-textStandard"
                    placeholder="Enter author contact information"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-textStandard mb-2">
                    Extensions (optional)
                  </label>
                  <div className="space-y-2">
                    {extensionsList.map((extension, index) => (
                      <div key={index} className="flex items-center p-3 border border-borderSubtle rounded-lg bg-bgSubtle">
                        <input
                          type="checkbox"
                          id={`extension-${index}`}
                          checked={extension.enabled}
                          onChange={() => toggleExtension(index)}
                          className="mr-3"
                        />
                        <label htmlFor={`extension-${index}`} className="flex-1 text-textStandard">
                          <span className="font-medium">{extension.display_name || extension.name}</span>
                          {extension.description && (
                            <span className="block text-sm text-textSubtle">{extension.description}</span>
                          )}
                        </label>
                      </div>
                    ))}
                  </div>
                </div>
              </>
            )}

            {/* Activities */}
            <div>
              <label className="block text-sm font-medium text-textStandard mb-2">
                Activities (optional)
              </label>
              <div className="flex flex-wrap gap-2 mb-4">
                {activities.map((activity, index) => (
                  <div
                    key={index}
                    className="inline-flex items-center bg-bgSubtle border border-borderSubtle rounded-full px-4 py-2 text-sm text-textStandard"
                  >
                    <span>{activity}</span>
                    <button
                      onClick={() => handleRemoveActivity(index)}
                      className="ml-2 text-textSubtle hover:text-red-500 transition-colors bg-transparent border-none"
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
                  className="flex-1 p-3 border border-borderSubtle rounded-lg bg-bgSubtle text-textStandard"
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

        {/* Generated Output */}
        <div className="bg-bgApp border border-borderSubtle rounded-lg p-6 shadow-sm">
          <h2 className="text-2xl font-medium mb-4 text-textProminent">
            Generated Recipe {outputFormat === 'url' ? 'URL' : 'YAML'}
          </h2>
          
          <div className="bg-bgSubtle rounded-lg p-4 mb-4 overflow-x-auto">
            <pre className="text-sm text-textStandard font-mono break-all whitespace-pre-wrap">
              {recipeOutput || `Fill in the required fields to generate a ${outputFormat === 'url' ? 'URL' : 'YAML'}`}
            </pre>
          </div>
          
          <div className="flex justify-end">
            <Button
              onClick={handleCopy}
              className="flex items-center gap-2"
              disabled={!recipeOutput}
            >
              {copied ? (
                <>
                  <Check className="h-4 w-4" />
                  Copied!
                </>
              ) : (
                <>
                  <Copy className="h-4 w-4" />
                  Copy {outputFormat === 'url' ? 'URL' : 'YAML'}
                </>
              )}
            </Button>
          </div>
        </div>

        {/* Instructions for Use */}
        <div className="mt-8 bg-bgApp border border-borderSubtle rounded-lg p-6 shadow-sm">
          <h2 className="text-2xl font-medium mb-4 text-textProminent">How to Use</h2>
          <ol className="list-decimal pl-6 space-y-2 text-textStandard">
            <li>Fill in the required fields above to generate a recipe.</li>
            <li>Choose between URL format (for direct sharing) or YAML format (for configuration files).</li>
            <li>For URL format:
              <ul className="list-disc pl-6 mt-2">
                <li>Copy the generated URL using the "Copy URL" button.</li>
                <li>Share the URL with others who have Goose Desktop installed.</li>
                <li>When someone clicks the URL, it will open Goose Desktop with your recipe configuration.</li>
              </ul>
            </li>
            <li>For YAML format:
              <ul className="list-disc pl-6 mt-2">
                <li>Copy the generated YAML using the "Copy YAML" button.</li>
                <li>Save it as a <code>.yaml</code> file.</li>
                <li>Use with the CLI: <code>goose run --recipe your-recipe.yaml</code></li>
                <li>Or create a deeplink with: <code>goose recipe deeplink your-recipe.yaml</code></li>
              </ul>
            </li>
          </ol>
        </div>
      </div>
    </Layout>
  );
}