/**
 * Matrix Room Interceptor
 * 
 * This module intercepts Matrix room chat creation and ensures proper session IDs.
 * It works by monkey-patching the navigation and chat creation to use unique Matrix session IDs.
 */

import { ChatType } from './types/chat';
import { generateMatrixSessionId } from './sessions';

/**
 * Checks if a chat object looks like a Matrix room chat
 */
function looksLikeMatrixChat(chat: any): boolean {
  if (!chat || typeof chat !== 'object') return false;
  
  // Check for Matrix room ID pattern (!xxx:server.com)
  const hasMatrixRoomId = chat.matrixRoomId && typeof chat.matrixRoomId === 'string' && 
                          chat.matrixRoomId.startsWith('!') && chat.matrixRoomId.includes(':');
  
  // Check for title that might indicate a Matrix room
  const hasRoomTitle = chat.title && typeof chat.title === 'string';
  
  // Check if it's explicitly marked as a Matrix tab
  const isMarkedAsMatrix = chat.isMatrixTab === true;
  
  return hasMatrixRoomId || isMarkedAsMatrix;
}

/**
 * Fixes a Matrix chat object to have the correct session ID
 */
function fixMatrixChat(chat: any): ChatType {
  if (!looksLikeMatrixChat(chat)) {
    return chat;
  }
  
  // If it already has a proper Matrix session ID, don't change it
  if (chat.id && chat.id.includes('_matrix_')) {
    console.log('[Matrix Interceptor] Chat already has Matrix session ID:', chat.id);
    return chat;
  }
  
  // Generate proper Matrix session ID
  const matrixRoomId = chat.matrixRoomId || `!unknown_${Date.now()}:matrix.org`;
  const newSessionId = generateMatrixSessionId(matrixRoomId);
  
  console.log('[Matrix Interceptor] Fixing Matrix chat:', {
    oldSessionId: chat.id,
    newSessionId,
    matrixRoomId,
    title: chat.title
  });
  
  return {
    ...chat,
    id: newSessionId,
    matrixRoomId,
    isMatrixTab: true,
  };
}

/**
 * Intercepts window.history.pushState and replaceState to fix Matrix chats
 */
export function interceptMatrixRoomNavigation() {
  const originalPushState = window.history.pushState;
  const originalReplaceState = window.history.replaceState;
  
  window.history.pushState = function(state: any, title: string, url?: string | URL | null) {
    if (state && state.chat && looksLikeMatrixChat(state.chat)) {
      console.log('[Matrix Interceptor] Intercepting pushState with Matrix chat');
      state = {
        ...state,
        chat: fixMatrixChat(state.chat)
      };
    }
    return originalPushState.call(this, state, title, url);
  };
  
  window.history.replaceState = function(state: any, title: string, url?: string | URL | null) {
    if (state && state.chat && looksLikeMatrixChat(state.chat)) {
      console.log('[Matrix Interceptor] Intercepting replaceState with Matrix chat');
      state = {
        ...state,
        chat: fixMatrixChat(state.chat)
      };
    }
    return originalReplaceState.call(this, state, title, url);
  };
  
  console.log('[Matrix Interceptor] Navigation interception installed');
}

/**
 * Intercepts React Router navigate calls (if we can access them)
 */
export function interceptReactRouterNavigate() {
  // This will be called from App.tsx where we have access to navigate
  return (originalNavigate: any) => {
    return (to: any, options?: any) => {
      if (options && options.state && options.state.chat && looksLikeMatrixChat(options.state.chat)) {
        console.log('[Matrix Interceptor] Intercepting React Router navigate with Matrix chat');
        options = {
          ...options,
          state: {
            ...options.state,
            chat: fixMatrixChat(options.state.chat)
          }
        };
      }
      return originalNavigate(to, options);
    };
  };
}

/**
 * Initialize the Matrix room interceptor
 * Call this early in the app initialization
 */
export function initializeMatrixInterceptor() {
  console.log('[Matrix Interceptor] Initializing...');
  interceptMatrixRoomNavigation();
  console.log('[Matrix Interceptor] Ready to intercept Matrix room navigation');
}
