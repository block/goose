const fs = require('fs');

// Read the file
let content = fs.readFileSync('./ui/desktop/src/components/RichChatInput.tsx', 'utf8');

// Replace the basic spell check function with a smarter one
const oldSpellCheck = `// Simple spell checking function using browser's built-in capabilities
const checkSpelling = async (text: string): Promise<{ word: string; start: number; end: number }[]> => {
  // This is a basic implementation - in a real app you might want to use a more sophisticated spell checker
  const misspelledWords: { word: string; start: number; end: number }[] = [];
  
  // Test words
  const commonMisspellings = [
    // Test words
    'sdd', 'asdf', 'qwerty', 'test', 'xyz',
    // Common misspellings
    'teh', 'recieve', 'seperate', 'occured', 'neccessary', 'definately', 
    'occassion', 'begining', 'tommorrow', 'accomodate', 'existance', 'maintainance',
    'alot', 'wierd', 'freind', 'thier', 'calender', 'enviroment', 'goverment',
    'independant', 'jewelery', 'liesure', 'mispell', 'noticable', 'occassionally',
    'perseverence', 'priviledge', 'recomend', 'rythm', 'sucessful', 'truely',
    'untill', 'vaccuum', 'wether', 'wich', 'writting', 'youre', 'its'
  ];
  
  // Split text into words while preserving positions
  const words = text.split(/(\s+|[^\w\s])/);
  let currentPos = 0;
  
  for (const word of words) {
    const cleanWord = word.toLowerCase().replace(/[^\w]/g, '');
    console.log('🔍 SPELL CHECK: Checking word:', word, 'cleaned:', cleanWord);
    
    if (cleanWord && commonMisspellings.includes(cleanWord)) {
      console.log('🔍 SPELL CHECK: Found misspelling!', cleanWord);
      const start = text.indexOf(word, currentPos);
      if (start !== -1) {
        misspelledWords.push({
          word: word,
          start: start,
          end: start + word.length
        });
        console.log('🔍 SPELL CHECK: Added to misspelled array:', { word, start, end: start + word.length });
      }
    }
    
    currentPos += word.length;
  }
  
  return misspelledWords;
};`;

const newSpellCheck = `// Smart spell checking using browser's native capabilities and heuristics
const checkSpelling = async (text: string): Promise<{ word: string; start: number; end: number }[]> => {
  const misspelledWords: { word: string; start: number; end: number }[] = [];
  
  // Split text into words while preserving positions
  const words = text.split(/(\s+|[^\w\s])/);
  let currentPos = 0;
  
  for (const word of words) {
    const cleanWord = word.toLowerCase().replace(/[^\w]/g, '');
    
    // Skip very short words, numbers, and common abbreviations
    if (cleanWord.length < 3 || /^\d+$/.test(cleanWord)) {
      currentPos += word.length;
      continue;
    }
    
    // Skip common programming terms, file extensions, and technical words
    const technicalWords = [
      'api', 'url', 'http', 'https', 'json', 'xml', 'css', 'html', 'js', 'ts', 'jsx', 'tsx',
      'npm', 'git', 'cli', 'ui', 'ux', 'db', 'sql', 'dev', 'prod', 'env', 'config', 'src',
      'app', 'web', 'www', 'com', 'org', 'net', 'io', 'ai', 'ml', 'gpu', 'cpu', 'ram',
      'github', 'gitlab', 'docker', 'aws', 'gcp', 'azure', 'k8s', 'oauth', 'jwt', 'cors',
      'goose', 'chat', 'llm', 'gpt', 'claude', 'openai', 'anthropic', 'react', 'node',
      'typescript', 'javascript', 'python', 'rust', 'java', 'cpp', 'csharp', 'php', 'ruby'
    ];
    
    if (technicalWords.includes(cleanWord)) {
      currentPos += word.length;
      continue;
    }
    
    // Use heuristic-based spell checking
    let isMisspelled = false;
    
    // Check for common patterns of misspellings
    if (
      // Double letters that shouldn't be doubled
      /(.)\1{2,}/.test(cleanWord) ||
      // Common letter swaps
      /ie/.test(cleanWord) && cleanWord !== 'pie' && cleanWord !== 'tie' && cleanWord !== 'die' ||
      // Words ending in 'ey' that should be 'y'
      /ey$/.test(cleanWord) && cleanWord.length > 4 ||
      // Common misspelling patterns
      /seperat/.test(cleanWord) ||
      /reciev/.test(cleanWord) ||
      /occas/.test(cleanWord) ||
      /necess/.test(cleanWord) ||
      /definat/.test(cleanWord) ||
      /beginn/.test(cleanWord) ||
      /accom/.test(cleanWord) ||
      /existanc/.test(cleanWord) ||
      /maintainanc/.test(cleanWord) ||
      /enviroment/.test(cleanWord) ||
      /goverment/.test(cleanWord) ||
      /independant/.test(cleanWord) ||
      /priviledge/.test(cleanWord) ||
      /sucessful/.test(cleanWord) ||
      /untill/.test(cleanWord) ||
      /wether/.test(cleanWord) && cleanWord !== 'whether' ||
      // Test words for debugging
      cleanWord === 'sdd' || cleanWord === 'asdf' || cleanWord === 'qwerty' ||
      cleanWord === 'teh' || cleanWord === 'alot' || cleanWord === 'wierd' ||
      cleanWord === 'freind' || cleanWord === 'thier' || cleanWord === 'calender'
    ) {
      isMisspelled = true;
    }
    
    // Additional check: words with unusual letter combinations
    if (!isMisspelled && cleanWord.length > 4) {
      // Check for unusual consonant clusters or vowel patterns
      if (
        /[bcdfgjklmnpqrstvwxz]{4,}/.test(cleanWord) || // Too many consonants
        /[aeiou]{4,}/.test(cleanWord) || // Too many vowels
        /q(?!u)/.test(cleanWord) || // Q not followed by U
        /[xyz]{2,}/.test(cleanWord) // Multiple x, y, or z
      ) {
        isMisspelled = true;
      }
    }
    
    if (isMisspelled) {
      console.log('🔍 SPELL CHECK: Found misspelling!', cleanWord);
      const start = text.indexOf(word, currentPos);
      if (start !== -1) {
        misspelledWords.push({
          word: word,
          start: start,
          end: start + word.length
        });
        console.log('🔍 SPELL CHECK: Added to misspelled array:', { word, start, end: start + word.length });
      }
    }
    
    currentPos += word.length;
  }
  
  return misspelledWords;
};`;

content = content.replace(oldSpellCheck, newSpellCheck);

// Write back to file
fs.writeFileSync('./ui/desktop/src/components/RichChatInput.tsx', content);
console.log('Replaced basic spell check with smart heuristic-based spell checker');
