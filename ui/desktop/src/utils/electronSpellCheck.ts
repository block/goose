// Example of how to use system dictionary in Electron
// This would go in your Electron main process

import { readFileSync } from 'fs';
import { ipcMain } from 'electron';

class ElectronSpellChecker {
  private wordSet: Set<string> = new Set();
  
  constructor() {
    this.loadSystemDictionary();
    this.setupIPC();
  }
  
  private loadSystemDictionary() {
    try {
      // Read the system dictionary
      const dictPath = '/usr/share/dict/words';
      const content = readFileSync(dictPath, 'utf8');
      const words = content.split('\n').filter(word => word.length > 0);
      
      // Create a Set for fast lookups
      words.forEach(word => {
        this.wordSet.add(word.toLowerCase());
      });
      
      console.log(`Loaded ${this.wordSet.size} words from system dictionary`);
    } catch (error) {
      console.error('Failed to load system dictionary:', error);
    }
  }
  
  private setupIPC() {
    // Handle spell check requests from renderer process
    ipcMain.handle('spell-check', (event, word: string) => {
      return this.isWordCorrect(word);
    });
    
    ipcMain.handle('spell-suggestions', (event, word: string) => {
      return this.getSuggestions(word);
    });
  }
  
  isWordCorrect(word: string): boolean {
    const cleanWord = word.toLowerCase().replace(/[^a-z]/g, '');
    return this.wordSet.has(cleanWord);
  }
  
  getSuggestions(word: string): string[] {
    // Simple edit distance suggestions
    const suggestions: string[] = [];
    const cleanWord = word.toLowerCase();
    
    // Find words with edit distance of 1
    for (const dictWord of this.wordSet) {
      if (this.editDistance(cleanWord, dictWord) === 1) {
        suggestions.push(dictWord);
        if (suggestions.length >= 5) break; // Limit suggestions
      }
    }
    
    return suggestions;
  }
  
  private editDistance(a: string, b: string): number {
    if (Math.abs(a.length - b.length) > 1) return 2; // Quick optimization
    
    const matrix = Array(a.length + 1).fill(null).map(() => Array(b.length + 1).fill(0));
    
    for (let i = 0; i <= a.length; i++) matrix[i][0] = i;
    for (let j = 0; j <= b.length; j++) matrix[0][j] = j;
    
    for (let i = 1; i <= a.length; i++) {
      for (let j = 1; j <= b.length; j++) {
        if (a[i - 1] === b[j - 1]) {
          matrix[i][j] = matrix[i - 1][j - 1];
        } else {
          matrix[i][j] = Math.min(
            matrix[i - 1][j] + 1,     // deletion
            matrix[i][j - 1] + 1,     // insertion
            matrix[i - 1][j - 1] + 1  // substitution
          );
        }
      }
    }
    
    return matrix[a.length][b.length];
  }
}

// Initialize in main process
export const electronSpellChecker = new ElectronSpellChecker();
