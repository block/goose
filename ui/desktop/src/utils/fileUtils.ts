import fs from 'node:fs';
import { Buffer } from 'node:buffer';

/**
 * Checks if a file is likely to be a text file by reading its first chunk
 * and checking for null bytes and valid UTF-8 encoding
 */
export function isTextFile(filePath: string): boolean {
  try {
    // Read the first 8KB of the file
    const buffer = Buffer.alloc(8192);
    const fd = fs.openSync(filePath, 'r');
    const bytesRead = fs.readSync(fd, buffer, 0, 8192, 0);
    fs.closeSync(fd);

    // If file is empty, consider it text
    if (bytesRead === 0) return true;

    // Check for null bytes which usually indicate binary content
    for (let i = 0; i < bytesRead; i++) {
      if (buffer[i] === 0) return false;
    }

    // Try to decode as UTF-8
    const content = buffer.slice(0, bytesRead).toString('utf8');

    // If we can decode it as UTF-8 and it contains printable characters
    // we'll consider it a text file
    return content.length > 0 && /^[\x20-\x7E\t\n\r\x80-\xFF]*$/.test(content);
  } catch (error) {
    console.error('Error checking file type:', error);
    return false;
  }
}
