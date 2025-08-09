import type { MCPServer } from "../types/server";

// Get the servers.json URL based on the current environment
function getServersUrl(): string {
  if (typeof window !== 'undefined') {
    // Client-side: construct URL based on current location
    const pathname = window.location.pathname;
    const hostname = window.location.hostname;
    
    // Check if we're in local development
    if (hostname === 'localhost' || hostname === '127.0.0.1') {
      // Local development - Docusaurus serves static files from the baseUrl
      // Since baseUrl is /goose/, static files are served from /goose/
      return '/goose/servers.json';
    }
    
    // Production or PR preview - extract the base path
    if (pathname.includes('/goose/')) {
      const gooseIndex = pathname.indexOf('/goose/');
      const afterGoose = pathname.substring(gooseIndex + 6); // +6 for "/goose/"
      
      if (afterGoose.startsWith('pr-preview/')) {
        // For PR previews like /goose/pr-preview/pr-123/
        const prMatch = afterGoose.match(/^pr-preview\/pr-\d+\//);
        if (prMatch) {
          return pathname.substring(0, gooseIndex + 6) + prMatch[0] + 'servers.json';
        }
      }
      // For production /goose/
      return pathname.substring(0, gooseIndex + 6) + 'servers.json';
    }
    
    // Fallback
    return '/servers.json';
  }
  
  // Server-side rendering - use relative path that works with baseUrl
  return '/goose/servers.json';
}

export async function fetchMCPServers(): Promise<MCPServer[]> {
  try {
    const serversUrl = getServersUrl();
    const response = await fetch(serversUrl);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching MCP servers:", error);
    throw error;
  }
}

export async function searchMCPServers(query: string): Promise<MCPServer[]> {
  const servers = await fetchMCPServers();
  const normalizedQuery = query.toLowerCase();
  
  return servers.filter((server) => {
    const normalizedName = server.name.toLowerCase();
    const normalizedDescription = server.description.toLowerCase();
    
    return (
      normalizedName.includes(normalizedQuery) ||
      normalizedDescription.includes(normalizedQuery)
    );
  });
}