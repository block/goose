/**
 * Utility for fetching GitHub repository star counts dynamically
 */

interface GitHubStarsCache {
  stars: number;
  timestamp: number;
}

const CACHE_DURATION = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

/**
 * Extract owner/repo from GitHub URL
 */
function extractRepoFromUrl(repoUrl: string): string | null {
  if (!repoUrl) return null;
  
  const match = repoUrl.match(/^https?:\/\/(www\.)?github\.com\/([^\/]+\/[^\/]+)/);
  if (!match) return null;
  
  // Clean up any trailing paths (tree/main, blob/main, etc.)
  const repo = match[2].replace(/\/(tree|blob)\/.*$/, '').replace(/[#?].*$/, '');
  return repo;
}

/**
 * Get cached star count if valid
 */
function getCachedStars(repo: string): number | null {
  try {
    const cacheKey = `github-stars-${repo}`;
    const cached = localStorage.getItem(cacheKey);
    
    if (!cached) return null;
    
    const { stars, timestamp }: GitHubStarsCache = JSON.parse(cached);
    
    // Check if cache is still valid (24 hours)
    if (Date.now() - timestamp < CACHE_DURATION) {
      return stars;
    }
    
    // Cache expired, remove it
    localStorage.removeItem(cacheKey);
    return null;
  } catch {
    return null;
  }
}

/**
 * Cache star count
 */
function setCachedStars(repo: string, stars: number): void {
  try {
    const cacheKey = `github-stars-${repo}`;
    const cacheData: GitHubStarsCache = {
      stars,
      timestamp: Date.now()
    };
    localStorage.setItem(cacheKey, JSON.stringify(cacheData));
  } catch {
    // Ignore localStorage errors (e.g., quota exceeded)
  }
}

/**
 * Fetch GitHub stars for a repository
 * Returns null if the API call fails (for hiding the star display)
 */
export async function fetchGitHubStars(repoUrl: string): Promise<number | null> {
  const repo = extractRepoFromUrl(repoUrl);
  if (!repo) return null;
  
  const cachedStars = getCachedStars(repo);
  if (cachedStars !== null) {
    return cachedStars;
  }
  
  try {
    const response = await fetch(`https://api.github.com/repos/${repo}`, {
      headers: {
        'Accept': 'application/vnd.github.v3+json',
      }
    });
    
    if (!response.ok) {
      return null;
    }
    
    const data = await response.json();
    const stars = data.stargazers_count || 0;
    
    setCachedStars(repo, stars);
    
    return stars;
  } catch {
    return null;
  }
}

/**
 * Format star count for display
 */
export function formatStarCount(stars: number): string {
  if (stars >= 1000) {
    return `${(stars / 1000).toFixed(1)}k`;
  }
  return stars.toString();
}
