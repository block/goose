import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Users, 
  UserPlus, 
  Search,
  MoreVertical, 
  Phone, 
  Video,
  UserX,
  Settings,
  X,
  Wifi,
  WifiOff,
  Camera,
  Upload
} from 'lucide-react';
import { useMatrix } from '../../contexts/MatrixContext';
import { MatrixUser, matrixService } from '../../services/MatrixService';
import MatrixAuth from './MatrixAuth';
import GooseChat from '../GooseChat';
import MatrixChat from '../MatrixChat';
import { useLocation } from 'react-router-dom';

// Helper function to handle avatar URLs (now handled by MatrixService)
const convertMxcToHttp = (avatarUrl: string): string => {
  // The MatrixService now handles MXC URL conversion using the Matrix client's built-in method
  // So we just return the URL as-is
  return avatarUrl;
};

// Component that displays avatar with authenticated fetching and fallback to initials
const AvatarImage: React.FC<{
  avatarUrl?: string;
  displayName?: string;
  className?: string;
  onError?: () => void;
}> = ({ avatarUrl, displayName, className, onError }) => {
  const [blobUrl, setBlobUrl] = React.useState<string | null>(null);
  const [showInitials, setShowInitials] = React.useState(false);
  const [isLoading, setIsLoading] = React.useState(false);
  const blobUrlRef = React.useRef<string | null>(null);

  // Fetch authenticated blob when avatarUrl changes
  React.useEffect(() => {
    console.log('AvatarImage useEffect - avatarUrl changed:', avatarUrl);
    
    if (!avatarUrl || !avatarUrl.startsWith('mxc://')) {
      // If it's not an MXC URL, use it directly
      console.log('AvatarImage - using non-MXC URL directly:', avatarUrl);
      setBlobUrl(avatarUrl || null);
      setShowInitials(false);
      return;
    }

    setIsLoading(true);
    setShowInitials(false);
    
    // Clean up previous blob URL
    if (blobUrlRef.current && blobUrlRef.current.startsWith('blob:')) {
      console.log('AvatarImage - cleaning up previous blob URL:', blobUrlRef.current);
      URL.revokeObjectURL(blobUrlRef.current);
      blobUrlRef.current = null;
    }
    setBlobUrl(null);

    console.log('AvatarImage - fetching authenticated blob for:', avatarUrl);

    // Fetch authenticated blob
    matrixService.getAuthenticatedMediaBlob(avatarUrl)
      .then((url) => {
        if (url) {
          console.log('AvatarImage - blob URL created successfully:', url);
          blobUrlRef.current = url;
          setBlobUrl(url);
        } else {
          console.log('AvatarImage - failed to get authenticated blob for:', avatarUrl);
          setShowInitials(true);
          onError?.();
        }
      })
      .catch((error) => {
        console.error('AvatarImage - error getting authenticated blob:', error);
        setShowInitials(true);
        onError?.();
      })
      .finally(() => {
        setIsLoading(false);
      });

    // Cleanup function
    return () => {
      if (blobUrlRef.current && blobUrlRef.current.startsWith('blob:')) {
        console.log('AvatarImage - cleanup: revoking blob URL:', blobUrlRef.current);
        URL.revokeObjectURL(blobUrlRef.current);
        blobUrlRef.current = null;
      }
    };
  }, [avatarUrl, onError]);

  // Cleanup on unmount
  React.useEffect(() => {
    return () => {
      if (blobUrlRef.current && blobUrlRef.current.startsWith('blob:')) {
        console.log('AvatarImage - unmount cleanup: revoking blob URL:', blobUrlRef.current);
        URL.revokeObjectURL(blobUrlRef.current);
        blobUrlRef.current = null;
      }
    };
  }, []);

  const handleImageError = () => {
    console.log('AvatarImage - image failed to load blob URL:', blobUrl, 'for avatar:', avatarUrl);
    setShowInitials(true);
    onError?.();
  };

  const handleImageLoad = () => {
    console.log('AvatarImage - image loaded successfully from blob URL:', blobUrl, 'for avatar:', avatarUrl);
    setShowInitials(false);
  };

  if (!avatarUrl || showInitials || isLoading || !blobUrl) {
    const displayText = isLoading ? '...' : (displayName || 'U').charAt(0).toUpperCase();
    console.log('AvatarImage - showing initials:', displayText, 'state:', { avatarUrl, showInitials, isLoading, blobUrl });
    return (
      <span className="text-lg font-medium text-text-on-accent">
        {displayText}
      </span>
    );
  }

  console.log('AvatarImage - rendering image with blob URL:', blobUrl, 'for avatar:', avatarUrl);
  return (
    <img
      src={blobUrl}
      alt={displayName}
      className={className}
      onLoad={handleImageLoad}
      onError={handleImageError}
    />
  );
};

