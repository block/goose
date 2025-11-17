
// Debug Matrix history loading for DM room
const roomId = '!dECWvepUhwCyZwBjhL:tchncs.de';

console.log('🔍 MATRIX HISTORY DEBUG:');
console.log('Room ID:', roomId);

// Check if getRoomHistoryAsGooseMessages is available
console.log('getRoomHistoryAsGooseMessages available:', typeof getRoomHistoryAsGooseMessages);

// Try to load more history manually
if (typeof getRoomHistoryAsGooseMessages === 'function') {
  console.log('📜 Attempting to load 500 messages from Matrix room...');
  
  getRoomHistoryAsGooseMessages(roomId, 500).then(messages => {
    console.log('📜 Matrix history result:', {
      totalMessages: messages.length,
      firstMessage: messages[0] ? {
        content: messages[0].content.substring(0, 50) + '...',
        timestamp: messages[0].timestamp,
        sender: messages[0].sender
      } : 'none',
      lastMessage: messages[messages.length - 1] ? {
        content: messages[messages.length - 1].content.substring(0, 50) + '...',
        timestamp: messages[messages.length - 1].timestamp,
        sender: messages[messages.length - 1].sender
      } : 'none'
    });
    
    // Check message age spread
    if (messages.length > 1) {
      const oldestTime = messages[0].timestamp.getTime();
      const newestTime = messages[messages.length - 1].timestamp.getTime();
      const daysDiff = (newestTime - oldestTime) / (1000 * 60 * 60 * 24);
      console.log('📜 Message time span:', daysDiff.toFixed(1), 'days');
    }
  }).catch(error => {
    console.error('❌ Failed to load Matrix history:', error);
  });
} else {
  console.log('❌ getRoomHistoryAsGooseMessages not available in console scope');
}

