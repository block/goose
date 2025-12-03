import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Hash, 
  Plus, 
  ArrowLeft,
  Users,
  Lock,
  Globe,
  MessageSquare,
  Settings,
  X
} from 'lucide-react';
import { useMatrix } from '../../contexts/MatrixContext';
import { useNavigate } from 'react-router-dom';
import { useTabContext } from '../../contexts/TabContext';
import { matrixService } from '../../services/MatrixService';
import { sessionMappingService } from '../../services/SessionMappingService';

interface SpaceRoomsViewProps {
  spaceId: string;
  spaceName: string;
  onBack: () => void;
}

interface RoomInfo {
  roomId: string;
  name: string;
  topic?: string;
  memberCount: number;
  isPublic: boolean;
  avatarUrl?: string;
  suggested: boolean;
}

const RoomCard: React.FC<{
  room: RoomInfo;
  onOpenRoom: (room: RoomInfo) => void;
}> = ({ room, onOpenRoom }) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={() => onOpenRoom(room)}
      className="
        relative cursor-pointer group
        bg-background-default
        transition-colors duration-200
        hover:bg-background-medium
        aspect-square
        flex flex-col
        rounded-2xl
        overflow-hidden
        p-6
      "
    >
      {/* Room Icon */}
      <div className="flex items-center justify-center w-12 h-12 bg-background-accent rounded-full mb-4">
        <Hash className="w-6 h-6 text-text-on-accent" />
      </div>

      {/* Privacy Badge */}
      <div className={`absolute top-4 right-4 p-1.5 rounded-full ${
        room.isPublic 
          ? 'bg-green-500/90 text-white' 
          : 'bg-orange-500/90 text-white'
      }`}>
        {room.isPublic ? (
          <Globe className="w-3 h-3" />
        ) : (
          <Lock className="w-3 h-3" />
        )}
      </div>

      {/* Room Info */}
      <div className="flex-1 flex flex-col justify-end">
        <h3 className="text-lg font-medium text-text-default truncate mb-1">
          {room.name}
        </h3>
        {room.topic && (
          <p className="text-xs text-text-muted truncate mb-2">
            {room.topic}
          </p>
        )}
        <div className="flex items-center gap-2 text-xs text-text-muted">
          <Users className="w-3 h-3" />
          <span>{room.memberCount} members</span>
        </div>
      </div>
    </motion.div>
  );
};

const EmptyRoomTile: React.FC<{ onCreateRoom: () => void }> = ({ onCreateRoom }) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={onCreateRoom}
      className="
        relative cursor-pointer group
        bg-background-default
        px-6 py-6
        transition-all duration-200
        hover:bg-background-medium
        aspect-square
        flex flex-col items-center justify-center
        rounded-2xl
      "
    >
      {/* Plus icon - hidden by default, shown on hover */}
      <motion.div
        initial={{ opacity: 0, scale: 0.8 }}
        whileHover={{ opacity: 1, scale: 1 }}
        className="opacity-0 group-hover:opacity-100 transition-all duration-200"
      >
        <div className="w-12 h-12 bg-background-accent rounded-full flex items-center justify-center mb-3">
          <Plus className="w-6 h-6 text-text-on-accent" />
        </div>
        <p className="text-sm font-medium text-text-default text-center">
          Create Room
        </p>
      </motion.div>
      
      {/* Subtle hint when not hovering */}
      <motion.div
        className="opacity-100 group-hover:opacity-0 transition-all duration-200 absolute inset-0 flex items-center justify-center"
      >
        <div className="w-8 h-8 rounded-full border-2 border-dashed border-text-muted/30 flex items-center justify-center">
          <div className="w-1 h-1 bg-text-muted/30 rounded-full" />
        </div>
      </motion.div>
    </motion.div>
  );
};

