/**
 * LLMTitleGenerationService - Generates contextual titles for Matrix rooms based on conversation content
 * 
 * This service analyzes Matrix room conversations and generates meaningful, human-readable titles
 * that reflect the actual content and context of the discussions.
 */

import { matrixService } from './MatrixService';

export interface TitleGenerationOptions {
  maxMessages?: number;
  includeParticipants?: boolean;
  roomType?: 'dm' | 'group' | 'collaborative';
  fallbackName?: string;
}

export interface GeneratedTitle {
  title: string;
  confidence: 'high' | 'medium' | 'low';
  source: 'llm' | 'content_analysis' | 'fallback';
  generatedAt: number;
  roomType: 'dm' | 'group' | 'collaborative';
}

export class LLMTitleGenerationService {
  private static instance: LLMTitleGenerationService;
  private readonly CACHE_KEY = 'goose-matrix-room-titles';
  private titleCache: Map<string, GeneratedTitle> = new Map();
  private readonly CACHE_DURATION = 24 * 60 * 60 * 1000; // 24 hours

  private constructor() {
    this.loadTitlesFromCache();
  }

  public static getInstance(): LLMTitleGenerationService {
    if (!LLMTitleGenerationService.instance) {
      LLMTitleGenerationService.instance = new LLMTitleGenerationService();
    }
    return LLMTitleGenerationService.instance;
  }

  /**
   * Generate a contextual title for a Matrix room
   */
  public async generateRoomTitle(
    roomId: string, 
    options: TitleGenerationOptions = {}
  ): Promise<GeneratedTitle> {
    const {
      maxMessages = 20,
      includeParticipants = true,
      roomType = 'group',
      fallbackName
    } = options;

    // Check cache first
    const cached = this.getCachedTitle(roomId);
    if (cached && this.isCacheValid(cached)) {
      console.log('📝 Using cached title for room:', roomId.substring(0, 20) + '...');
      return cached;
    }

    try {
      // Get room information
      const rooms = matrixService.getRooms();
      const room = rooms.find(r => r.roomId === roomId);
      
      if (!room) {
        return this.createFallbackTitle(roomId, roomType, fallbackName);
      }

      // Get recent messages for context
      const messages = await matrixService.getRoomHistory(roomId, maxMessages);
      
      if (messages.length === 0) {
        return this.createFallbackTitle(roomId, roomType, room.name || fallbackName);
      }

      // Determine room type if not provided
      const detectedRoomType = this.detectRoomType(room, messages);
      const finalRoomType = roomType === 'group' ? detectedRoomType : roomType;

      // Try LLM-based generation first
      const llmTitle = await this.generateTitleWithLLM(room, messages, finalRoomType, includeParticipants);
      if (llmTitle) {
        const generatedTitle: GeneratedTitle = {
          title: llmTitle,
          confidence: 'high',
          source: 'llm',
          generatedAt: Date.now(),
          roomType: finalRoomType,
        };
        
        this.cacheTitle(roomId, generatedTitle);
        return generatedTitle;
      }

      // Fallback to content analysis
      const contentTitle = this.generateTitleFromContent(room, messages, finalRoomType);
      const generatedTitle: GeneratedTitle = {
        title: contentTitle,
        confidence: 'medium',
        source: 'content_analysis',
        generatedAt: Date.now(),
        roomType: finalRoomType,
      };
      
      this.cacheTitle(roomId, generatedTitle);
      return generatedTitle;

    } catch (error) {
      console.error('📝 Error generating room title:', error);
      return this.createFallbackTitle(roomId, roomType, fallbackName);
    }
  }

