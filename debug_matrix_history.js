// Matrix History Debug Script
// Copy and paste this into your browser's developer console

// Store original console methods
const originalLog = console.log;
const originalError = console.error;
const originalWarn = console.warn;

// Create a filtered log collector for history-specific logs
const historyLogs = [];

// Override console methods to capture Matrix history-related logs
console.log = function(...args) {
  const message = args.join(' ');
  
  // Check for Matrix history-related log patterns
  if (message.includes('📜') ||
      message.includes('Event type breakdown:') ||
      message.includes('No message events found') ||
      message.includes('Examining first few events:') ||
      message.includes('Found') && message.includes('events in room timeline') ||
      message.includes('Found') && message.includes('message events') ||
      message.includes('Processing message') ||
      message.includes('Final result for message') ||
      message.includes('Message type breakdown:') ||
      message.includes('Processed') && message.includes('messages from room history') ||
      message.includes('useChatStream: Calling getRoomHistory') ||
      message.includes('useChatStream: Raw Matrix history response')) {
    
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = `[${timestamp}] ${message}`;
    historyLogs.push(logEntry);
    
    // Display in a highlighted way based on content
    let color = '#1976d2'; // Default blue
    let background = '#e3f2fd';
    
    if (message.includes('Event type breakdown:') || message.includes('No message events found')) {
      color = '#f57c00'; // Orange for important info
      background = '#fff3e0';
    } else if (message.includes('❌') || message.includes('Failed')) {
      color = '#d32f2f'; // Red for errors
      background = '#ffebee';
    } else if (message.includes('✅') || message.includes('Successfully')) {
      color = '#388e3c'; // Green for success
      background = '#e8f5e9';
    }
    
    originalLog('%c' + logEntry, `background: ${background}; color: ${color}; padding: 2px 4px; border-radius: 3px;`);
  } else {
    // Call original log for non-Matrix history logs
    originalLog.apply(console, arguments);
  }
};

// Also capture errors and warnings
console.error = function(...args) {
  const message = args.join(' ');
  if (message.includes('📜') || message.includes('Matrix') || message.includes('matrix') || message.includes('getRoomHistory')) {
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = `[${timestamp}] ERROR: ${message}`;
    historyLogs.push(logEntry);
    originalLog('%c' + logEntry, 'background: #ffebee; color: #d32f2f; padding: 2px 4px; border-radius: 3px;');
  }
  originalError.apply(console, arguments);
};

console.warn = function(...args) {
  const message = args.join(' ');
  if (message.includes('📜') || message.includes('Matrix') || message.includes('matrix') || message.includes('getRoomHistory')) {
    const timestamp = new Date().toLocaleTimeString();
    const logEntry = `[${timestamp}] WARN: ${message}`;
    historyLogs.push(logEntry);
    originalLog('%c' + logEntry, 'background: #fff3e0; color: #f57c00; padding: 2px 4px; border-radius: 3px;');
  }
  originalWarn.apply(console, arguments);
};

// Helper functions
window.showHistoryLogs = function() {
  console.group('📜 All Matrix History Logs');
  historyLogs.forEach(log => originalLog(log));
  console.groupEnd();
};

window.clearHistoryLogs = function() {
  historyLogs.length = 0;
  originalLog('🧹 Matrix history logs cleared');
};

window.restoreHistoryConsole = function() {
  console.log = originalLog;
  console.error = originalError;
  console.warn = originalWarn;
  originalLog('🔄 Console restored to original state');
};

// Function to manually trigger room history fetch for debugging
window.debugRoomHistory = async function(roomId) {
  if (!roomId) {
    // Try to find a Matrix room ID automatically
    const matrixService = window.matrixService;
    if (matrixService && matrixService.client) {
      const rooms = matrixService.client.getRooms();
      const joinedRooms = rooms.filter(room => room.getMyMembership() === 'join');
      if (joinedRooms.length > 0) {
        roomId = joinedRooms[0].roomId;
        originalLog('🔍 Auto-selected room for debugging:', roomId.substring(0, 20) + '...');
      }
    }
  }
  
  if (!roomId) {
    originalLog('❌ No room ID provided and could not auto-select. Usage: debugRoomHistory("!roomId:server.com")');
    return;
  }
  
  originalLog('🔍 Manually triggering room history fetch for:', roomId.substring(0, 20) + '...');
  
  try {
    const matrixService = window.matrixService;
    if (!matrixService) {
      originalLog('❌ matrixService not available on window');
      return;
    }
    
    const history = await matrixService.getRoomHistory(roomId, 20);
    originalLog('🔍 Manual room history result:', history);
    
    return history;
  } catch (error) {
    originalLog('❌ Manual room history fetch failed:', error);
  }
};

