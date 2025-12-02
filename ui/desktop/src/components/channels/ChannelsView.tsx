import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Hash, 
  Plus, 
  Search,
  Lock,
  Globe,
  Users,
  Settings,
  X,
  Wifi,
  WifiOff,
  Edit2,
  Star
} from 'lucide-react';
import { useMatrix } from '../../contexts/MatrixContext';
import MatrixAuth from '../peers/MatrixAuth';
import { useNavigate } from 'react-router-dom';
import { useTabContext } from '../../contexts/TabContext';
import { matrixService } from '../../services/MatrixService';

interface Channel {
  roomId: string;
  name: string;
  topic?: string;
  isPublic: boolean;
  memberCount: number;
  avatarUrl?: string;
  coverPhotoUrl?: string;
  lastActivity?: Date;
  unreadCount?: number;
  isFavorite?: boolean;
}

interface ChannelsViewProps {
  onClose?: () => void;
}

const ChannelCard: React.FC<{ 
  channel: Channel; 
  onOpenChannel: (channel: Channel) => void;
  onEditChannel: (channel: Channel) => void;
  onToggleFavorite: (channel: Channel) => void;
}> = ({ channel, onOpenChannel, onEditChannel, onToggleFavorite }) => {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onHoverStart={() => setIsHovered(true)}
      onHoverEnd={() => setIsHovered(false)}
      onClick={() => onOpenChannel(channel)}
      className="
        relative cursor-pointer group
        bg-background-default
        transition-colors duration-200
        hover:bg-background-medium
        aspect-square
        flex flex-col
        rounded-2xl
        overflow-hidden
      "
    >
      {/* Cover Photo Section - Top half */}
      {channel.coverPhotoUrl ? (
        <div className="relative w-full h-[60%] overflow-hidden">
          <img
            src={channel.coverPhotoUrl}
            alt={channel.name}
            className="w-full h-full object-cover"
          />
          {/* Overlay for better text visibility */}
          <div className="absolute inset-0 bg-gradient-to-b from-transparent to-black/20" />
        </div>
      ) : (
        /* Default gradient background if no cover photo - theme aware */
        <div className="relative w-full h-[60%] bg-gradient-to-br from-background-medium via-background-muted to-background-default overflow-hidden">
          {/* Subtle animated background pattern */}
          <div className="absolute inset-0 opacity-30">
            <div className="absolute inset-0 bg-[radial-gradient(circle_at_50%_50%,rgba(var(--color-background-accent-rgb),0.3),transparent_70%)]" />
          </div>
          {/* Hash icon for channels without cover photos */}
          <div className="absolute inset-0 flex items-center justify-center">
            <Hash className="w-12 h-12 text-text-muted/20" />
          </div>
        </div>
      )}

      {/* Favorite button - top left */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onToggleFavorite(channel);
        }}
        className={`absolute top-4 left-4 z-10 p-1.5 rounded-full backdrop-blur-sm transition-all ${
          channel.isFavorite
            ? 'bg-yellow-500/90 text-white'
            : 'bg-black/30 text-white hover:bg-black/50'
        }`}
        title={channel.isFavorite ? 'Remove from favorites' : 'Add to favorites'}
      >
        <Star className={`w-4 h-4 ${channel.isFavorite ? 'fill-current' : ''}`} />
      </button>

      {/* Edit and Privacy buttons - top right */}
      <div className="absolute top-4 right-4 flex items-center gap-1 z-10">
        {/* Edit button - shown on hover */}
        <motion.button
          initial={{ opacity: 0, scale: 0.8 }}
          animate={{ opacity: isHovered ? 1 : 0, scale: isHovered ? 1 : 0.8 }}
          onClick={(e) => {
            e.stopPropagation();
            onEditChannel(channel);
          }}
          className="p-1.5 rounded-full bg-black/30 backdrop-blur-sm text-white hover:bg-black/50 transition-colors"
          title="Edit channel"
        >
          <Edit2 className="w-3 h-3" />
        </motion.button>
        
        {/* Privacy indicator */}
        <div className={`p-1.5 rounded-full backdrop-blur-sm ${
          channel.isPublic 
            ? 'bg-green-500/90 text-white' 
            : 'bg-orange-500/90 text-white'
        }`}>
          {channel.isPublic ? (
            <Globe className="w-3 h-3" />
          ) : (
            <Lock className="w-3 h-3" />
          )}
        </div>
      </div>

      {/* Content Section - Bottom half */}
      <div className="flex-1 px-6 pt-6 pb-6 flex flex-col justify-end">
        <h3 className="text-lg font-light text-text-default truncate mb-1">
          {channel.name}
        </h3>
        {channel.topic && (
          <p className="text-xs text-text-muted truncate mb-2">
            {channel.topic}
          </p>
        )}
        <div className="flex items-center gap-2 text-xs text-text-muted">
          <Users className="w-3 h-3" />
          <span>{channel.memberCount} members</span>
        </div>
      </div>
    </motion.div>
  );
};