const CreateRoomModal: React.FC<{
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, topic: string, isPublic: boolean) => Promise<void>;
}> = ({ isOpen, onClose, onCreate }) => {
  const [name, setName] = useState('');
  const [topic, setTopic] = useState('');
  const [isPublic, setIsPublic] = useState(false);
  const [isCreating, setIsCreating] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setIsCreating(true);
    try {
      await onCreate(name.trim(), topic.trim(), isPublic);
      onClose();
      setName('');
      setTopic('');
      setIsPublic(false);
    } catch (error) {
      console.error('Failed to create room:', error);
      alert('Failed to create room. Please try again.');
    } finally {
      setIsCreating(false);
    }
  };

  if (!isOpen) return null;

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
      onClick={onClose}
    >
      <motion.div
        initial={{ opacity: 0, scale: 0.95 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: 0.95 }}
        className="bg-background-default rounded-2xl p-6 w-full max-w-md mx-4"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-semibold text-text-default">Create Room</h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
            disabled={isCreating}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Room Name */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-2">
              Room Name *
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="general, announcements, etc."
              className="w-full px-4 py-3 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent"
              disabled={isCreating}
              required
            />
          </div>

          {/* Room Topic */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-2">
              Topic (optional)
            </label>
            <textarea
              value={topic}
              onChange={(e) => setTopic(e.target.value)}
              placeholder="Describe what this room is about..."
              rows={3}
              className="w-full px-4 py-3 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent resize-none"
              disabled={isCreating}
            />
          </div>

          {/* Privacy Setting */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-3">
              Privacy
            </label>
            <div className="space-y-2">
              <label className="flex items-center gap-3 p-3 rounded-lg border border-border-default cursor-pointer hover:bg-background-medium transition-colors">
                <input
                  type="radio"
                  name="privacy"
                  checked={isPublic}
                  onChange={() => setIsPublic(true)}
                  className="w-4 h-4"
                  disabled={isCreating}
                />
                <Globe className="w-5 h-5 text-green-600" />
                <div className="flex-1">
                  <p className="font-medium text-text-default">Public</p>
                  <p className="text-xs text-text-muted">Anyone can discover and join</p>
                </div>
              </label>
              
              <label className="flex items-center gap-3 p-3 rounded-lg border border-border-default cursor-pointer hover:bg-background-medium transition-colors">
                <input
                  type="radio"
                  name="privacy"
                  checked={!isPublic}
                  onChange={() => setIsPublic(false)}
                  className="w-4 h-4"
                  disabled={isCreating}
                />
                <Lock className="w-5 h-5 text-orange-600" />
                <div className="flex-1">
                  <p className="font-medium text-text-default">Private</p>
                  <p className="text-xs text-text-muted">Only invited members can join</p>
                </div>
              </label>
            </div>
          </div>

          {/* Action Buttons */}
          <div className="flex gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              disabled={isCreating}
              className="flex-1 px-4 py-3 rounded-lg border border-border-default text-text-default hover:bg-background-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!name.trim() || isCreating}
              className="flex-1 px-4 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isCreating ? 'Creating...' : 'Create Room'}
            </button>
          </div>
        </form>
      </motion.div>
    </motion.div>
  );
};

