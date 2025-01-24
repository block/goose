import type { MCPServer } from '../types/server';

export async function fetchMCPServers(): Promise<MCPServer[]> {
  const baseUrl = import.meta.env.VITE_BASENAME || "";
  try {
    // Fetch all servers from the unified JSON file
    const response = await fetch(`${baseUrl}servers.json`);
    if (!response.ok) {
      throw new Error('Failed to fetch servers');
    }
    
    const servers = await response.json();
    return servers.sort((a, b) => b.githubStars - a.githubStars);
  } catch (error) {
    console.error('Error fetching servers:', error);
    throw error;
  }
}

export async function searchMCPServers(query: string): Promise<MCPServer[]> {
  const allServers = await fetchMCPServers();
  const searchTerms = query.toLowerCase().split(' ').filter(term => term.length > 0);
  
  return allServers.filter(server => {
    const searchText = `${server.name} ${server.description}`.toLowerCase();
    return searchTerms.every(term => searchText.includes(term));
  });
}