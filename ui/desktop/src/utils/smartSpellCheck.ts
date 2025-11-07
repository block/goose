// Smart spell checking using heuristics and patterns
export interface MisspelledWord {
  word: string;
  start: number;
  end: number;
}

export const checkSpelling = async (text: string): Promise<MisspelledWord[]> => {
  const misspelledWords: MisspelledWord[] = [];
  
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
      (/wether/.test(cleanWord) && cleanWord !== 'whether') ||
      // Test words for debugging
      cleanWord === 'sdd' || cleanWord === 'asdf' || cleanWord === 'qwerty' ||
      cleanWord === 'teh' || cleanWord === 'alot' || cleanWord === 'wierd' ||
      cleanWord === 'freind' || cleanWord === 'thier' || cleanWord === 'calender'
    ) {
      isMisspelled = true;
    }
    
    // Additional check: words with unusual letter combinations
    if (!isMisspelled && cleanWord.length > 4) {
      // Check for unusual patterns
      const doubleLetterPattern = /(.)\1{2,}/; // Three or more of the same letter
      const tooManyConsonants = /[bcdfgjklmnpqrstvwxz]{4,}/; // 4+ consonants in a row
      const tooManyVowels = /[aeiou]{4,}/; // 4+ vowels in a row
      const qWithoutU = /q(?!u)/; // Q not followed by U
      const multipleXYZ = /[xyz]{2,}/; // Multiple x, y, or z
      
      if (
        doubleLetterPattern.test(cleanWord) ||
        tooManyConsonants.test(cleanWord) ||
        tooManyVowels.test(cleanWord) ||
        qWithoutU.test(cleanWord) ||
        multipleXYZ.test(cleanWord)
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
};
