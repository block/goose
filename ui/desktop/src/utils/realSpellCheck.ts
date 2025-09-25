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
  const misspelledWords: MisspelledWord[] = [];
  
  console.log('🔍 REAL SPELL CHECK: Starting spell check for:', text);
  
  try {
    const checker = await initSpellChecker();
    console.log('🔍 REAL SPELL CHECK: Checker initialized:', !!checker);
    
    // Split text into words while preserving positions
    const wordRegex = /\b[a-zA-Z]+\b/g;
    let match;
    
    while ((match = wordRegex.exec(text)) !== null) {
      const word = match[0];
      const start = match.index;
      const end = start + word.length;
      
      console.log('🔍 REAL SPELL CHECK: Checking word:', word);
      
      // Skip very short words and common abbreviations
      if (word.length < 3) {
        console.log('🔍 REAL SPELL CHECK: Skipping short word:', word);
        continue;
      }
      
      // Skip common technical terms that might not be in dictionary
      const technicalTerms = [
        'api', 'url', 'http', 'https', 'json', 'xml', 'css', 'html', 'js', 'ts', 'jsx', 'tsx',
        'npm', 'git', 'cli', 'ui', 'ux', 'db', 'sql', 'dev', 'prod', 'env', 'config', 'src',
        'app', 'web', 'www', 'com', 'org', 'net', 'io', 'ai', 'ml', 'gpu', 'cpu', 'ram',
        'github', 'gitlab', 'docker', 'aws', 'gcp', 'azure', 'oauth', 'jwt', 'cors', 'auth',
        'goose', 'chat', 'llm', 'gpt', 'claude', 'openai', 'anthropic', 'react', 'node',
        'typescript', 'javascript', 'python', 'rust', 'java', 'cpp', 'php', 'ruby', 'golang',
        'webpack', 'babel', 'eslint', 'prettier', 'tailwind', 'scss', 'sass', 'redux', 'graphql'
      ];
      
      if (technicalTerms.includes(word.toLowerCase())) {
        console.log('🔍 REAL SPELL CHECK: Skipping technical term:', word);
        continue;
      }
      
      // Check if the word is misspelled
      const isCorrect = checker.check(word);
      console.log('🔍 REAL SPELL CHECK: Word', word, 'is', isCorrect ? 'CORRECT' : 'MISSPELLED');
      
      if (!isCorrect) {
        console.log('🔍 REAL SPELL CHECK: Found misspelling!', word);
        
        // Get suggestions for the misspelled word
        const suggestions = checker.suggest ? checker.suggest(word, 3) : []; // Get up to 3 suggestions
        
        misspelledWords.push({
          word: word,
          start: start,
          end: end,
          suggestions: suggestions
        });
        
        console.log('🔍 REAL SPELL CHECK: Added misspelling:', { word, start, end, suggestions });
      }
    }
    
    console.log('🔍 REAL SPELL CHECK: Final misspelled words:', misspelledWords);
    
  } catch (error) {
    console.error('🔍 REAL SPELL CHECK ERROR:', error);
    
    // Fallback to simple spell check for testing
    console.log('🔍 REAL SPELL CHECK: Using fallback spell check');
    const simpleWords = ['recieve', 'seperate', 'definately', 'teh', 'wierd', 'freind'];
    const wordRegex = /\b[a-zA-Z]+\b/g;
    let match;
    
    while ((match = wordRegex.exec(text)) !== null) {
      const word = match[0];
      if (simpleWords.includes(word.toLowerCase())) {
        misspelledWords.push({
          word: word,
          start: match.index,
          end: match.index + word.length,
          suggestions: []
        });
        console.log('🔍 FALLBACK SPELL CHECK: Found misspelling:', word);
      }
    }
  }
  
  return misspelledWords;
};

// Export the spell checker instance for other uses (like getting suggestions)
export const getSpellChecker = async (): Promise<Typo> => {
  return await initSpellChecker();
};
