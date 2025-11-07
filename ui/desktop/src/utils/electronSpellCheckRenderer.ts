// Renderer process side of Electron spell checking
// This would be used in your React components

declare global {
  interface Window {
    electronAPI?: {
      spellCheck: (word: string) => Promise<boolean>;
      spellSuggestions: (word: string) => Promise<string[]>;
    };
  }
}

class ElectronSpellCheckRenderer {
  async isWordCorrect(word: string): Promise<boolean> {
    if (!window.electronAPI) {
      console.warn('Electron API not available, falling back to browser spell check');
      return true; // Fallback
    }
    
    try {
      return await window.electronAPI.spellCheck(word);
    } catch (error) {
      console.error('Electron spell check failed:', error);
      return true; // Fallback to assuming correct
    }
  }
  
  async getSuggestions(word: string): Promise<string[]> {
    if (!window.electronAPI) {
      return []; // No suggestions available
    }
    
    try {
      return await window.electronAPI.spellSuggestions(word);
    } catch (error) {
      console.error('Electron spell suggestions failed:', error);
      return [];
    }
  }
}

export const electronSpellCheckRenderer = new ElectronSpellCheckRenderer();

// Spell check function that works with the existing interface
export async function checkSpelling(text: string): Promise<Array<{ word: string; start: number; end: number; suggestions: string[] }>> {
  const errors: Array<{ word: string; start: number; end: number; suggestions: string[] }> = [];
  
  // Split text into words while preserving positions
  const wordRegex = /\b[a-zA-Z]+\b/g;
  let match;
  
  const promises: Promise<void>[] = [];
  
  while ((match = wordRegex.exec(text)) !== null) {
    const word = match[0];
    const start = match.index;
    const end = start + word.length;
    
    // Check each word asynchronously
    promises.push(
      electronSpellCheckRenderer.isWordCorrect(word).then(async (isCorrect) => {
        if (!isCorrect) {
          const suggestions = await electronSpellCheckRenderer.getSuggestions(word);
          errors.push({
            word,
            start,
            end,
            suggestions
          });
        }
      })
    );
  }
  
  // Wait for all checks to complete
  await Promise.all(promises);
  
  // Sort errors by position
  errors.sort((a, b) => a.start - b.start);
  
  return errors;
}
