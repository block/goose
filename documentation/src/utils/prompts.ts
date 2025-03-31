import type { Prompt } from '@site/src/types/prompt';

const promptContext = require.context(
  '../pages/prompt-library/data/prompts',
  false, 
  /\.json$/
);

// Convert the modules into an array of prompts
const prompts: Prompt[] = promptContext.keys().map((key) => {
  const prompt = promptContext(key);
  return prompt.default || prompt; // handle both ESM and CommonJS modules
});

export async function searchPrompts(query: string): Promise<Prompt[]> {
  const searchTerms = query.toLowerCase().split(' ').filter(Boolean);
  
  if (!searchTerms.length) {
    return prompts;
  }

  return prompts.filter((prompt) => {
    const searchableText = [
      prompt.title,
      prompt.description,
      prompt.example_prompt,
      ...prompt.extensions.map(ext => ext.name)
    ].join(' ').toLowerCase();

    return searchTerms.every(term => searchableText.includes(term));
  });
}

export async function getPromptById(id: string): Promise<Prompt | null> {
  return prompts.find(prompt => prompt.id === id) || null;
}