  /**
   * Generate title using LLM analysis
   */
  private async generateTitleWithLLM(
    room: any, 
    messages: any[], 
    roomType: 'dm' | 'group' | 'collaborative',
    includeParticipants: boolean
  ): Promise<string | null> {
    try {
      // Prepare conversation context
      const recentMessages = messages.slice(-10); // Last 10 messages for context
      const messageContext = recentMessages.map(msg => {
        const sender = msg.senderInfo.displayName || msg.senderInfo.userId.split(':')[0].substring(1);
        return `${sender}: ${msg.content.substring(0, 150)}`;
      }).join('\n');

      const participantInfo = includeParticipants 
        ? `Participants: ${room.members.map(m => m.displayName || m.userId.split(':')[0].substring(1)).join(', ')}`
        : '';

      const roomTypeContext = {
        dm: 'This is a direct message conversation between two people.',
        group: 'This is a group chat with multiple participants.',
        collaborative: 'This is a collaborative AI session where users work together with AI assistance.'
      }[roomType];

      // Create prompt for title generation
      const prompt = `Analyze this ${roomType === 'dm' ? 'direct message conversation' : 'group chat conversation'} and generate a short, descriptive title (2-6 words) that captures the main topic or theme.

${roomTypeContext}
${participantInfo}

Recent conversation:
${messageContext}

Requirements:
- Title should be 2-6 words maximum
- Should reflect the main topic or activity
- Should be friendly and conversational
- Don't include participant names unless it's about them specifically
- Examples of good titles: "Weekend Plans Discussion", "Code Review Help", "Recipe Sharing", "Travel Tips", "Project Planning"

Generate only the title, nothing else:`;

      // Use the existing Matrix service to send a request to the LLM
      // Note: This is a simplified approach - in a real implementation, you might want to use a dedicated LLM API
      const response = await this.callLLMForTitle(prompt);
      
      if (response && response.length > 0 && response.length <= 50) {
        // Clean up the response
        const cleanTitle = response
          .replace(/^["']|["']$/g, '') // Remove quotes
          .replace(/^Title:\s*/i, '') // Remove "Title:" prefix
          .trim();
        
        return cleanTitle;
      }

      return null;
    } catch (error) {
      console.error('📝 LLM title generation failed:', error);
      return null;
    }
  }

  /**
   * Simple LLM call for title generation using Goose backend
   * RATE LIMITING FIX: Add throttling and error handling
   */
  private async callLLMForTitle(prompt: string): Promise<string | null> {
    try {
      console.log('📝 Making LLM API call for title generation...');
      
      // RATE LIMITING FIX: Add a simple throttle mechanism
      const now = Date.now();
      const lastCallKey = 'llm-title-last-call';
      const lastCall = parseInt(localStorage.getItem(lastCallKey) || '0');
      const minInterval = 5000; // 5 seconds between calls
      
      if (now - lastCall < minInterval) {
        console.log('📝 Rate limiting: Skipping LLM call, too soon after last call');
        return null;
      }
      
      localStorage.setItem(lastCallKey, now.toString());
      
      // Create a temporary session for title generation
      const { startAgent } = await import('../api');
      
      const agentResponse = await startAgent({
        body: {
          working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
          recipe: {
            title: 'Matrix Room Title Generator',
            description: 'Generate contextual titles for Matrix chat rooms',
            instructions: 'You are a helpful assistant that generates short, descriptive titles for chat conversations. Always respond with just the title, nothing else.',
          },
        },
        throwOnError: false,
      });

      if (!agentResponse.data?.id) {
        console.error('📝 Failed to create title generation session');
        return null;
      }

      const sessionId = agentResponse.data.id;
      
      // Send the title generation prompt
      const response = await fetch('/api/chat/reply', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Secret-Key': await window.electron.getSecretKey(),
        },
        body: JSON.stringify({
          messages: [
            {
              role: 'user',
              content: [{ type: 'text', text: prompt }],
              created: Math.floor(Date.now() / 1000),
            }
          ],
          session_id: sessionId,
        }),
      });

      if (!response.ok) {
        // RATE LIMITING FIX: Better error handling for different HTTP status codes
        if (response.status === 429) {
          console.warn('📝 Rate limit exceeded for title generation - will use fallback');
        } else if (response.status === 402 || response.status === 403) {
          console.warn('📝 API quota/auth issue for title generation - will use fallback');
        } else {
          console.error('📝 Title generation API call failed:', response.status, response.statusText);
        }
        return null;
      }

      // Process the SSE stream to get the response
      const reader = response.body?.getReader();
      if (!reader) {
        console.error('📝 No response body for title generation');
        return null;
      }

      const decoder = new TextDecoder();
      let buffer = '';
      let generatedTitle = '';

      try {
        let running = true;
        while (running) {
          const { done, value } = await reader.read();
          if (done) {
            running = false;
            break;
          }

          buffer += decoder.decode(value, { stream: true });
          const events = buffer.split('\n\n');
          buffer = events.pop() || '';

          for (const event of events) {
            if (event.startsWith('data: ')) {
              try {
                const data = event.slice(6);
                const parsedEvent = JSON.parse(data);

                if (parsedEvent.type === 'Message' && parsedEvent.message.role === 'assistant') {
                  // Extract text content from the message
                  for (const content of parsedEvent.message.content) {
                    if (content.type === 'text') {
                      generatedTitle += content.text;
                    }
                  }
                } else if (parsedEvent.type === 'Finish') {
                  running = false;
                  break;
                } else if (parsedEvent.type === 'Error') {
                  console.error('📝 Error in title generation stream:', parsedEvent.error);
                  return null;
                }
              } catch (e) {
                console.error('📝 Error parsing title generation event:', e);
              }
            }
          }
        }
      } finally {
        reader.releaseLock();
      }

      // Clean up the generated title
      const cleanTitle = generatedTitle
        .replace(/^["']|["']$/g, '') // Remove quotes
        .replace(/^Title:\s*/i, '') // Remove "Title:" prefix
        .replace(/\n/g, ' ') // Replace newlines with spaces
        .trim();

      // Validate title length and content
      if (cleanTitle && cleanTitle.length > 0 && cleanTitle.length <= 50) {
        console.log('📝 Successfully generated title:', cleanTitle);
        
        // Clean up the temporary session (optional, but good practice)
        try {
          const { deleteSession } = await import('../api');
          await deleteSession({
            path: { session_id: sessionId },
            throwOnError: false,
          });
        } catch (cleanupError) {
          console.warn('📝 Failed to cleanup title generation session:', cleanupError);
        }
        
        return cleanTitle;
      } else {
        console.warn('📝 Generated title is invalid:', { title: cleanTitle, length: cleanTitle.length });
        return null;
      }

    } catch (error) {
      console.error('📝 LLM title generation failed:', error);
      return null;
    }
  }

  /**
   * Generate title from content analysis (fallback method)
   */
  private generateTitleFromContent(
    room: any, 
    messages: any[], 
    roomType: 'dm' | 'group' | 'collaborative'
  ): string {
    // Use existing room name if it's meaningful
    if (room.name && !room.name.startsWith('!') && room.name.length > 3) {
      return this.addRoomTypePrefix(room.name, roomType);
    }

    // Analyze message content for keywords
    const recentMessages = messages.slice(-15);
    const allText = recentMessages.map(m => m.content).join(' ').toLowerCase();
    
    // Common topic patterns
    const topicPatterns = [
      { pattern: /\b(code|coding|programming|debug|bug|error|function|class|variable)\b/g, title: 'Code Discussion' },
      { pattern: /\b(meeting|schedule|calendar|appointment|time|date)\b/g, title: 'Meeting Planning' },
      { pattern: /\b(project|task|work|deadline|progress|status)\b/g, title: 'Project Work' },
      { pattern: /\b(help|question|problem|issue|stuck|confused)\b/g, title: 'Help & Support' },
      { pattern: /\b(food|recipe|cook|eat|restaurant|meal)\b/g, title: 'Food & Recipes' },
      { pattern: /\b(travel|trip|vacation|flight|hotel|visit)\b/g, title: 'Travel Plans' },
      { pattern: /\b(game|gaming|play|fun|entertainment)\b/g, title: 'Gaming & Fun' },
      { pattern: /\b(music|song|album|artist|concert)\b/g, title: 'Music Chat' },
      { pattern: /\b(movie|film|show|watch|netflix|series)\b/g, title: 'Movies & Shows' },
      { pattern: /\b(book|read|reading|author|novel)\b/g, title: 'Book Discussion' },
      { pattern: /\b(weather|rain|snow|sunny|cold|hot)\b/g, title: 'Weather Talk' },
      { pattern: /\b(news|politics|world|country|government)\b/g, title: 'News & Politics' },
    ];

    // Find matching patterns
    for (const { pattern, title } of topicPatterns) {
      const matches = allText.match(pattern);
      if (matches && matches.length >= 3) {
        return this.addRoomTypePrefix(title, roomType);
      }
    }

    // Check for collaborative AI patterns
    if (roomType === 'collaborative') {
      const aiPatterns = /\b(ai|assistant|goose|help|task|collaborate|work together)\b/g;
      if (allText.match(aiPatterns)) {
        return 'AI Collaboration';
      }
    }

    // Generate based on participants and room type
    const participantCount = room.members.length;
    
    if (roomType === 'dm') {
      const otherParticipant = room.members.find(m => !m.userId.includes('goose'));
      const name = otherParticipant?.displayName || 'User';
      return `Chat with ${name}`;
    } else if (roomType === 'collaborative') {
      return `Team Collaboration (${participantCount} members)`;
    } else {
      return `Group Chat (${participantCount} members)`;
    }
  }

  /**
   * Add appropriate prefix based on room type
   */
  private addRoomTypePrefix(title: string, roomType: 'dm' | 'group' | 'collaborative'): string {
    // Don't add prefix if title already indicates type
    const hasTypeIndicator = /^(dm|chat|group|collab|team|direct)/i.test(title);
    if (hasTypeIndicator) {
      return title;
    }

    switch (roomType) {
      case 'dm':
        return title; // DMs don't need prefix
      case 'collaborative':
        return `Collab: ${title}`;
      case 'group':
        return `Group: ${title}`;
      default:
        return title;
    }
  }

  /**
   * Detect room type based on room properties and messages
   */
  private detectRoomType(room: any, messages: any[]): 'dm' | 'group' | 'collaborative' {
    const memberCount = room.members.length;
    
    // Check for AI/Goose participants
    const hasAI = room.members.some(m => 
      m.userId.toLowerCase().includes('goose') || 
      m.displayName?.toLowerCase().includes('goose') ||
      m.displayName?.toLowerCase().includes('ai') ||
      m.displayName?.toLowerCase().includes('assistant')
    );

    // Check message patterns for AI collaboration
    const recentMessages = messages.slice(-10);
    const hasAIMessages = recentMessages.some(msg => 
      msg.type === 'assistant' || 
      msg.content.includes('🤖') ||
      msg.content.includes('🦆') ||
      msg.senderInfo.displayName?.toLowerCase().includes('goose')
    );

    if (memberCount === 2) {
      return 'dm';
    } else if (hasAI || hasAIMessages) {
      return 'collaborative';
    } else {
      return 'group';
    }
  }

  /**
   * Create fallback title when generation fails
   */
  private createFallbackTitle(
    roomId: string, 
    roomType: 'dm' | 'group' | 'collaborative',
    fallbackName?: string
  ): GeneratedTitle {
    let title: string;
    
    if (fallbackName) {
      title = fallbackName;
    } else {
      const roomIdShort = roomId.substring(1, 8);
      switch (roomType) {
        case 'dm':
          title = `Direct Message ${roomIdShort}`;
          break;
        case 'collaborative':
          title = `AI Session ${roomIdShort}`;
          break;
        default:
          title = `Group Chat ${roomIdShort}`;
      }
    }

    return {
      title,
      confidence: 'low',
      source: 'fallback',
      generatedAt: Date.now(),
      roomType,
    };
  }

  /**
   * Get cached title if available and valid
   */
  private getCachedTitle(roomId: string): GeneratedTitle | null {
    return this.titleCache.get(roomId) || null;
  }

  /**
   * Check if cached title is still valid
   */
  private isCacheValid(title: GeneratedTitle): boolean {
    return (Date.now() - title.generatedAt) < this.CACHE_DURATION;
  }

  /**
   * Cache a generated title
   */
  private cacheTitle(roomId: string, title: GeneratedTitle): void {
    this.titleCache.set(roomId, title);
    this.saveTitlesToCache();
  }

  /**
   * Load titles from localStorage
   */
  private loadTitlesFromCache(): void {
    try {
      const stored = localStorage.getItem(this.CACHE_KEY);
      if (stored) {
        const data = JSON.parse(stored);
        this.titleCache = new Map(Object.entries(data));
        console.log(`📝 Loaded ${this.titleCache.size} cached titles`);
      }
    } catch (error) {
      console.error('📝 Error loading title cache:', error);
      this.titleCache = new Map();
    }
  }

  /**
   * Save titles to localStorage
   */
  private saveTitlesToCache(): void {
    try {
      const data = Object.fromEntries(this.titleCache.entries());
      localStorage.setItem(this.CACHE_KEY, JSON.stringify(data));
    } catch (error) {
      console.error('📝 Error saving title cache:', error);
    }
  }

  /**
   * Clear expired titles from cache
   */
  public cleanupExpiredTitles(): void {
    const now = Date.now();
    let removedCount = 0;

    for (const [roomId, title] of this.titleCache.entries()) {
      if ((now - title.generatedAt) > this.CACHE_DURATION) {
        this.titleCache.delete(roomId);
        removedCount++;
      }
    }

    if (removedCount > 0) {
      this.saveTitlesToCache();
      console.log(`📝 Cleaned up ${removedCount} expired titles`);
    }
  }

  /**
   * Force regenerate title for a room
   */
  public async regenerateTitle(roomId: string, options: TitleGenerationOptions = {}): Promise<GeneratedTitle> {
    // Remove from cache to force regeneration
    this.titleCache.delete(roomId);
    return this.generateRoomTitle(roomId, options);
  }

  /**
   * Get all cached titles (for debugging)
   */
  public getAllCachedTitles(): Map<string, GeneratedTitle> {
    return new Map(this.titleCache);
  }

  /**
   * Clear all cached titles
   */
  public clearAllTitles(): void {
    this.titleCache.clear();
    localStorage.removeItem(this.CACHE_KEY);
    console.log('📝 Cleared all cached titles');
  }
}

// Export singleton instance
export const llmTitleGenerationService = LLMTitleGenerationService.getInstance();
