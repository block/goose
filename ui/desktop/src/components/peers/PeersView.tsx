import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  Users, 
  UserPlus, 
  Search, 
  MoreVertical, 
  MessageCircle, 
  Phone, 
  Video,
  UserCheck,
  UserX,
  Settings,
  X,
  Circle,
  CheckCircle,
  Clock,
  AlertCircle
} from 'lucide-react';

// Types for our peer system
interface Peer {
  id: string;
  username: string;
  displayName: string;
  avatar?: string;
  status: 'online' | 'away' | 'busy' | 'offline';
  lastSeen?: Date;
  isBlocked?: boolean;
  publicKey?: string; // For Signal integration later
}

interface PendingRequest {
  id: string;
  username: string;
  displayName: string;
  avatar?: string;
  type: 'incoming' | 'outgoing';
  timestamp: Date;
}

interface PeersViewProps {
  onClose: () => void;
}

// Mock data for development
const mockPeers: Peer[] = [
  {
    id: '1',
    username: 'alice_dev',
    displayName: 'Alice Johnson',
    status: 'online',
    lastSeen: new Date(),
  },
  {
    id: '2',
    username: 'bob_designer',
    displayName: 'Bob Smith',
    status: 'away',
    lastSeen: new Date(Date.now() - 1000 * 60 * 15), // 15 minutes ago
  },
  {
    id: '3',
    username: 'charlie_pm',
    displayName: 'Charlie Brown',
    status: 'busy',
    lastSeen: new Date(Date.now() - 1000 * 60 * 30), // 30 minutes ago
  },
  {
    id: '4',
    username: 'diana_qa',
    displayName: 'Diana Prince',
    status: 'offline',
    lastSeen: new Date(Date.now() - 1000 * 60 * 60 * 2), // 2 hours ago
  },
];

const mockPendingRequests: PendingRequest[] = [
  {
    id: '1',
    username: 'eve_new',
    displayName: 'Eve Wilson',
    type: 'incoming',
    timestamp: new Date(Date.now() - 1000 * 60 * 10), // 10 minutes ago
  },
  {
    id: '2',
    username: 'frank_dev',
    displayName: 'Frank Miller',
    type: 'outgoing',
    timestamp: new Date(Date.now() - 1000 * 60 * 60), // 1 hour ago
  },
];

const StatusIndicator: React.FC<{ status: Peer['status'] }> = ({ status }) => {
  const statusConfig = {
    online: { color: 'bg-green-500', label: 'Online' },
    away: { color: 'bg-yellow-500', label: 'Away' },
    busy: { color: 'bg-red-500', label: 'Busy' },
    offline: { color: 'bg-gray-400', label: 'Offline' },
  };

  const config = statusConfig[status];
  
  return (
    <div className="flex items-center gap-2">
      <div className={`w-2 h-2 rounded-full ${config.color}`} />
      <span className="text-xs text-text-muted">{config.label}</span>
    </div>
  );
};

