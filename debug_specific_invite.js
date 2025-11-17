
// Check the specific room and invite state for this message
const roomId = '!DBfjmPwujCwWdASTxV:tchncs.de';
const messageId = 'goose_1763141649757_47hp9jdqf';

console.log('=== DEBUGGING SPECIFIC INVITE ===');
console.log('Room ID:', roomId);
console.log('Message ID:', messageId);

// Check Matrix room membership
const room = matrixService.client?.getRoom(roomId);
console.log('Room exists:', !!room);
console.log('Room name:', room?.name);
console.log('My membership:', room?.getMyMembership());

// Check invite state
const inviteState = matrixInviteStateService.getInviteState(roomId);
console.log('Invite state:', inviteState);
console.log('Should show invite:', matrixInviteStateService.shouldShowInvite(roomId));

// Check session mapping
const sessionMapping = sessionMappingService.getSessionByMatrixRoomId(roomId);
console.log('Session mapping:', sessionMapping);

console.log('=== END DEBUG ===');

