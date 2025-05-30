// Helper to normalize URLs for comparison
function normalizeUrl(url: string): string {
  try {
    const parsed = new URL(url.toLowerCase());
    // Remove trailing slashes and normalize protocol
    return `${parsed.protocol}//${parsed.host}${parsed.pathname.replace(/\/$/, '')}${parsed.search}${parsed.hash}`;
  } catch {
    // If URL parsing fails, just lowercase it
    return url.toLowerCase();
  }
}

// Helper to determine if a link should be included in results
function linkIsEligible(url: string): boolean {
  try {
    const parsed = new URL(url.toLowerCase());
    const ineligibleHosts = ['localhost', '127.0.0.1'];
    return !ineligibleHosts.some((host) => parsed.hostname.includes(host));
  } catch {
    return false;
  }
}

export function extractUrls(content: string, previousUrls: string[] = []): string[] {
  let remainingContent = content;
  const extractedUrls: string[] = [];

  // First, extract markdown links
  const markdownLinkRegex = /\[([^\]]+)\]\((https?:\/\/[^)]+)\)/g;
  const markdownMatches = Array.from(content.matchAll(markdownLinkRegex));

  // Add markdown URLs to our results and remove them from the content
  markdownMatches.forEach((match) => {
    extractedUrls.push(match[2]);
    // Replace the entire markdown link with whitespace to preserve string indices
    remainingContent = remainingContent.replace(match[0], ' '.repeat(match[0].length));
  });

  // Now look for standalone URLs in the remaining content
  const urlRegex = /(https?:\/\/[^\s<>"']+)/g;
  const urlMatches = Array.from(remainingContent.matchAll(urlRegex));
  const standardUrls = urlMatches.map((match) => match[1]);
  extractedUrls.push(...standardUrls);

  // Remove duplicates
  const uniqueCurrentUrls = [...new Set(extractedUrls)];

  // Filter out ineligible URLs
  const eligibleUrls = uniqueCurrentUrls.filter(linkIsEligible);

  // Normalize all URLs for comparison
  const normalizedPreviousUrls = previousUrls.map(normalizeUrl);
  const normalizedCurrentUrls = eligibleUrls.map(normalizeUrl);

  // Filter out duplicates from previous URLs
  const uniqueUrls = eligibleUrls.filter((_url, index) => {
    const normalized = normalizedCurrentUrls[index];
    const isDuplicate = normalizedPreviousUrls.some(
      (prevUrl) => normalizeUrl(prevUrl) === normalized
    );
    return !isDuplicate;
  });

  return uniqueUrls;
}
