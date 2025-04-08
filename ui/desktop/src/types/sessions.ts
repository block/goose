import { Message } from './message';

// Base session metadata interface
export interface SessionMetadata {
  description?: string;
  message_count: number;
  total_tokens: number | null;
  working_dir?: string;
}

// Core session details interface
export interface SessionDetails {
  id: string;
  path: string;
  created: string;
  modified: string;
  metadata: SessionMetadata;
  messages: Message[];
}

// Shared session details interface
export interface SharedSessionDetails {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  total_tokens: number;
}

// API response interfaces
export interface APISessionResponse {
  session_id: string;
  messages: Message[];
  metadata?: SessionMetadata;
}
