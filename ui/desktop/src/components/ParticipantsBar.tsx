import React from 'react';
import { useMatrix } from '../contexts/MatrixContext';
import AvatarImage from './AvatarImage';
import { Users } from 'lucide-react';

interface ParticipantsBarProps {
  matrixRoomId: string;
  className?: string;
}

export const ParticipantsBar: React.FC<ParticipantsBarProps> = ({ 
  matrixRoomId, 
  className = '' 
}) => {
  const { rooms, currentUser } = useMatrix();
  
  // Find the current room
  const currentRoom = rooms.find(room => room.roomId === matrixRoomId);
  
  if (!currentRoom || !currentRoom.members || currentRoom.members.length <= 1) {
    return null;
  }
  
  // Filter out the current user and get other participants
  const otherParticipants = currentRoom.members.filter(
    member => member.userId !== currentUser?.userId
  );
  
  // For DMs (2 people total), show a simple format
  if (currentRoom.members.length === 2) {
    const otherUser = otherParticipants[0];
    if (!otherUser) return null;
    
    return (
      <div className={`flex items-center gap-2 px-4 py-2 bg-background-default/95 backdrop-blur-sm border-b border-border-subtle sticky top-0 z-50 ${className}`}>
        <AvatarImage
          avatarUrl={otherUser.avatarUrl}
          displayName={otherUser.displayName || otherUser.userId}
          size="sm"
          className="ring-1 ring-border-subtle"
        />
        <div className="flex flex-col min-w-0">
          <span className="text-sm font-medium text-text-prominent truncate">
            {otherUser.displayName || otherUser.userId.split(':')[0].substring(1)}
          </span>
          <span className="text-xs text-text-muted truncate">
            Direct message
          </span>
        </div>
      </div>
    );
  }
  
  // For group chats (3+ people), show participant count and avatars
  return (
    <div className={`flex items-center gap-3 px-4 py-2 bg-background-default/95 backdrop-blur-sm border-b border-border-subtle sticky top-0 z-50 ${className}`}>
      <div className="flex items-center gap-2">
        <Users className="w-4 h-4 text-text-muted" />
        <span className="text-sm font-medium text-text-prominent">
          {currentRoom.name || 'Group Chat'}
        </span>
      </div>
      
      <div className="flex items-center gap-1">
        <span className="text-xs text-text-muted">
          {currentRoom.members.length} participant{currentRoom.members.length !== 1 ? 's' : ''}
        </span>
      </div>
      
      {/* Show first few participant avatars */}
      <div className="flex -space-x-1 ml-auto">
        {otherParticipants.slice(0, 3).map((member) => (
          <AvatarImage
            key={member.userId}
            avatarUrl={member.avatarUrl}
            displayName={member.displayName || member.userId}
            size="sm"
            className="ring-2 ring-background-default"
          />
        ))}
        {otherParticipants.length > 3 && (
          <div className="w-6 h-6 rounded-full bg-background-accent flex items-center justify-center ring-2 ring-background-default">
            <span className="text-xs font-medium text-text-on-accent">
              +{otherParticipants.length - 3}
            </span>
          </div>
        )}
      </div>
    </div>
  );
};

export default ParticipantsBar;