const EmptyChannelTile: React.FC<{ onCreateChannel: () => void }> = ({ onCreateChannel }) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={onCreateChannel}
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
          Create Channel
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

const EditChannelModal: React.FC<{
  isOpen: boolean;
  onClose: () => void;
  channel: Channel;
  onEdit: (roomId: string, name: string, topic: string, coverPhotoFile?: File) => Promise<void>;
}> = ({ isOpen, onClose, channel, onEdit }) => {
  const [name, setName] = useState(channel.name);
  const [topic, setTopic] = useState(channel.topic || '');
  const [coverPhotoFile, setCoverPhotoFile] = useState<File | null>(null);
  const [coverPhotoPreview, setCoverPhotoPreview] = useState<string | null>(null);
  const [isEditing, setIsEditing] = useState(false);

  // Update local state when channel prop changes
  useEffect(() => {
    setName(channel.name);
    setTopic(channel.topic || '');
    setCoverPhotoFile(null);
    setCoverPhotoPreview(null);
  }, [channel]);

  const handleCoverPhotoChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      setCoverPhotoFile(file);
      // Create preview URL
      const previewUrl = URL.createObjectURL(file);
      setCoverPhotoPreview(previewUrl);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setIsEditing(true);
    try {
      await onEdit(channel.roomId, name.trim(), topic.trim(), coverPhotoFile || undefined);
      onClose();
    } catch (error) {
      console.error('Failed to edit channel:', error);
      alert('Failed to edit channel. Please try again.');
    } finally {
      setIsEditing(false);
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
          <h2 className="text-xl font-semibold text-text-default">Edit Channel</h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
            disabled={isEditing}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Cover Photo Upload */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-2">
              Cover Photo (optional)
            </label>
            <div className="relative">
              {/* Preview */}
              <div className="w-full h-32 rounded-lg overflow-hidden bg-gradient-to-br from-indigo-500 via-purple-500 to-pink-500 mb-2">
                {coverPhotoPreview ? (
                  <img
                    src={coverPhotoPreview}
                    alt="Cover preview"
                    className="w-full h-full object-cover"
                  />
                ) : channel.coverPhotoUrl ? (
                  <img
                    src={channel.coverPhotoUrl}
                    alt="Current cover"
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <div className="w-full h-full flex items-center justify-center">
                    <span className="text-white text-sm">No cover photo</span>
                  </div>
                )}
              </div>
              {/* File Input */}
              <input
                type="file"
                accept="image/*"
                onChange={handleCoverPhotoChange}
                disabled={isEditing}
                className="block w-full text-sm text-text-muted
                  file:mr-4 file:py-2 file:px-4
                  file:rounded-lg file:border-0
                  file:text-sm file:font-medium
                  file:bg-background-accent file:text-text-on-accent
                  hover:file:bg-background-accent/80
                  file:cursor-pointer cursor-pointer
                  disabled:opacity-50 disabled:cursor-not-allowed"
              />
            </div>
          </div>

          {/* Channel Name */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-2">
              Channel Name *
            </label>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="general, announcements, etc."
              className="w-full px-4 py-3 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent"
              disabled={isEditing}
              required
            />
          </div>

          {/* Channel Topic */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-2">
              Topic (optional)
            </label>
            <textarea
              value={topic}
              onChange={(e) => setTopic(e.target.value)}
              placeholder="Describe what this channel is about..."
              rows={3}
              className="w-full px-4 py-3 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent resize-none"
              disabled={isEditing}
            />
          </div>

          {/* Action Buttons */}
          <div className="flex gap-3 pt-4">
            <button
              type="button"
              onClick={onClose}
              disabled={isEditing}
              className="flex-1 px-4 py-3 rounded-lg border border-border-default text-text-default hover:bg-background-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!name.trim() || isEditing}
              className="flex-1 px-4 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isEditing ? 'Saving...' : 'Save Changes'}
            </button>
          </div>
        </form>
      </motion.div>
    </motion.div>
  );
};

const CreateChannelModal: React.FC<{
  isOpen: boolean;
  onClose: () => void;
  onCreate: (name: string, topic: string, isPublic: boolean) => Promise<void>;
}> = ({ isOpen, onClose, onCreate }) => {
  const [name, setName] = useState('');
  const [topic, setTopic] = useState('');
  const [isPublic, setIsPublic] = useState(true);
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
      setIsPublic(true);
    } catch (error) {
      console.error('Failed to create channel:', error);
      alert('Failed to create channel. Please try again.');
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
          <h2 className="text-xl font-semibold text-text-default">Create Channel</h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
            disabled={isCreating}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Channel Name */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-2">
              Channel Name *
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

          {/* Channel Topic */}
          <div>
            <label className="block text-sm font-medium text-text-default mb-2">
              Topic (optional)
            </label>
            <textarea
              value={topic}
              onChange={(e) => setTopic(e.target.value)}
              placeholder="Describe what this channel is about..."
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
              {isCreating ? 'Creating...' : 'Create Channel'}
            </button>
          </div>
        </form>
      </motion.div>
    </motion.div>
  );
};

const ChannelsView: React.FC<ChannelsViewProps> = ({ onClose }) => {
  const { 
    isConnected, 
    isReady, 
    currentUser,
    rooms,
    setRoomName,
    setRoomTopic,
    setRoomAvatar
  } = useMatrix();
  
  const { openMatrixChat } = useTabContext();
  const navigate = useNavigate();
  
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [editingChannel, setEditingChannel] = useState<Channel | null>(null);
  const [showMatrixAuth, setShowMatrixAuth] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [favorites, setFavorites] = useState<Set<string>>(new Set());

  // Load favorites from localStorage on mount
  useEffect(() => {
    const stored = localStorage.getItem('channelFavorites');
    if (stored) {
      try {
        const favoriteIds = JSON.parse(stored);
        setFavorites(new Set(favoriteIds));
      } catch (error) {
        console.error('Failed to load favorites:', error);
      }
    }
  }, []);

  // Show Matrix auth if not connected
  useEffect(() => {
    if (!isConnected && !showMatrixAuth) {
      setShowMatrixAuth(true);
    }
  }, [isConnected, showMatrixAuth]);

  // Helper function to convert MXC URL to HTTP URL with authentication
  const convertMxcToHttp = (mxcUrl: string | undefined): string | undefined => {
    if (!mxcUrl || !mxcUrl.startsWith('mxc://')) {
      console.log('🖼️ convertMxcToHttp: Not an MXC URL:', mxcUrl);
      return mxcUrl;
    }
    
    // Parse the MXC URL: mxc://server/mediaId
    const mxcMatch = mxcUrl.match(/^mxc:\/\/([^/]+)\/(.+)$/);
    if (!mxcMatch) {
      console.error('🖼️ convertMxcToHttp: Invalid MXC URL format:', mxcUrl);
      return undefined;
    }
    
    const [, serverName, mediaId] = mxcMatch;
    
    const client = (matrixService as any).client;
    if (client) {
      const baseUrl = client.baseUrl || 'https://matrix.tchncs.de';
      const accessToken = client.getAccessToken();
      
      if (accessToken) {
        // Use authenticated client media endpoint with access token in URL
        // Format: /_matrix/client/v1/media/download/{serverName}/{mediaId}?access_token={token}
        const authenticatedUrl = `${baseUrl}/_matrix/client/v1/media/download/${serverName}/${mediaId}?access_token=${accessToken}`;
        console.log('🖼️ convertMxcToHttp: Using authenticated endpoint with token');
        return authenticatedUrl;
      } else {
        console.warn('🖼️ convertMxcToHttp: No access token available, trying unauthenticated');
        // Fall back to unauthenticated endpoint
        return `${baseUrl}/_matrix/media/v3/download/${serverName}/${mediaId}`;
      }
    }
    
    console.log('🖼️ convertMxcToHttp: No client available, returning MXC URL:', mxcUrl);
    return mxcUrl;
  };

  // Filter channels (non-DM rooms) from Matrix rooms and add favorite status
  const channels: Channel[] = rooms
    .filter(room => !room.isDirectMessage)
    .map(room => ({
      roomId: room.roomId,
      name: room.name || 'Unnamed Channel',
      topic: room.topic,
      isPublic: room.isPublic || false,
      memberCount: room.members.length,
      avatarUrl: convertMxcToHttp(room.avatarUrl),
      coverPhotoUrl: convertMxcToHttp(room.avatarUrl), // Use room avatar as cover photo
      lastActivity: room.lastActivity,
      unreadCount: 0, // TODO: Implement unread count
      isFavorite: favorites.has(room.roomId),
    }))
    // Sort: favorites first, then by name
    .sort((a, b) => {
      if (a.isFavorite && !b.isFavorite) return -1;
      if (!a.isFavorite && b.isFavorite) return 1;
      return a.name.localeCompare(b.name);
    });

  // Filter channels based on search query
  const filteredChannels = searchQuery
    ? channels.filter(channel =>
        channel.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        channel.topic?.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : channels;

  const handleOpenChannel = async (channel: Channel) => {
    try {
      console.log('📱 Opening channel:', channel);
      
      // Open a new tab/chat session with Matrix room parameters
      openMatrixChat(channel.roomId, currentUser?.userId || '');
      
      // Navigate to the pair view where the tabs are displayed
      navigate('/pair');
    } catch (error) {
      console.error('Failed to open channel:', error);
    }
  };

  const handleCreateChannel = async (name: string, topic: string, isPublic: boolean) => {
    // TODO: Implement channel creation via Matrix service
    console.log('Creating channel:', { name, topic, isPublic });
    alert('Channel creation not yet implemented');
  };

  const handleEditChannel = (channel: Channel) => {
    setEditingChannel(channel);
    setShowEditModal(true);
  };

  const handleSaveChannelEdit = async (roomId: string, name: string, topic: string, coverPhotoFile?: File) => {
    console.log('🔧 handleSaveChannelEdit called with:', { roomId, name, topic, hasCoverPhoto: !!coverPhotoFile });
    
    try {
      // Update room name if changed
      const currentChannel = channels.find(c => c.roomId === roomId);
      if (currentChannel && name !== currentChannel.name) {
        console.log('📝 Updating room name...');
        await setRoomName(roomId, name);
      }
      
      // Update room topic if changed
      if (currentChannel && topic !== (currentChannel.topic || '')) {
        console.log('📝 Updating room topic...');
        await setRoomTopic(roomId, topic);
      }
      
      // Update cover photo if a new file was selected
      if (coverPhotoFile) {
        console.log('📸 Uploading cover photo for room:', roomId, {
          name: coverPhotoFile.name,
          type: coverPhotoFile.type,
          size: coverPhotoFile.size,
          sizeKB: (coverPhotoFile.size / 1024).toFixed(2) + ' KB'
        });
        
        // Validate file before upload
        if (coverPhotoFile.size > 10 * 1024 * 1024) { // 10MB limit
          throw new Error('Cover photo must be smaller than 10MB');
        }
        
        if (!coverPhotoFile.type.startsWith('image/')) {
          throw new Error('Cover photo must be an image file');
        }
        
        const avatarUrl = await setRoomAvatar(roomId, coverPhotoFile);
        console.log('✅ Cover photo uploaded successfully, MXC URL:', avatarUrl);
        
        // Wait a moment for the server to process the upload
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        // Verify the upload
        const httpUrl = convertMxcToHttp(avatarUrl);
        console.log('🔍 Verifying upload at HTTP URL:', httpUrl);
      }
      
      console.log('✅ Channel updated successfully');
    } catch (error) {
      console.error('❌ Failed to update channel:', error);
      if (error instanceof Error) {
        console.error('Error message:', error.message);
        console.error('Error stack:', error.stack);
      }
      throw error; // Re-throw to let the modal handle the error
    }
  };

  const handleToggleFavorite = (channel: Channel) => {
    const newFavorites = new Set(favorites);
    if (newFavorites.has(channel.roomId)) {
      newFavorites.delete(channel.roomId);
    } else {
      newFavorites.add(channel.roomId);
    }
    setFavorites(newFavorites);
    // Save to localStorage
    localStorage.setItem('channelFavorites', JSON.stringify(Array.from(newFavorites)));
  };

  // Show Matrix authentication modal
  if (showMatrixAuth) {
    return <MatrixAuth onClose={() => setShowMatrixAuth(false)} />;
  }

  return (
    <div className="relative flex flex-col h-screen bg-background-muted">
      {/* Header */}
      <div className="pt-14 pb-4 px-4 mb-0.5 bg-background-default rounded-2xl">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center">
              <Hash className="w-5 h-5 text-text-on-accent" />
            </div>
            <div>
              <h1 className="text-xl font-semibold text-text-default">Channels</h1>
              <p className="text-sm text-text-muted">
                {isConnected ? `${channels.length} channels` : 'Not connected'}
              </p>
            </div>
          </div>
          
          <div className="flex items-center gap-2">
            {/* Connection Status */}
            <div className="flex items-center gap-1 px-3 py-1.5 rounded-lg bg-background-muted">
              {isConnected ? (
                <>
                  <Wifi className="w-3 h-3 text-green-600" />
                  <span className="text-xs text-green-600">Connected</span>
                </>
              ) : (
                <>
                  <WifiOff className="w-3 h-3 text-red-600" />
                  <span className="text-xs text-red-600">Disconnected</span>
                </>
              )}
            </div>
            
            <button
              onClick={() => setShowMatrixAuth(true)}
              className="p-2 rounded-lg hover:bg-background-medium transition-colors"
              title="Matrix Settings"
            >
              <Settings className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Search Bar */}
        {isConnected && isReady && (
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search channels..."
              className="w-full pl-10 pr-4 py-2 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent text-sm"
            />
          </div>
        )}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        <div className="h-full flex flex-col">
          <div className="flex-1 overflow-y-auto">
            {!isConnected ? (
              <div className="text-center py-12">
                <WifiOff className="w-12 h-12 text-text-muted mx-auto mb-4" />
                <h3 className="text-lg font-medium text-text-default mb-2">Not Connected</h3>
                <p className="text-text-muted mb-6">
                  Connect to Matrix to access channels and collaborate with your team.
                </p>
                <button
                  onClick={() => setShowMatrixAuth(true)}
                  className="px-6 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors"
                >
                  Connect to Matrix
                </button>
              </div>
            ) : !isReady ? (
              <div className="text-center py-12">
                <div className="w-8 h-8 border-2 border-background-accent border-t-transparent rounded-full animate-spin mx-auto mb-4" />
                <h3 className="text-lg font-medium text-text-default mb-2">Loading...</h3>
                <p className="text-text-muted">Syncing with Matrix server...</p>
              </div>
            ) : (
              <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-0.5">
                {filteredChannels.map((channel) => (
                  <ChannelCard
                    key={channel.roomId}
                    channel={channel}
                    onOpenChannel={handleOpenChannel}
                    onEditChannel={handleEditChannel}
                    onToggleFavorite={handleToggleFavorite}
                  />
                ))}
                {/* Empty tiles for creating new channels - fill remaining space */}
                {Array.from({ length: 50 - filteredChannels.length }).map((_, index) => (
                  <EmptyChannelTile
                    key={`empty-${index}`}
                    onCreateChannel={() => setShowCreateModal(true)}
                  />
                ))}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Create Channel Modal */}
      <AnimatePresence>
        {showCreateModal && (
          <CreateChannelModal
            isOpen={showCreateModal}
            onClose={() => setShowCreateModal(false)}
            onCreate={handleCreateChannel}
          />
        )}
      </AnimatePresence>

      {/* Edit Channel Modal */}
      <AnimatePresence>
        {showEditModal && editingChannel && (
          <EditChannelModal
            isOpen={showEditModal}
            onClose={() => {
              setShowEditModal(false);
              setEditingChannel(null);
            }}
            channel={editingChannel}
            onEdit={handleSaveChannelEdit}
          />
        )}
      </AnimatePresence>

      {/* Matrix Auth Modal */}
      <AnimatePresence>
        {showMatrixAuth && (
          <MatrixAuth onClose={() => setShowMatrixAuth(false)} />
        )}
      </AnimatePresence>
    </div>
  );
};

export default ChannelsView;
