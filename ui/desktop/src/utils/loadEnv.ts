import { execSync } from 'child_process';
import log from './logger';

export function loadShellEnv(isProduction: boolean = false): void {
  // Only proceed if running on macOS and in production mode
  if (process.platform !== 'darwin' || !isProduction) {
    log.info(
      `Skipping zsh environment loading: ${
        process.platform !== 'darwin' ? 'Not running on macOS' : 'Not in production mode'
      }`
    );
    return;
  }

  try {
    log.info('LOADING ENV');

    const shell = process.env.SHELL || '/bin/bash'; // Detect user's shell

    const envStr = execSync(`${shell} -l -i -c 'env'`, {
      encoding: 'utf-8',
    });

    // Parse and set environment variables
    envStr.split('\n').forEach((line) => {
      const matches = line.match(/^([^=]+)=(.*)$/);
      if (matches) {
        const [, key, value] = matches;
        log.info(`Setting ${key}`);
        process.env[key] = value;
      }
    });

    log.info('Successfully loaded zsh environment variables');
  } catch (error) {
    log.error('Failed to load zsh environment variables:', error);
  }
}
