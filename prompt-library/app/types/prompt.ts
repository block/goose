export interface Prompt {
  id: string;
  title: string;
  description: string;
  category: string;
  function: string;  // Changed from 'role' to 'function'
  prompt: string;
  example: string;
  tags: string[];
  author: string;
  dateAdded: string;
  lastUpdated: string;
  rating: number;
  usageCount: number;
  verified: boolean;
  extensions: string[];
  external?: boolean;
  extensionUrl?: string;
  variables: {
    name: string;
    description: string;
    required: boolean;
    type: string;
  }[];
}