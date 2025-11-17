
// Debug the session mapping for the current DM room
const roomId = '!dECWvepUhwCyZwBjhL:tchncs.de'; // Replace with your actual room ID

console.log('🔍 DEBUGGING SESSION MAPPING:');
console.log('Room ID:', roomId);

// Check session mapping
const mapping = sessionMappingService.getMapping(roomId);
console.log('Session mapping:', mapping);

if (mapping) {
  console.log('Backend session ID:', mapping.gooseSessionId);
  console.log('Should make backend calls:', sessionMappingService.shouldMakeBackendCalls(roomId));
  console.log('Backend session ID for API:', sessionMappingService.getBackendSessionId(roomId));
}

// Check all mappings
console.log('All mappings:', sessionMappingService.getAllMappings());