const PeerCard: React.FC<{ 
  peer: Peer; 
  onStartChat: (peer: Peer) => void;
  onRemovePeer: (peer: Peer) => void;
}> = ({ peer, onStartChat, onRemovePeer }) => {
  const [showActions, setShowActions] = useState(false);

  const formatLastSeen = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(diff / (1000 * 60 * 60 * 24));

    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    return `${days}d ago`;
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="bg-background-default rounded-xl p-4 hover:bg-background-medium transition-colors relative group"
    >
      {/* Avatar and Info */}
      <div className="flex items-start gap-3">
        <div className="relative">
          <div className="w-12 h-12 bg-background-accent rounded-full flex items-center justify-center">
            <span className="text-lg font-medium text-text-on-accent">
              {peer.displayName.charAt(0).toUpperCase()}
            </span>
          </div>
          {/* Status dot */}
          <div className={`absolute -bottom-1 -right-1 w-4 h-4 rounded-full border-2 border-background-default ${
            peer.status === 'online' ? 'bg-green-500' :
            peer.status === 'away' ? 'bg-yellow-500' :
            peer.status === 'busy' ? 'bg-red-500' : 'bg-gray-400'
          }`} />
        </div>

        <div className="flex-1 min-w-0">
          <h3 className="font-medium text-text-default truncate">{peer.displayName}</h3>
          <p className="text-sm text-text-muted truncate">@{peer.username}</p>
          <div className="mt-1">
            <StatusIndicator status={peer.status} />
          </div>
          {peer.status !== 'online' && peer.lastSeen && (
            <p className="text-xs text-text-muted mt-1">
              Last seen {formatLastSeen(peer.lastSeen)}
            </p>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center gap-2">
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={() => onStartChat(peer)}
            className="p-2 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors"
            title="Start AI chat session"
          >
            <MessageCircle className="w-4 h-4" />
          </motion.button>

          <div className="relative">
            <motion.button
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              onClick={() => setShowActions(!showActions)}
              className="p-2 rounded-lg hover:bg-background-medium transition-colors opacity-0 group-hover:opacity-100"
            >
              <MoreVertical className="w-4 h-4" />
            </motion.button>

            <AnimatePresence>
              {showActions && (
                <motion.div
                  initial={{ opacity: 0, scale: 0.95, y: -10 }}
                  animate={{ opacity: 1, scale: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.95, y: -10 }}
                  className="absolute right-0 top-full mt-2 bg-background-default border border-border-default rounded-lg shadow-lg py-2 min-w-[160px] z-10"
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
      </div>
    </motion.div>
  );
};

const PendingRequestCard: React.FC<{ 
  request: PendingRequest;
  onAccept: (request: PendingRequest) => void;
  onDecline: (request: PendingRequest) => void;
  onCancel: (request: PendingRequest) => void;
}> = ({ request, onAccept, onDecline, onCancel }) => {
  const formatTimestamp = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / (1000 * 60));
    const hours = Math.floor(diff / (1000 * 60 * 60));

    if (minutes < 1) return 'Just now';
    if (minutes < 60) return `${minutes}m ago`;
    return `${hours}h ago`;
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="bg-background-default rounded-xl p-4 border border-border-default"
    >
      <div className="flex items-center gap-3">
        <div className="w-10 h-10 bg-background-accent rounded-full flex items-center justify-center">
          <span className="text-sm font-medium text-text-on-accent">
            {request.displayName.charAt(0).toUpperCase()}
          </span>
        </div>

        <div className="flex-1 min-w-0">
          <h4 className="font-medium text-text-default truncate">{request.displayName}</h4>
          <p className="text-sm text-text-muted truncate">@{request.username}</p>
          <p className="text-xs text-text-muted mt-1">
            {request.type === 'incoming' ? 'Wants to be friends' : 'Friend request sent'} • {formatTimestamp(request.timestamp)}
          </p>
        </div>

        <div className="flex items-center gap-2">
          {request.type === 'incoming' ? (
            <>
              <motion.button
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                onClick={() => onAccept(request)}
                className="p-2 rounded-lg bg-green-500 text-white hover:bg-green-600 transition-colors"
                title="Accept request"
              >
                <UserCheck className="w-4 h-4" />
              </motion.button>
              <motion.button
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
                onClick={() => onDecline(request)}
                className="p-2 rounded-lg bg-red-500 text-white hover:bg-red-600 transition-colors"
                title="Decline request"
              >
                <UserX className="w-4 h-4" />
              </motion.button>
            </>
          ) : (
            <motion.button
              whileHover={{ scale: 1.05 }}
              whileTap={{ scale: 0.95 }}
              onClick={() => onCancel(request)}
              className="p-2 rounded-lg hover:bg-background-medium transition-colors"
              title="Cancel request"
            >
              <X className="w-4 h-4" />
            </motion.button>
          )}
        </div>
      </div>
    </motion.div>
  );
};

const AddFriendModal: React.FC<{
  isOpen: boolean;
  onClose: () => void;
  onSendRequest: (username: string) => void;
}> = ({ isOpen, onClose, onSendRequest }) => {
  const [username, setUsername] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim()) return;

    setIsLoading(true);
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000));
    onSendRequest(username.trim());
    setUsername('');
    setIsLoading(false);
    onClose();
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
          <h2 className="text-xl font-semibold text-text-default">Add Friend</h2>
          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="mb-6">
            <label className="block text-sm font-medium text-text-default mb-2">
              Username
            </label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Enter username (e.g., alice_dev)"
              className="w-full px-4 py-3 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent"
              disabled={isLoading}
            />
            <p className="text-xs text-text-muted mt-2">
              Enter the exact username of the person you want to add as a friend.
            </p>
          </div>

          <div className="flex gap-3">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 py-3 rounded-lg border border-border-default hover:bg-background-medium transition-colors"
              disabled={isLoading}
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={!username.trim() || isLoading}
              className="flex-1 px-4 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isLoading ? 'Sending...' : 'Send Request'}
            </button>
          </div>
        </form>
      </motion.div>
    </motion.div>
  );
};

