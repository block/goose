import { execSync } from 'child_process';
import path from 'path';
import log from './logger';

export function loadZshEnv(): void {
  try {
    // Execute zsh and source the zshrc file, then export all environment variables
    const zshrcPath = path.join(process.env.HOME || '', '.zshrc');
    const envStr = execSync(`/bin/zsh -c 'source ${zshrcPath} && env'`, {
      encoding: 'utf-8'
    });

    // Parse and set environment variables
    envStr.split('\n').forEach(line => {
      const matches = line.match(/^([^=]+)=(.*)$/);
      if (matches) {
        const [, key, value] = matches;
        if (!process.env[key]) {
          process.env[key] = value;
        }
      }
    });

    log.info('Successfully loaded zsh environment variables');
  } catch (error) {
    log.error('Failed to load zsh environment variables:', error);
  }
}