interface PeersViewProps {
  onClose?: () => void;
}

const StatusIndicator: React.FC<{ status?: string }> = ({ status }) => {
  const statusConfig = {
    online: { color: 'bg-green-500', label: 'Online' },
    unavailable: { color: 'bg-yellow-500', label: 'Away' },
    offline: { color: 'bg-gray-400', label: 'Offline' },
  };

  const config = statusConfig[status as keyof typeof statusConfig] || statusConfig.offline;
  
  return (
    <div className="flex items-center gap-2">
      <div className={`w-2 h-2 rounded-full ${config.color}`} />
      <span className="text-xs text-text-muted">{config.label}</span>
    </div>
  );
};

const PeerCard: React.FC<{ 
  peer: MatrixUser; 
  onStartChat: (peer: MatrixUser) => void;
  onRemovePeer: (peer: MatrixUser) => void;
}> = ({ peer, onStartChat, onRemovePeer }) => {
  const [showActions, setShowActions] = useState(false);

  const formatLastSeen = (lastActiveAgo?: number) => {
    if (!lastActiveAgo) return 'Unknown';
    
    const minutes = Math.floor(lastActiveAgo / (1000 * 60));
    const hours = Math.floor(lastActiveAgo / (1000 * 60 * 60));
    const days = Math.floor(lastActiveAgo / (1000 * 60 * 60 * 24));

    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return `${days}d ago`;
  };

  const isOnline = peer.presence === 'online';

  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={() => onStartChat(peer)}
      className={`
        relative cursor-pointer group
        bg-background-default
        px-6 py-6
        transition-colors duration-200
        hover:bg-background-medium
        aspect-square
        flex flex-col justify-between
        rounded-2xl
        ${isOnline ? 'ring-1 ring-green-200 bg-green-50/30' : ''}
      `}
    >
      {/* Avatar in top left */}
      <div className="relative w-fit">
        <div className="w-12 h-12 bg-background-accent rounded-full flex items-center justify-center overflow-hidden">
          <AvatarImage
            avatarUrl={peer.avatarUrl}
            displayName={peer.displayName}
            className="w-full h-full object-cover"
            onError={() => console.log('Peer avatar fallback to initials for:', peer.userId)}
          />
        </div>
        {/* Status dot */}
        <div className={`absolute -bottom-1 -right-1 w-4 h-4 rounded-full border-2 border-background-default ${
          peer.presence === 'online' ? 'bg-green-500' :
          peer.presence === 'unavailable' ? 'bg-yellow-500' : 'bg-gray-400'
        }`} />
      </div>

      {/* Status tag in top right */}
      <div className="absolute top-4 right-4">
        <div className={`px-2 py-1 rounded-full text-xs font-medium ${
          isOnline 
            ? 'bg-green-100 text-green-700' 
            : 'bg-background-muted text-text-muted'
        }`}>
          {peer.presence === 'online' ? 'Online' :
           peer.presence === 'unavailable' ? 'Away' : 'Offline'}
        </div>
      </div>

      {/* Actions menu in top right corner (on hover) */}
      <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
        <div className="relative">
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={(e) => {
              e.stopPropagation();
              setShowActions(!showActions);
            }}
            className="p-1 rounded-lg hover:bg-background-medium transition-colors"
          >
            <MoreVertical className="w-4 h-4 text-text-muted" />
          </motion.button>

          <AnimatePresence>
            {showActions && (
              <motion.div
                initial={{ opacity: 0, scale: 0.95, y: -10 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.95, y: -10 }}
                className="absolute right-0 top-full mt-2 bg-background-default border border-border-default rounded-lg shadow-lg py-2 min-w-[160px] z-20"
                onClick={(e) => e.stopPropagation()}
              >
                <button
                  onClick={() => {
                    // TODO: Implement voice call
                    setShowActions(false);
                  }}
                  className="w-full px-4 py-2 text-left hover:bg-background-medium flex items-center gap-2 text-sm"
                >
                  <Phone className="w-4 h-4" />
                  Voice Call
                </button>
                <button
                  onClick={() => {
                    // TODO: Implement video call
                    setShowActions(false);
                  }}
                  className="w-full px-4 py-2 text-left hover:bg-background-medium flex items-center gap-2 text-sm"
                >
                  <Video className="w-4 h-4" />
                  Video Call
                </button>
                <div className="border-t border-border-default my-2" />
                <button
                  onClick={() => {
                    onRemovePeer(peer);
                    setShowActions(false);
                  }}
                  className="w-full px-4 py-2 text-left hover:bg-background-medium flex items-center gap-2 text-sm text-red-500"
                >
                  <UserX className="w-4 h-4" />
                  Remove Friend
                </button>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>

      {/* Name and info at bottom */}
      <div className="mt-auto w-full">
        <h3 className="text-lg font-light text-text-default truncate mb-1">
          {peer.displayName || peer.userId.split(':')[0].substring(1)}
        </h3>
        <p className="text-xs text-text-muted truncate">
          {peer.userId}
        </p>
        {peer.presence !== 'online' && peer.lastActiveAgo && (
          <p className="text-xs text-text-muted mt-1">
            {formatLastSeen(peer.lastActiveAgo)}
          </p>
        )}
      </div>
    </motion.div>
  );
};

const EmptyPeerTile: React.FC<{ onAddFriend: () => void }> = ({ onAddFriend }) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      onClick={onAddFriend}
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
          <UserPlus className="w-6 h-6 text-text-on-accent" />
        </div>
        <p className="text-sm font-medium text-text-default text-center">
          Add Friend
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

const AddFriendModal: React.FC<{
  isOpen: boolean;
  onClose: () => void;
  onSendRequest: (userId: string) => void;
}> = ({ isOpen, onClose, onSendRequest }) => {
  const { searchUsers } = useMatrix();
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<MatrixUser[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const handleSearch = async () => {
    if (!searchQuery.trim()) return;

    setIsSearching(true);
    try {
      const results = await searchUsers(searchQuery.trim());
      setSearchResults(results);
    } catch (error) {
      console.error('Search failed:', error);
      setSearchResults([]);
    } finally {
      setIsSearching(false);
    }
  };

  const handleAddFriend = async (userId: string) => {
    setIsLoading(true);
    try {
      await onSendRequest(userId);
      onClose();
    } catch (error) {
      console.error('Failed to add friend:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    handleSearch();
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
        className="bg-background-default rounded-2xl p-6 w-full max-w-md mx-4 max-h-[80vh] overflow-hidden flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-xl font-semibold text-text-default">Add Friend</h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit} className="mb-4">
          <div className="mb-4">
            <label className="block text-sm font-medium text-text-default mb-2">
              Search Users
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder="Enter username or display name"
                className="flex-1 px-4 py-3 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent"
                disabled={isSearching || isLoading}
              />
              <button
                type="submit"
                disabled={!searchQuery.trim() || isSearching || isLoading}
                className="px-4 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {isSearching ? 'Searching...' : 'Search'}
              </button>
            </div>
            <p className="text-xs text-text-muted mt-2">
              Search for users by their display name or Matrix ID (e.g., @user:matrix.org)
            </p>
          </div>
        </form>

        {/* Search Results */}
        <div className="flex-1 overflow-y-auto">
          {searchResults.length > 0 ? (
            <div className="space-y-2">
              <h3 className="text-sm font-medium text-text-muted mb-2">Search Results</h3>
              {searchResults.map((user) => (
                <div
                  key={user.userId}
                  className="flex items-center gap-3 p-3 rounded-lg border border-border-default hover:bg-background-medium transition-colors"
                >
                  <div className="w-10 h-10 bg-background-accent rounded-full flex items-center justify-center overflow-hidden">
                    <AvatarImage
                      avatarUrl={user.avatarUrl}
                      displayName={user.displayName}
                      className="w-full h-full object-cover"
                      onError={() => console.log('Search result avatar fallback to initials for:', user.userId)}
                    />
                  </div>
                  <div className="flex-1 min-w-0">
                    <h4 className="font-medium text-text-default truncate">
                      {user.displayName || user.userId.split(':')[0].substring(1)}
                    </h4>
                    <p className="text-sm text-text-muted truncate">{user.userId}</p>
                  </div>
                  <button
                    onClick={() => handleAddFriend(user.userId)}
                    disabled={isLoading}
                    className="px-3 py-1 rounded-lg bg-green-500 text-white hover:bg-green-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed text-sm"
                  >
                    Add
                  </button>
                </div>
              ))}
            </div>
          ) : searchQuery && !isSearching ? (
            <div className="text-center py-8">
              <Users className="w-8 h-8 text-text-muted mx-auto mb-2" />
              <p className="text-text-muted">No users found for "{searchQuery}"</p>
            </div>
          ) : !searchQuery ? (
            <div className="text-center py-8">
              <Search className="w-8 h-8 text-text-muted mx-auto mb-2" />
              <p className="text-text-muted">Search for users to add as friends</p>
            </div>
          ) : null}
        </div>
      </motion.div>
    </motion.div>
  );
};

const AvatarUploadModal: React.FC<{
  isOpen: boolean;
  onClose: () => void;
  currentAvatar?: string;
  onUpload: (file: File) => Promise<void>;
  onRemove: () => Promise<void>;
}> = ({ isOpen, onClose, currentAvatar, onUpload, onRemove }) => {
  const [isUploading, setIsUploading] = useState(false);
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  const handleFileSelect = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    // Validate file type
    if (!file.type.startsWith('image/')) {
      alert('Please select an image file');
      return;
    }

    // Validate file size (5MB limit)
    if (file.size > 5 * 1024 * 1024) {
      alert('File size must be less than 5MB');
      return;
    }

    setIsUploading(true);
    try {
      await onUpload(file);
      onClose();
    } catch (error) {
      console.error('Failed to upload avatar:', error);
      alert('Failed to upload avatar. Please try again.');
    } finally {
      setIsUploading(false);
    }
  };

  const handleRemove = async () => {
    setIsUploading(true);
    try {
      await onRemove();
      onClose();
    } catch (error) {
      console.error('Failed to remove avatar:', error);
      alert('Failed to remove avatar. Please try again.');
    } finally {
      setIsUploading(false);
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
          <h2 className="text-xl font-semibold text-text-default">Update Avatar</h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
            disabled={isUploading}
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Current Avatar Preview */}
        <div className="flex flex-col items-center mb-6">
          <div className="w-24 h-24 bg-background-accent rounded-full flex items-center justify-center overflow-hidden mb-4">
            {currentAvatar ? (
              <AvatarImage
                avatarUrl={currentAvatar}
                displayName="Current avatar"
                className="w-full h-full object-cover"
                onError={() => console.log('Avatar upload modal - current avatar failed to load')}
              />
            ) : (
              <Camera className="w-8 h-8 text-text-on-accent" />
            )}
          </div>
          <p className="text-sm text-text-muted text-center">
            {currentAvatar ? 'Current avatar' : 'No avatar set'}
          </p>
        </div>

        {/* Action Buttons */}
        <div className="space-y-3">
          <button
            onClick={() => fileInputRef.current?.click()}
            disabled={isUploading}
            className="w-full px-4 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
          >
            {isUploading ? (
              <div className="w-4 h-4 border-2 border-text-on-accent border-t-transparent rounded-full animate-spin" />
            ) : (
              <Upload className="w-4 h-4" />
            )}
            {isUploading ? 'Uploading...' : 'Upload New Avatar'}
          </button>

          {currentAvatar && (
            <button
              onClick={handleRemove}
              disabled={isUploading}
              className="w-full px-4 py-3 rounded-lg border border-red-500 text-red-500 hover:bg-red-50 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Remove Avatar
            </button>
          )}

          <button
            onClick={onClose}
            disabled={isUploading}
            className="w-full px-4 py-3 rounded-lg border border-border-default text-text-default hover:bg-background-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Cancel
          </button>
        </div>

        {/* Hidden File Input */}
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          onChange={handleFileSelect}
          className="hidden"
        />

        <p className="text-xs text-text-muted mt-4 text-center">
          Supported formats: JPG, PNG, GIF. Max size: 5MB
        </p>
      </motion.div>
    </motion.div>
  );
};

const PeersView: React.FC<PeersViewProps> = ({ onClose }) => {
  const { 
    isConnected, 
    isReady, 
    currentUser, 
    friends, 
    addFriend, 
    getOrCreateDirectMessageRoom,
    setAvatar,
    removeAvatar
  } = useMatrix();
  
  const location = useLocation();
  
  const [showAddFriendModal, setShowAddFriendModal] = useState(false);
  const [showMatrixAuth, setShowMatrixAuth] = useState(false);
  const [showAvatarModal, setShowAvatarModal] = useState(false);
  const [windowSize, setWindowSize] = useState({ width: window.innerWidth, height: window.innerHeight });
  
  // Chat state
  const [activeChatRoomId, setActiveChatRoomId] = useState<string | null>(null);
  const [activeChatRecipientId, setActiveChatRecipientId] = useState<string | null>(null);

  // Handle opening chat from notification or route state
  useEffect(() => {
    const routeState = location.state as any;
    if (routeState?.openChat && routeState?.roomId && routeState?.senderId) {
      console.log('📱 Opening chat from notification:', routeState);
      setActiveChatRoomId(routeState.roomId);
      setActiveChatRecipientId(routeState.senderId);
    }
  }, [location.state]);

  // Debug log for avatar modal state
  useEffect(() => {
    console.log('showAvatarModal changed:', showAvatarModal);
  }, [showAvatarModal]);

  // Show Matrix auth if not connected
  useEffect(() => {
    if (!isConnected && !showMatrixAuth) {
      setShowMatrixAuth(true);
    }
  }, [isConnected, showMatrixAuth]);

  // Handle window resize for responsive empty tiles
  useEffect(() => {
    const handleResize = () => {
      setWindowSize({ width: window.innerWidth, height: window.innerHeight });
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Calculate empty tiles to fill the entire viewport
  const calculateEmptyTiles = (friendsCount: number) => {
    // Estimate tiles that can fit in viewport
    // Assuming each tile is roughly 200px (including gaps) and viewport height minus headers
    const estimatedTileHeight = 200;
    const headerHeight = 150; // Approximate height of user info section with 50px top padding
    const availableHeight = windowSize.height - headerHeight;
    
    // Calculate how many rows can fit
    const rowsInViewport = Math.floor(availableHeight / estimatedTileHeight);
    
    // Calculate tiles per row based on current screen width
    const screenWidth = windowSize.width;
    let tilesPerRow = 6; // xl default
    if (screenWidth < 640) tilesPerRow = 2; // sm
    else if (screenWidth < 768) tilesPerRow = 3; // md  
    else if (screenWidth < 1024) tilesPerRow = 4; // lg
    else if (screenWidth < 1280) tilesPerRow = 5; // xl
    
    const totalTilesInViewport = Math.max(rowsInViewport * tilesPerRow, 12); // Minimum 12 tiles (2 rows)
    const emptyTilesNeeded = Math.max(0, totalTilesInViewport - friendsCount);
    
    return emptyTilesNeeded;
  };

  const handleStartChat = async (friend: MatrixUser) => {
    try {
      console.log('📱 Starting chat with friend:', friend);
      const roomId = await getOrCreateDirectMessageRoom(friend.userId);
      console.log('📱 Got/created DM room:', roomId);
      
      setActiveChatRoomId(roomId);
      setActiveChatRecipientId(friend.userId);
    } catch (error) {
      console.error('Failed to create/get DM room:', error);
    }
  };

  const handleCloseChat = () => {
    setActiveChatRoomId(null);
    setActiveChatRecipientId(null);
  };

  const handleRemoveFriend = (friend: MatrixUser) => {
    // TODO: Implement remove friend functionality
    console.log('Remove friend:', friend);
  };

  const handleSendFriendRequest = async (userId: string) => {
    try {
      await addFriend(userId);
    } catch (error) {
      console.error('Failed to send friend request:', error);
      throw error;
    }
  };

  const handleAvatarUpload = async (file: File) => {
    await setAvatar(file);
  };

  const handleAvatarRemove = async () => {
    await removeAvatar();
  };

  // Debug logs
  console.log('PeersView render - isConnected:', isConnected, 'currentUser:', currentUser, 'showMatrixAuth:', showMatrixAuth);
  console.log('PeersView render - friends:', friends);
  
  // Debug avatar URLs
  if (currentUser?.avatarUrl) {
    console.log('Current user avatar URL:', currentUser.avatarUrl);
    console.log('Converted avatar URL:', convertMxcToHttp(currentUser.avatarUrl));
  }
  
  // Debug friend avatars
  friends.forEach(friend => {
    if (friend.avatarUrl) {
      console.log(`Friend ${friend.displayName || friend.userId} avatar URL:`, friend.avatarUrl);
    }
  });

  // Show Matrix authentication modal
  if (showMatrixAuth) {
    return <MatrixAuth onClose={() => setShowMatrixAuth(false)} />;
  }

  return (
    <div className="flex flex-col h-screen bg-background-muted">


      {/* Current User Info */}
      {currentUser && (
        <div className="pt-14 pb-4 px-4 mb-0.5 bg-background-default rounded-2xl">
          <div className="flex items-end gap-3">
            <button
              onClick={() => {
                console.log('Avatar clicked!');
                setShowAvatarModal(true);
              }}
              className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center overflow-hidden hover:ring-2 hover:ring-background-accent hover:ring-offset-2 transition-all duration-200 cursor-pointer"
              title="Change Avatar"
            >
              <AvatarImage
                avatarUrl={currentUser.avatarUrl}
                displayName={currentUser.displayName}
                className="w-full h-full object-cover"
              />
            </button>
            <div className="flex-1">
              <p className="font-medium text-text-default">
                {currentUser.displayName || currentUser.userId.split(':')[0].substring(1)}
              </p>
              <p className="text-xs text-text-muted">{currentUser.userId}</p>
              {/* Connection Status */}
              <div className="flex items-center gap-1 mt-1">
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
            </div>
            <div className="flex items-center gap-3">
              <StatusIndicator status={currentUser.presence} />
              <button
                onClick={() => setShowMatrixAuth(true)}
                className="p-2 rounded-lg hover:bg-background-medium transition-colors"
                title="Matrix Settings"
              >
                <Settings className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        <div className="h-full flex flex-col">
          {/* Friends List */}
          <div className="flex-1 overflow-y-auto">
            {!isConnected ? (
              <div className="text-center py-12">
                <WifiOff className="w-12 h-12 text-text-muted mx-auto mb-4" />
                <h3 className="text-lg font-medium text-text-default mb-2">Not Connected</h3>
                <p className="text-text-muted mb-6">
                  Connect to Matrix to see your friends and start collaborative AI sessions.
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
              <div className="space-y-6">
                <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-0.5">
                  {friends.map((friend) => (
                    <PeerCard
                      key={friend.userId}
                      peer={friend}
                      onStartChat={handleStartChat}
                      onRemovePeer={handleRemoveFriend}
                    />
                  ))}
                  {/* Empty tiles */}
                  {Array.from({ length: calculateEmptyTiles(friends.length) }).map((_, index) => (
                    <EmptyPeerTile
                      key={`empty-${index}`}
                      onAddFriend={() => setShowAddFriendModal(true)}
                    />
                  ))}
                </div>
                
                {/* Goose-to-Goose Communication Section */}
                <div className="px-4">
                  <GooseChat />
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Add Friend Modal */}
      <AnimatePresence>
        {showAddFriendModal && (
          <AddFriendModal
            isOpen={showAddFriendModal}
            onClose={() => setShowAddFriendModal(false)}
            onSendRequest={handleSendFriendRequest}
          />
        )}
      </AnimatePresence>

      {/* Avatar Upload Modal */}
      <AnimatePresence>
        {showAvatarModal && (
          <AvatarUploadModal
            isOpen={showAvatarModal}
            onClose={() => setShowAvatarModal(false)}
            currentAvatar={currentUser?.avatarUrl}
            onUpload={handleAvatarUpload}
            onRemove={handleAvatarRemove}
          />
        )}
      </AnimatePresence>

      {/* Matrix Auth Modal */}
      <AnimatePresence>
        {showMatrixAuth && (
          <MatrixAuth onClose={() => setShowMatrixAuth(false)} />
        )}
      </AnimatePresence>

      {/* Matrix Chat Overlay */}
      <AnimatePresence>
        {activeChatRoomId && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
          >
            <motion.div
              initial={{ opacity: 0, scale: 0.95, y: 20 }}
              animate={{ opacity: 1, scale: 1, y: 0 }}
              exit={{ opacity: 0, scale: 0.95, y: 20 }}
              className="bg-background-default rounded-2xl w-full max-w-4xl mx-4 h-[80vh] overflow-hidden shadow-2xl"
            >
              <MatrixChat
                roomId={activeChatRoomId}
                recipientId={activeChatRecipientId || undefined}
                onBack={handleCloseChat}
                className="h-full"
              />
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export default PeersView;