const PeersView: React.FC<PeersViewProps> = ({ onClose }) => {
  const [peers, setPeers] = useState<Peer[]>(mockPeers);
  const [pendingRequests, setPendingRequests] = useState<PendingRequest[]>(mockPendingRequests);
  const [searchQuery, setSearchQuery] = useState('');
  const [showAddFriendModal, setShowAddFriendModal] = useState(false);
  const [activeTab, setActiveTab] = useState<'friends' | 'requests'>('friends');

  // Filter peers based on search query
  const filteredPeers = peers.filter(peer =>
    peer.displayName.toLowerCase().includes(searchQuery.toLowerCase()) ||
    peer.username.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Group peers by status
  const onlinePeers = filteredPeers.filter(peer => peer.status === 'online');
  const offlinePeers = filteredPeers.filter(peer => peer.status !== 'online');

  const handleStartChat = (peer: Peer) => {
    // TODO: Implement P2P chat session creation
    console.log('Starting chat with:', peer);
    // This will eventually create a shared AI chat session
  };

  const handleRemovePeer = (peer: Peer) => {
    setPeers(prev => prev.filter(p => p.id !== peer.id));
  };

  const handleAcceptRequest = (request: PendingRequest) => {
    // Add to friends list
    const newPeer: Peer = {
      id: Date.now().toString(),
      username: request.username,
      displayName: request.displayName,
      status: 'online', // Assume they're online when accepted
      lastSeen: new Date(),
    };
    setPeers(prev => [...prev, newPeer]);
    
    // Remove from pending requests
    setPendingRequests(prev => prev.filter(r => r.id !== request.id));
  };

  const handleDeclineRequest = (request: PendingRequest) => {
    setPendingRequests(prev => prev.filter(r => r.id !== request.id));
  };

  const handleCancelRequest = (request: PendingRequest) => {
    setPendingRequests(prev => prev.filter(r => r.id !== request.id));
  };

  const handleSendFriendRequest = (username: string) => {
    const newRequest: PendingRequest = {
      id: Date.now().toString(),
      username,
      displayName: username.replace('_', ' ').replace(/\b\w/g, l => l.toUpperCase()),
      type: 'outgoing',
      timestamp: new Date(),
    };
    setPendingRequests(prev => [...prev, newRequest]);
  };

  const incomingRequests = pendingRequests.filter(r => r.type === 'incoming');
  const outgoingRequests = pendingRequests.filter(r => r.type === 'outgoing');

  return (
    <div className="flex flex-col h-screen bg-background-muted">
      {/* Header */}
      <div className="flex items-center justify-between p-6 border-b border-border-default bg-background-default">
        <div className="flex items-center gap-3">
          <Users className="w-6 h-6 text-text-default" />
          <h1 className="text-2xl font-semibold text-text-default">Friends</h1>
          <span className="px-2 py-1 rounded-full bg-background-accent text-text-on-accent text-sm font-medium">
            {peers.length}
          </span>
        </div>

        <div className="flex items-center gap-3">
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={() => setShowAddFriendModal(true)}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors"
          >
            <UserPlus className="w-4 h-4" />
            Add Friend
          </motion.button>

          <button
            onClick={onClose}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex border-b border-border-default bg-background-default">
        <button
          onClick={() => setActiveTab('friends')}
          className={`flex-1 px-6 py-3 text-sm font-medium transition-colors relative ${
            activeTab === 'friends'
              ? 'text-text-default border-b-2 border-background-accent'
              : 'text-text-muted hover:text-text-default'
          }`}
        >
          Friends ({peers.length})
        </button>
        <button
          onClick={() => setActiveTab('requests')}
          className={`flex-1 px-6 py-3 text-sm font-medium transition-colors relative ${
            activeTab === 'requests'
              ? 'text-text-default border-b-2 border-background-accent'
              : 'text-text-muted hover:text-text-default'
          }`}
        >
          Requests ({pendingRequests.length})
          {pendingRequests.length > 0 && (
            <span className="absolute -top-1 -right-1 w-2 h-2 bg-red-500 rounded-full" />
          )}
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-hidden">
        {activeTab === 'friends' ? (
          <div className="h-full flex flex-col">
            {/* Search */}
            <div className="p-6 bg-background-default border-b border-border-default">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-text-muted" />
                <input
                  type="text"
                  placeholder="Search friends..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="w-full pl-10 pr-4 py-3 rounded-lg border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-background-accent"
                />
              </div>
            </div>

            {/* Friends List */}
            <div className="flex-1 overflow-y-auto p-6 space-y-6">
              {filteredPeers.length === 0 ? (
                <div className="text-center py-12">
                  <Users className="w-12 h-12 text-text-muted mx-auto mb-4" />
                  <h3 className="text-lg font-medium text-text-default mb-2">
                    {searchQuery ? 'No friends found' : 'No friends yet'}
                  </h3>
                  <p className="text-text-muted mb-6">
                    {searchQuery 
                      ? 'Try adjusting your search terms'
                      : 'Add friends to start collaborative AI chat sessions'
                    }
                  </p>
                  {!searchQuery && (
                    <button
                      onClick={() => setShowAddFriendModal(true)}
                      className="px-6 py-3 rounded-lg bg-background-accent text-text-on-accent hover:bg-background-accent/80 transition-colors"
                    >
                      Add Your First Friend
                    </button>
                  )}
                </div>
              ) : (
                <>
                  {/* Online Friends */}
                  {onlinePeers.length > 0 && (
                    <div>
                      <h2 className="text-sm font-medium text-text-muted mb-3 flex items-center gap-2">
                        <Circle className="w-2 h-2 fill-green-500 text-green-500" />
                        Online ({onlinePeers.length})
                      </h2>
                      <div className="space-y-3">
                        {onlinePeers.map((peer) => (
                          <PeerCard
                            key={peer.id}
                            peer={peer}
                            onStartChat={handleStartChat}
                            onRemovePeer={handleRemovePeer}
                          />
                        ))}
                      </div>
                    </div>
                  )}

                  {/* Offline Friends */}
                  {offlinePeers.length > 0 && (
                    <div>
                      <h2 className="text-sm font-medium text-text-muted mb-3 flex items-center gap-2">
                        <Circle className="w-2 h-2 fill-gray-400 text-gray-400" />
                        Offline ({offlinePeers.length})
                      </h2>
                      <div className="space-y-3">
                        {offlinePeers.map((peer) => (
                          <PeerCard
                            key={peer.id}
                            peer={peer}
                            onStartChat={handleStartChat}
                            onRemovePeer={handleRemovePeer}
                          />
                        ))}
                      </div>
                    </div>
                  )}
                </>
              )}
            </div>
          </div>
        ) : (
          /* Requests Tab */
          <div className="h-full overflow-y-auto p-6 space-y-6">
            {pendingRequests.length === 0 ? (
              <div className="text-center py-12">
                <AlertCircle className="w-12 h-12 text-text-muted mx-auto mb-4" />
                <h3 className="text-lg font-medium text-text-default mb-2">No pending requests</h3>
                <p className="text-text-muted">
                  Friend requests will appear here when you receive them or send them.
                </p>
              </div>
            ) : (
              <>
                {/* Incoming Requests */}
                {incomingRequests.length > 0 && (
                  <div>
                    <h2 className="text-sm font-medium text-text-muted mb-3 flex items-center gap-2">
                      <CheckCircle className="w-4 h-4" />
                      Incoming Requests ({incomingRequests.length})
                    </h2>
                    <div className="space-y-3">
                      {incomingRequests.map((request) => (
                        <PendingRequestCard
                          key={request.id}
                          request={request}
                          onAccept={handleAcceptRequest}
                          onDecline={handleDeclineRequest}
                          onCancel={handleCancelRequest}
                        />
                      ))}
                    </div>
                  </div>
                )}

                {/* Outgoing Requests */}
                {outgoingRequests.length > 0 && (
                  <div>
                    <h2 className="text-sm font-medium text-text-muted mb-3 flex items-center gap-2">
                      <Clock className="w-4 h-4" />
                      Sent Requests ({outgoingRequests.length})
                    </h2>
                    <div className="space-y-3">
                      {outgoingRequests.map((request) => (
                        <PendingRequestCard
                          key={request.id}
                          request={request}
                          onAccept={handleAcceptRequest}
                          onDecline={handleDeclineRequest}
                          onCancel={handleCancelRequest}
                        />
                      ))}
                    </div>
                  </div>
                )}
              </>
            )}
          </div>
        )}
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
    </div>
  );
};

export default PeersView;
