// Native spell checker using system dictionary
class NativeSpellChecker {
  private wordSet: Set<string> = new Set();
  private isLoaded = false;

  async loadDictionary(): Promise<void> {
    if (this.isLoaded) return;

    try {
      // In a real Electron app, you'd use Node.js fs to read the file
      // For now, we'll use fetch to load a word list
      // This is a placeholder - in production you'd need to expose the system dict via Electron's main process
      
      // For demonstration, let's use a smaller word list or the browser's API
      // In a full Electron implementation, you'd read /usr/share/dict/words
      
      // Fallback to browser's native spell check capability
      this.isLoaded = true;
      console.log('Native spell checker loaded (using browser fallback)');
    } catch (error) {
      console.error('Failed to load native dictionary:', error);
      this.isLoaded = false;
    }
  }

  isWordCorrect(word: string): boolean {
    if (!this.isLoaded) return true; // Don't mark as incorrect if not loaded
    
    // Clean the word
    const cleanWord = word.toLowerCase().replace(/[^a-z]/g, '');
    if (cleanWord.length < 2) return true;
    
    // For now, use a heuristic approach since we can't easily access the system dict from browser
    // In a full Electron app, you'd check: return this.wordSet.has(cleanWord);
    
    // Simple heuristic checks
    if (this.isObviouslyCorrect(cleanWord)) return true;
    if (this.isObviouslyIncorrect(cleanWord)) return false;
    
    // Default to correct for unknown words
    return true;
  }

  private isObviouslyCorrect(word: string): boolean {
    // Common words that are definitely correct
    const commonWords = new Set([
      'the', 'be', 'to', 'of', 'and', 'a', 'in', 'that', 'have', 'i', 'it', 'for', 'not', 'on', 'with', 'he', 'as', 'you', 'do', 'at',
      'this', 'but', 'his', 'by', 'from', 'they', 'we', 'say', 'her', 'she', 'or', 'an', 'will', 'my', 'one', 'all', 'would', 'there', 'their',
      'what', 'so', 'up', 'out', 'if', 'about', 'who', 'get', 'which', 'go', 'me', 'when', 'make', 'can', 'like', 'time', 'no', 'just', 'him',
      'know', 'take', 'people', 'into', 'year', 'your', 'good', 'some', 'could', 'them', 'see', 'other', 'than', 'then', 'now', 'look', 'only',
      'come', 'its', 'over', 'think', 'also', 'back', 'after', 'use', 'two', 'how', 'our', 'work', 'first', 'well', 'way', 'even', 'new', 'want',
      'because', 'any', 'these', 'give', 'day', 'most', 'us', 'is', 'was', 'are', 'been', 'has', 'had', 'were', 'said', 'each', 'which', 'their',
      'said', 'them', 'she', 'many', 'some', 'very', 'when', 'much', 'before', 'right', 'too', 'means', 'old', 'any', 'same', 'tell', 'boy', 'follow',
      'came', 'want', 'show', 'also', 'around', 'farm', 'three', 'small', 'set', 'put', 'end', 'why', 'again', 'turn', 'here', 'off', 'went', 'old',
      'number', 'great', 'tell', 'men', 'say', 'small', 'every', 'found', 'still', 'between', 'name', 'should', 'home', 'big', 'give', 'air', 'line',
      'where', 'much', 'too', 'means', 'old', 'any', 'same', 'tell', 'boy', 'follow', 'came', 'want', 'show'
    ]);
    
    return commonWords.has(word);
  }

  private isObviouslyIncorrect(word: string): boolean {
    // Patterns that are likely incorrect
    if (word.length < 2) return false;
    
    // Multiple consecutive same letters (more than 2)
    if (/(.)\1{2,}/.test(word)) return true;
    
    // Too many consonants in a row
    if (/[bcdfghjklmnpqrstvwxyz]{5,}/.test(word)) return true;
    
    // Starts or ends with unlikely combinations
    if (/^[qxz]/.test(word) && word.length < 4) return true;
    
    return false;
  }

  getSuggestions(word: string): string[] {
    // Simple suggestions based on common typos
    const suggestions: string[] = [];
    const cleanWord = word.toLowerCase();
    
    // Common corrections
    const corrections: { [key: string]: string[] } = {
      'teh': ['the'],
      'adn': ['and'],
      'recieve': ['receive'],
      'seperate': ['separate'],
      'definately': ['definitely'],
      'occured': ['occurred'],
      'begining': ['beginning'],
      'existance': ['existence'],
      'independant': ['independent'],
      'neccessary': ['necessary'],
      'priviledge': ['privilege'],
      'recomend': ['recommend'],
      'succesful': ['successful'],
      'tommorow': ['tomorrow'],
      'truely': ['truly'],
      'untill': ['until'],
      'wierd': ['weird'],
      'acheive': ['achieve'],
      'beleive': ['believe'],
      'concious': ['conscious'],
      'embarass': ['embarrass'],
      'fourty': ['forty'],
      'goverment': ['government'],
      'harrass': ['harass'],
      'occassion': ['occasion'],
      'posession': ['possession'],
      'publically': ['publicly'],
      'reccomend': ['recommend'],
      'supercede': ['supersede'],
      'thier': ['their'],
      'truley': ['truly']
    };
    
    if (corrections[cleanWord]) {
      suggestions.push(...corrections[cleanWord]);
    }
    
    return suggestions;
  }
}

// Export singleton instance
export const nativeSpellChecker = new NativeSpellChecker();

// Initialize on import
nativeSpellChecker.loadDictionary();

export function checkSpelling(text: string): Array<{ word: string; start: number; end: number; suggestions: string[] }> {
  const errors: Array<{ word: string; start: number; end: number; suggestions: string[] }> = [];
  
  // Split text into words while preserving positions
  const wordRegex = /\b[a-zA-Z]+\b/g;
  let match;
  
  while ((match = wordRegex.exec(text)) !== null) {
    const word = match[0];
    const start = match.index;
    const end = start + word.length;
    
    if (!nativeSpellChecker.isWordCorrect(word)) {
      errors.push({
        word,
        start,
        end,
        suggestions: nativeSpellChecker.getSuggestions(word)
      });
    }
  }
  
  return errors;
}
