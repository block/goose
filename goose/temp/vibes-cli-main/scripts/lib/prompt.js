/**
 * Readline prompt utilities
 *
 * Shared helpers for interactive CLI prompts.
 */

import * as readline from 'readline';

/**
 * Create a readline interface
 * @returns {readline.Interface}
 */
export function createReadline() {
  return readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });
}

/**
 * Prompt user for input
 * @param {string} question - The question to ask
 * @returns {Promise<string>} User's trimmed answer
 */
export async function prompt(question) {
  const rl = createReadline();
  return new Promise((resolve) => {
    rl.question(question, (answer) => {
      rl.close();
      resolve(answer.trim());
    });
  });
}

/**
 * Prompt user for yes/no confirmation
 * @param {string} question - The question to ask (will append " (y/N): ")
 * @returns {Promise<boolean>} True if user confirmed
 */
export async function confirm(question) {
  const answer = await prompt(`${question} (y/N): `);
  return answer.toLowerCase() === 'y' || answer.toLowerCase() === 'yes';
}
