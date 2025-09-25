// Test if typo-js is working correctly
const Typo = require('typo-js');

console.log('Testing Typo.js...');
console.log('Typo constructor:', typeof Typo);

try {
  // Try to create a basic spell checker
  const checker = new Typo('en_US');
  console.log('Basic checker created:', !!checker);
  
  // Test some words
  const testWords = ['hello', 'recieve', 'seperate', 'test'];
  testWords.forEach(word => {
    const isCorrect = checker.check(word);
    console.log(`Word "${word}": ${isCorrect ? 'CORRECT' : 'MISSPELLED'}`);
  });
  
} catch (error) {
  console.error('Error creating Typo checker:', error);
}
