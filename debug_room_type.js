
// Debug the Matrix room details and session mapping logic
const roomId = '!dECWvepUhwCyZwBjhL:tchncs.de';
const room = matrixService.client?.getRoom(roomId);

console.log('🔍 MATRIX ROOM ANALYSIS:');
console.log('Room ID:', roomId);
console.log('Room exists:', !!room);
console.log('Room name:', room?.name);
console.log('Room members count:', room?.getJoinedMemberCount());
console.log('Room members:', room?.getJoinedMembers()?.map(m => m.userId));
console.log('Room type:', room?.getType());
console.log('Is direct message:', room?.getDMInviter() ? true : false);

// Check session mapping
console.log('
🔍 SESSION MAPPING:');
const mapping = sessionMappingService.getSessionByMatrixRoomId(roomId);
console.log('Existing mapping:', mapping);

// Check if this should be a collaborative session
console.log('
🔍 COLLABORATION CHECK:');
console.log('Room state:', sessionMappingService.getMatrixRoomState(roomId));
console.log('Is collaborative:', sessionMappingService.isMatrixCollaborativeSession(roomId));

