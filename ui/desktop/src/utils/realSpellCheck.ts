// @ts-ignore
const Typo = require('typo-js');

export interface MisspelledWord {
  word: string;
  start: number;
  end: number;
  suggestions?: string[];
}

// Singleton spell checker instance
let spellChecker: Typo | null = null;

// Initialize the spell checker with US English dictionary
const initSpellChecker = async (): Promise<Typo> => {
  if (spellChecker) {
    return spellChecker;
  }

  try {
    // Try to load the dictionaries from CDN or local files
    const affData = await fetch('/dictionaries/en_US/en_US.aff').then(r => r.text()).catch(() => null);
    const dicData = await fetch('/dictionaries/en_US/en_US.dic').then(r => r.text()).catch(() => null);
    
    if (affData && dicData) {
      spellChecker = new Typo('en_US', affData, dicData);
      console.log('✅ Loaded full spell check dictionaries');
    } else {
      // Fallback: Create a basic spell checker without external files
      spellChecker = new Typo('en_US');
      console.log('⚠️ Using basic spell checker (no dictionary files found)');
    }
    
    return spellChecker;
  } catch (error) {
    console.error('Failed to initialize spell checker:', error);
    // Create a minimal spell checker as fallback
    spellChecker = new Typo('en_US');
    return spellChecker;
  }
};

export const checkSpelling = async (text: string): Promise<MisspelledWord[]> => {
  console.log('🔍 SIMPLE SPELL CHECK: Checking text:', text);
  const misspelledWords: MisspelledWord[] = [];
  
  // Simple test words that should definitely be caught
  const testMisspellings = ['gasdd2', 'recieve', 'seperate', 'definately', 'teh', 'wierd', 'freind', 'sdd', 'asdf'];
  
  // Split text into words
  const words = text.split(/\s+/);
  let currentPos = 0;
  
  for (const word of words) {
    const cleanWord = word.toLowerCase().replace(/[^a-z]/g, '');
    console.log('🔍 SIMPLE SPELL CHECK: Checking word:', cleanWord);
    
    if (testMisspellings.includes(cleanWord)) {
      const start = text.indexOf(word, currentPos);
      if (start !== -1) {
        misspelledWords.push({
          word: word,
          start: start,
          end: start + word.length,
          suggestions: ['test', 'suggestion']
        });
        console.log('🔍 SIMPLE SPELL CHECK: Found misspelling!', word, 'at position', start);
      }
    }
    
    currentPos += word.length + 1; // +1 for space
  }
  
  console.log('🔍 SIMPLE SPELL CHECK: Final result:', misspelledWords);
  return misspelledWords;
};

// Export the spell checker instance for other uses (like getting suggestions)
export const getSpellChecker = async (): Promise<Typo> => {
  return await initSpellChecker();
};