const SpaceRoomsView: React.FC<SpaceRoomsViewProps> = ({ spaceId, spaceName, onBack }) => {
  const { currentUser, rooms } = useMatrix();
  const { openMatrixChat } = useTabContext();
  const navigate = useNavigate();
  const [spaceRooms, setSpaceRooms] = useState<RoomInfo[]>([]);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    loadSpaceRooms();
  }, [spaceId]);

  const loadSpaceRooms = () => {
    setIsLoading(true);
    try {
      // Get children of this Space
      const children = matrixService.getSpaceChildren(spaceId);
      
      console.log('📦 Space children:', children);
      
      // Map children to room info - get rooms directly from Matrix client
      const roomInfos: RoomInfo[] = children
        .map(child => {
          // Get the room directly from the Matrix client (bypasses filtering)
          const matrixRoom = matrixService.client?.getRoom(child.roomId);
          
          if (!matrixRoom) {
            console.warn('Room not found in Matrix client:', child.roomId);
            return null;
          }
          
          // Get join rules to determine if public
          const joinRulesEvent = matrixRoom.currentState.getStateEvents('m.room.join_rules', '');
          const isPublic = joinRulesEvent?.getContent()?.join_rule === 'public';
          
          return {
            roomId: child.roomId,
            name: child.name || matrixRoom.name || 'Unnamed Room',
            topic: matrixRoom.currentState.getStateEvents('m.room.topic', '')?.getContent()?.topic,
            memberCount: matrixRoom.getMembers().length,
            isPublic: isPublic,
            avatarUrl: matrixRoom.currentState.getStateEvents('m.room.avatar', '')?.getContent()?.url || null,
            suggested: child.suggested,
          };
        })
        .filter((room): room is RoomInfo => room !== null);
      
      console.log('📦 Loaded', roomInfos.length, 'rooms in Space');
      setSpaceRooms(roomInfos);
    } catch (error) {
      console.error('Failed to load space rooms:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleOpenRoom = async (room: RoomInfo) => {
    console.log('📱 Opening room:', room);
    
    try {
      // Check if session mapping exists
      const existingMapping = sessionMappingService.getMapping(room.roomId);
      
      if (!existingMapping) {
        console.log('📋 No session mapping found, creating backend session with Matrix history for room:', room.roomId);
        
        // Get room members directly from Matrix client
        const matrixRoom = matrixService.client?.getRoom(room.roomId);
        const participants = matrixRoom?.getMembers().map(m => m.userId) || [currentUser?.userId || ''];
        
        // Create backend session mapping
        await sessionMappingService.createMappingWithBackendSession(room.roomId, participants, room.name);
        console.log('✅ Backend session mapping created for existing room');
        
        // Load Matrix history to display in UI
        // Note: The backend session is created fresh, but the UI will load and display
        // all historical messages from Matrix using getRoomHistoryAsGooseMessages()
        // This ensures users see the complete conversation history with proper timestamps
        const roomHistory = await matrixService.getRoomHistoryAsGooseMessages(room.roomId, 100);
        console.log(`📜 Matrix room has ${roomHistory.length} historical messages that will be displayed in UI`);
        console.log('📜 History includes messages from all participants with original timestamps preserved');
      } else {
        console.log('📋 Session mapping already exists for room:', room.roomId);
      }
      
      // Open the room in a chat tab
      openMatrixChat(room.roomId, currentUser?.userId || '', room.name);
      
      // Navigate to the pair view
      navigate('/pair');
    } catch (error) {
      console.error('❌ Failed to open room:', error);
      // Still try to open the chat even if mapping creation fails
      openMatrixChat(room.roomId, currentUser?.userId || '', room.name);
      navigate('/pair');
    }
  };

  const handleCreateRoom = async (name: string, topic: string, isPublic: boolean) => {
    try {
      console.log('📝 Creating room in Space:', { spaceId, name, topic, isPublic });
      
      // Create a regular Matrix room
      const room = await matrixService.client!.createRoom({
        name: name,
        topic: topic,
        preset: isPublic ? 'public_chat' : 'private_chat',
        visibility: isPublic ? 'public' : 'private',
        initial_state: isPublic ? [
          {
            type: 'm.room.join_rules',
            content: {
              join_rule: 'public'
            }
          }
        ] : undefined
      });
      
      const roomId = room.room_id;
      console.log('✅ Room created:', roomId);
      
      // Create session mapping for the new room with backend session
      const participants = [currentUser?.userId || ''];
      await sessionMappingService.createMappingWithBackendSession(roomId, participants, name);
      console.log('✅ Backend session mapping created for room');
      
      // Add the room to this Space
      await matrixService.addRoomToSpace(spaceId, roomId, false);
      console.log('✅ Room added to Space');
      
      // Clear Matrix rooms cache to refresh
      (matrixService as any).cachedRooms = null;
      
      // Reload space rooms
      setTimeout(() => {
        loadSpaceRooms();
      }, 1000);
      
    } catch (error) {
      console.error('❌ Failed to create room in Space:', error);
      throw error;
    }
  };

  return (
    <div className="relative flex flex-col h-screen bg-background-muted">
      {/* Header */}
      <div className="pt-14 pb-4 px-4 mb-0.5 bg-background-default rounded-2xl">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <button
              onClick={onBack}
              className="p-2 rounded-lg hover:bg-background-medium transition-colors"
              title="Back to Channels"
            >
              <ArrowLeft className="w-5 h-5" />
            </button>
            <div className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center">
              <Hash className="w-5 h-5 text-text-on-accent" />
            </div>
            <div>
              <h1 className="text-xl font-semibold text-text-default">{spaceName}</h1>
              <p className="text-sm text-text-muted">
                {spaceRooms.length} {spaceRooms.length === 1 ? 'room' : 'rooms'}
              </p>
            </div>
          </div>
          
          <button
            onClick={() => setShowCreateModal(true)}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors"
          >
            <Plus className="w-4 h-4" />
            <span className="text-sm font-medium">New Room</span>
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        <div className="h-full flex flex-col">
          <div className="flex-1 overflow-y-auto">
            {isLoading ? (
              <div className="text-center py-12">
                <div className="w-8 h-8 border-2 border-background-accent border-t-transparent rounded-full animate-spin mx-auto mb-4" />
                <h3 className="text-lg font-medium text-text-default mb-2">Loading...</h3>
                <p className="text-text-muted">Loading rooms in this space...</p>
              </div>
            ) : spaceRooms.length === 0 ? (
              <div className="text-center py-12">
                <MessageSquare className="w-12 h-12 text-text-muted mx-auto mb-4" />
                <h3 className="text-lg font-medium text-text-default mb-2">No Rooms Yet</h3>
                <p className="text-text-muted mb-6">
                  Create your first room in this space to get started.
                </p>
                <button
                  onClick={() => setShowCreateModal(true)}
                  className="px-6 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors"
                >
                  Create Room
                </button>
              </div>
            ) : (
              <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 gap-0.5">
                {spaceRooms.map((room) => (
                  <RoomCard
                    key={room.roomId}
                    room={room}
                    onOpenRoom={handleOpenRoom}
                  />
                ))}
                {/* Empty tiles for creating new rooms */}
                {Array.from({ length: Math.max(0, 6 - spaceRooms.length) }).map((_, index) => (
                  <EmptyRoomTile
                    key={`empty-${index}`}
                    onCreateRoom={() => setShowCreateModal(true)}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Create Room Modal */}
      <AnimatePresence>
        {showCreateModal && (
          <CreateRoomModal
            isOpen={showCreateModal}
            onClose={() => setShowCreateModal(false)}
            onCreate={handleCreateRoom}
          />
        )}
      </AnimatePresence>
    </div>
  );
};

export default SpaceRoomsView;