// Function to inspect a specific room's events
window.inspectRoomEvents = function(roomId) {
  if (!roomId) {
    originalLog('❌ No room ID provided. Usage: inspectRoomEvents("!roomId:server.com")');
    return;
  }
  
  const matrixService = window.matrixService;
  if (!matrixService || !matrixService.client) {
    originalLog('❌ matrixService or client not available');
    return;
  }
  
  const room = matrixService.client.getRoom(roomId);
  if (!room) {
    originalLog('❌ Room not found:', roomId);
    return;
  }
  
  const timeline = room.getLiveTimeline();
  const events = timeline.getEvents();
  
  originalLog('🔍 Room inspection for:', roomId.substring(0, 20) + '...');
  originalLog('🔍 Total events:', events.length);
  
  // Count event types
  const eventTypes = events.map(event => event.getType());
  const eventTypeCounts = eventTypes.reduce((acc, type) => {
    acc[type] = (acc[type] || 0) + 1;
    return acc;
  }, {});
  
  originalLog('🔍 Event type breakdown:', eventTypeCounts);
  
  // Show first few events in detail
  originalLog('🔍 First 5 events in detail:');
  events.slice(0, Math.min(5, events.length)).forEach((event, index) => {
    const content = event.getContent();
    originalLog(`Event ${index + 1}:`, {
      type: event.getType(),
      sender: event.getSender(),
      timestamp: new Date(event.getTs()).toISOString(),
      content: content,
      hasBody: !!content.body,
      msgtype: content.msgtype,
      bodyPreview: content.body ? content.body.substring(0, 100) + '...' : 'No body'
    });
  });
  
  // Filter message events specifically
  const messageEvents = events.filter(event => event.getType() === 'm.room.message');
  originalLog('🔍 Message events found:', messageEvents.length);
  
  if (messageEvents.length > 0) {
    originalLog('🔍 First few message events:');
    messageEvents.slice(0, Math.min(3, messageEvents.length)).forEach((event, index) => {
      const content = event.getContent();
      originalLog(`Message ${index + 1}:`, {
        sender: event.getSender(),
        timestamp: new Date(event.getTs()).toISOString(),
        body: content.body,
        msgtype: content.msgtype,
        gooseType: content['goose.message.type'] || content['goose.type'],
        sessionMessage: content.body?.includes('goose-session-message:')
      });
    });
  }
  
  return {
    totalEvents: events.length,
    eventTypeCounts,
    messageEvents: messageEvents.length,
    events: events.slice(0, 10) // Return first 10 events for further inspection
  };
};

// Function to list all joined Matrix rooms
window.listMatrixRooms = function() {
  const matrixService = window.matrixService;
  if (!matrixService || !matrixService.client) {
    originalLog('❌ matrixService or client not available');
    return;
  }
  
  const rooms = matrixService.client.getRooms();
  const joinedRooms = rooms.filter(room => room.getMyMembership() === 'join');
  
  originalLog('🏠 All joined Matrix rooms:');
  joinedRooms.forEach((room, index) => {
    const timeline = room.getLiveTimeline();
    const events = timeline.getEvents();
    const messageEvents = events.filter(event => event.getType() === 'm.room.message');
    
    originalLog(`${index + 1}. ${room.name || 'Unnamed'} (${room.roomId.substring(0, 20)}...)`, {
      roomId: room.roomId,
      name: room.name,
      members: room.getMembers().length,
      totalEvents: events.length,
      messageEvents: messageEvents.length,
      lastActivity: new Date(room.getLastActiveTimestamp()).toISOString()
    });
  });
  
  return joinedRooms.map(room => ({
    roomId: room.roomId,
    name: room.name,
    shortId: room.roomId.substring(0, 20) + '...'
  }));
};

// Initial setup message
originalLog('%c📜 Matrix History Debug Logger Active', 'background: #4caf50; color: white; padding: 4px 8px; border-radius: 4px; font-weight: bold;');
originalLog('Available commands:');
originalLog('  showHistoryLogs() - Show all captured history logs');
originalLog('  clearHistoryLogs() - Clear the history log buffer');
originalLog('  restoreHistoryConsole() - Restore original console behavior');
originalLog('  debugRoomHistory(roomId) - Manually fetch room history (auto-selects if no roomId)');
originalLog('  inspectRoomEvents(roomId) - Inspect raw events in a room');
originalLog('  listMatrixRooms() - List all joined Matrix rooms with event counts');
originalLog('');
originalLog('Try: listMatrixRooms() to see available rooms, then inspectRoomEvents(roomId)');
originalLog('Now try opening a Matrix room and the relevant logs will be highlighted...');
