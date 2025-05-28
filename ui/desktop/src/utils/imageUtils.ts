/**
 * Utility functions for detecting and handling image paths in messages
 */

/**
 * Extracts image file paths from a message text
 * Looks for paths that match the pattern of pasted images from the temp directory
 *
 * @param text The message text to extract image paths from
 * @returns An array of image file paths found in the message
 */
export function extractImagePaths(text: string): string[] {
  if (!text) return [];

  // Match paths that look like pasted image paths from the temp directory
  // Pattern: /path/to/goose-pasted-images/pasted-img-TIMESTAMP-RANDOM.ext
  // This regex looks for:
  // - Word boundary or start of string
  // - A path containing "goose-pasted-images"
  // - Followed by a filename starting with "pasted-"
  // - Ending with common image extensions
  // - Word boundary or end of string
  const regex =
    /(?:^|\s)((?:[^\s]*\/)?goose-pasted-images\/pasted-[^\s]+\.(png|jpg|jpeg|gif|webp))(?=\s|$)/gi;

  const matches = [];
  let match;

  while ((match = regex.exec(text)) !== null) {
    matches.push(match[1]);
  }

  return matches;
}

/**
 * Removes image paths from the text
 *
 * @param text The original text
 * @param imagePaths Array of image paths to remove
 * @returns Text with image paths removed
 */
export function removeImagePathsFromText(text: string, imagePaths: string[]): string {
  if (!text || imagePaths.length === 0) return text;

  let result = text;

  // Remove each image path from the text
  imagePaths.forEach((path) => {
    // Escape special regex characters in the path
    const escapedPath = path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    // Create a regex that matches the path with optional surrounding whitespace
    const pathRegex = new RegExp(`(^|\\s)${escapedPath}(?=\\s|$)`, 'g');
    result = result.replace(pathRegex, '$1');
  });

  // Clean up any extra whitespace
  return result.replace(/\s+/g, ' ').trim();
}
