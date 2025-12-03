/**
 * Matrix Room Helper
 * 
 * This utility provides functions for creating and managing Matrix room sessions.
 * Use these functions when opening Matrix rooms to ensure proper session management
 * and message routing.
 */

import { ChatType } from '../types/chat';
import { generateMatrixSessionId } from '../sessions';

export interface MatrixRoomInfo {
  roomId: string;
  name: string;
  recipientId?: string;
}

/**
 * Creates a ChatType object for a Matrix room with proper session management
 * 
 * This ensures:
 * 1. Each Matrix room gets a unique session ID
 * 2. The session ID is stable (same room = same session)
 * 3. Matrix metadata is properly attached
 * 4. Messages will be routed correctly
 * 
 * @param room - The Matrix room information
 * @returns A ChatType object ready to be used for the Matrix room
 * 
 * @example
 * ```typescript
 * const room = {
 *   roomId: '!KxKXXYDKFfbKQXgDXO:tchncs.de',
 *   name: 'Room Alpha',
 *   recipientId: '@user:tchncs.de'
 * };
 * 
 * const chat = createMatrixRoomChat(room);
 * // Now use this chat object to open a tab or navigate to pair view
 * ```
 */
export function createMatrixRoomChat(room: MatrixRoomInfo): ChatType {
  // Generate unique session ID for this Matrix room
  const sessionId = generateMatrixSessionId(room.roomId);
  
  console.log('[Matrix Room Helper] Creating chat for Matrix room:', {
    roomId: room.roomId,
    roomName: room.name,
    sessionId,
    timestamp: new Date().toISOString()
  });
  
  return {
    id: sessionId,
    title: room.name,
    messages: [],
    messageHistoryIndex: 0,
    matrixRoomId: room.roomId,
    matrixRecipientId: room.recipientId || null,
    isMatrixTab: true,
    recipeConfig: null,
    recipeParameters: null
  };
}

/**
 * Checks if a chat is a Matrix room chat
 * 
 * @param chat - The chat object to check
 * @returns true if the chat is a Matrix room
 */
export function isMatrixRoom(chat: ChatType): boolean {
  return chat.isMatrixTab === true && !!chat.matrixRoomId;
}

/**
 * Gets the Matrix room ID from a chat object
 * 
 * @param chat - The chat object
 * @returns The Matrix room ID or null if not a Matrix chat
 */
export function getMatrixRoomId(chat: ChatType): string | null {
  return chat.matrixRoomId || null;
}

/**
 * Example usage in a component that opens Matrix rooms:
 * 
 * ```typescript
 * import { createMatrixRoomChat } from '../utils/matrixRoomHelper';
 * import { useNavigate } from 'react-router-dom';
 * 
 * const SpaceRoomsView = ({ spaceId }) => {
 *   const navigate = useNavigate();
 *   
 *   const handleOpenRoom = (room) => {
 *     // Create chat object with proper Matrix session
 *     const chat = createMatrixRoomChat({
 *       roomId: room.roomId,
 *       name: room.name,
 *       recipientId: room.recipientId
 *     });
 *     
 *     // Navigate to pair view with the Matrix chat
 *     navigate('/pair', { state: { chat } });
 *     
 *     // OR if using a tab system:
 *     // addTab(chat);
 *   };
 *   
 *   return (
 *     <div>
 *       {rooms.map(room => (
 *         <button key={room.roomId} onClick={() => handleOpenRoom(room)}>
 *           {room.name}
 *         </button>
 *       ))}
 *     </div>
 *   );
 * };
 * ```
 */
