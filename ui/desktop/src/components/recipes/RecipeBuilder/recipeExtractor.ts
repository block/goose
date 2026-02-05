import { Recipe, parseRecipeFromFile } from '../../../recipe';
import { Message, MessageContent } from '../../../api';

export interface ExtractedRecipe {
  recipe: Recipe;
  yamlString: string;
}

export function extractYamlFromText(text: string): string | null {
  const yamlBlockRegex = /```(?:yaml|yml)\n([\s\S]*?)```/gi;
  const matches = [...text.matchAll(yamlBlockRegex)];

  if (matches.length === 0) {
    return null;
  }

  return matches[matches.length - 1][1].trim();
}

export async function parseYamlToRecipe(yamlString: string): Promise<Recipe | null> {
  try {
    const recipe = await parseRecipeFromFile(yamlString);
    return recipe;
  } catch (error) {
    console.error('Failed to parse YAML:', error);
    return null;
  }
}

export function extractYamlFromMessage(message: Message): string | null {
  if (message.role !== 'assistant') {
    return null;
  }

  const textContent = message.content.find((c: MessageContent) => c.type === 'text') as
    | { type: 'text'; text: string }
    | undefined;

  if (!textContent) {
    return null;
  }

  return extractYamlFromText(textContent.text);
}

export async function extractRecipeFromMessage(message: Message): Promise<ExtractedRecipe | null> {
  const yamlString = extractYamlFromMessage(message);
  if (!yamlString) {
    return null;
  }

  const recipe = await parseYamlToRecipe(yamlString);
  if (!recipe) {
    return null;
  }

  return { recipe, yamlString };
}

export async function extractRecipeFromMessages(
  messages: Message[]
): Promise<ExtractedRecipe | null> {
  for (let i = messages.length - 1; i >= 0; i--) {
    const result = await extractRecipeFromMessage(messages[i]);
    if (result) {
      return result;
    }
  }
  return null;
}
