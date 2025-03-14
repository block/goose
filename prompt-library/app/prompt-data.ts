import type { Prompt } from "./types/prompt";

// Use Vite's import.meta.glob to dynamically import all JSON files in the roles directory
const promptModules = import.meta.glob<Prompt>('../prompts/roles/**/*.json', { eager: true, import: 'default' });

// Convert the modules object into an array of prompts
const promptsData: Prompt[] = Object.values(promptModules);

export async function fetchPrompts(): Promise<Prompt[]> {
  return promptsData;
}

export async function searchPrompts(query: string): Promise<Prompt[]> {
  const lowercaseQuery = query.toLowerCase();
  
  return promptsData.filter(prompt => 
    prompt.title.toLowerCase().includes(lowercaseQuery) ||
    prompt.description.toLowerCase().includes(lowercaseQuery) ||
    prompt.category.toLowerCase().includes(lowercaseQuery) ||
    prompt.function.toLowerCase().includes(lowercaseQuery) ||
    prompt.tags.some(tag => tag.toLowerCase().includes(lowercaseQuery))
  );
}