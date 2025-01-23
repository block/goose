import { execSync } from 'child_process';
import path from 'path';
import log from './logger';
import fs from 'node:fs';

export function loadZshEnv(isProduction: boolean = false): void {
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
    // Execute zsh and source the zshrc file, then export all environment variables
    const zshrcPath = path.join(process.env.HOME || '', '.zshrc');

    // if no file then return
    if (!fs.existsSync(zshrcPath)) {
      console.log('No zshrc file found');
      return;
    }

    const envStr = execSync(`/bin/zsh -c 'source ${zshrcPath} && env'`, {
      encoding: 'utf-8',
    });

    // Parse and set environment variables
    envStr.split('\n').forEach((line) => {
      const matches = line.match(/^([^=]+)=(.*)$/);
      if (matches) {
        const [, key, value] = matches;
        process.env[key] = value;
      }
    });

    log.info('Successfully loaded zsh environment variables');
  } catch (error) {
    log.error('Failed to load zsh environment variables:', error);
  }
}
