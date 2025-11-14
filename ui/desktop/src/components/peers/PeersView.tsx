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
  WifiOff
} from 'lucide-react';
import { useMatrix } from '../../contexts/MatrixContext';
import { MatrixUser } from '../../services/MatrixService';
import MatrixAuth from './MatrixAuth';

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
          {peer.avatarUrl ? (
            <img src={peer.avatarUrl} alt={peer.displayName} className="w-full h-full object-cover" />
          ) : (
            <span className="text-lg font-medium text-text-on-accent">
              {(peer.displayName || peer.userId).charAt(0).toUpperCase()}
            </span>
          )}
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
                    {user.avatarUrl ? (
                      <img src={user.avatarUrl} alt={user.displayName} className="w-full h-full object-cover" />
                    ) : (
                      <span className="text-sm font-medium text-text-on-accent">
                        {(user.displayName || user.userId).charAt(0).toUpperCase()}
                      </span>
                    )}
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

const PeersView: React.FC<PeersViewProps> = ({ onClose }) => {
  const { 
    isConnected, 
    isReady, 
    currentUser, 
    friends, 
    addFriend, 
    createAISession 
  } = useMatrix();
  
  const [showAddFriendModal, setShowAddFriendModal] = useState(false);
  const [showMatrixAuth, setShowMatrixAuth] = useState(false);
  const [windowSize, setWindowSize] = useState({ width: window.innerWidth, height: window.innerHeight });

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
    const headerHeight = 200; // Approximate height of header + user info sections
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
      const sessionName = `Chat with ${friend.displayName || friend.userId}`;
      const roomId = await createAISession(sessionName, [friend.userId]);
      console.log('Created AI session:', roomId);
      // TODO: Navigate to chat with this room
    } catch (error) {
      console.error('Failed to create AI session:', error);
    }
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

  // Show Matrix authentication modal
  if (showMatrixAuth) {
    return <MatrixAuth onClose={() => setShowMatrixAuth(false)} />;
  }

  return (
    <div className="flex flex-col h-screen bg-background-muted">
      {/* Header */}
      <div className="flex items-center justify-start p-6 border-b border-border-default bg-background-default">
        <div className="flex items-center gap-3">
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={() => setShowAddFriendModal(true)}
            disabled={!isConnected}
            className="p-2 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            title="Add Friend"
          >
            <UserPlus className="w-5 h-5" />
          </motion.button>

          <button
            onClick={() => setShowMatrixAuth(true)}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
            title="Matrix Settings"
          >
            <Settings className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Current User Info */}
      {currentUser && (
        <div className="p-4 bg-background-default border-b border-border-default">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center overflow-hidden">
              {currentUser.avatarUrl ? (
                <img src={currentUser.avatarUrl} alt={currentUser.displayName} className="w-full h-full object-cover" />
              ) : (
                <span className="text-sm font-medium text-text-on-accent">
                  {(currentUser.displayName || currentUser.userId).charAt(0).toUpperCase()}
                </span>
              )}
            </div>
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
            <StatusIndicator status={currentUser.presence} />
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

      {/* Matrix Auth Modal */}
      <AnimatePresence>
        {showMatrixAuth && (
          <MatrixAuth onClose={() => setShowMatrixAuth(false)} />
        )}
      </AnimatePresence>
    </div>
  );
};

export default PeersView;
