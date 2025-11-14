import React, { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useMatrix } from '../contexts/MatrixContext';
import { GooseChatMessage } from '../services/MatrixService';

const GooseChat: React.FC = () => {
  const navigate = useNavigate();
  const { 
    isConnected, 
    gooseInstances, 
    sendGooseMessage, 
    sendTaskRequest,
    sendCollaborationInvite,
    acceptCollaborationInvite,
    createGooseCollaborationRoom,
    announceCapabilities,
    getOrCreateDirectMessageRoom,
    onGooseMessage,
    debugGooseMessage,
    getDebugInfo
  } = useMatrix();
  
  const [messages, setMessages] = useState<GooseChatMessage[]>([]);
  const [selectedRoom, setSelectedRoom] = useState<string>('');
  const [messageText, setMessageText] = useState('');
  const [taskDescription, setTaskDescription] = useState('');
  const [collaborationProject, setCollaborationProject] = useState('');

  // Listen for incoming Goose messages
  useEffect(() => {
    const unsubscribe = onGooseMessage((message: GooseChatMessage) => {
      console.log('🦆 Received Goose message:', message);
      setMessages(prev => [...prev, message]);
    });

    return unsubscribe;
  }, [onGooseMessage]);

  const handleSendMessage = async () => {
    if (!messageText.trim() || !selectedRoom) return;

    try {
      await sendGooseMessage(selectedRoom, messageText);
      setMessageText('');
    } catch (error) {
      console.error('Failed to send Goose message:', error);
    }
  };

  const handleSendTaskRequest = async () => {
    if (!taskDescription.trim() || !selectedRoom) return;

    try {
      await sendTaskRequest(selectedRoom, taskDescription, 'general', {
        priority: 'medium',
        requiredCapabilities: ['coding', 'analysis'],
      });
      setTaskDescription('');
    } catch (error) {
      console.error('Failed to send task request:', error);
    }
  };

  const handleSendCollaborationInvite = async () => {
    if (!collaborationProject.trim() || !selectedRoom) return;

    try {
      await sendCollaborationInvite(selectedRoom, collaborationProject, ['coding', 'research']);
      setCollaborationProject('');
    } catch (error) {
      console.error('Failed to send collaboration invite:', error);
    }
  };

  const handleCreateCollaborationRoom = async () => {
    try {
      const gooseIds = gooseInstances.map(g => g.userId);
      const roomId = await createGooseCollaborationRoom('Multi-Goose Collaboration', gooseIds);
      setSelectedRoom(roomId);
      
      // Announce our capabilities
      await announceCapabilities(roomId, ['coding', 'analysis', 'research', 'debugging'], 'idle');
    } catch (error) {
      console.error('Failed to create collaboration room:', error);
    }
  };

  const handleAcceptCollaboration = async (message: GooseChatMessage) => {
    try {
      console.log('🤝 Accepting collaboration invite:', message);
      
      // Accept the collaboration invite via Matrix
      await acceptCollaborationInvite(message.roomId, message.messageId, ['ai-chat', 'collaboration']);
      
      // Navigate to the main chat interface with the collaboration room
      console.log('✅ Accepted collaboration invite, opening chat session for room:', message.roomId);
      
      // Navigate to the root route (/) which will render the MatrixChat in the main chat interface
      navigate('/', { 
        state: { 
          matrixMode: true,
          matrixRoomId: message.roomId,
          matrixRecipientId: message.sender,
          resetChat: true,
          collaborationMode: true
        } 
      });
      
    } catch (error) {
      console.error('❌ Failed to accept collaboration:', error);
      alert('Failed to join collaboration session. Please try again.');
    }
  };

  const formatMessageType = (type: string) => {
    switch (type) {
      case 'goose.chat': return '💬 Chat';
      case 'goose.task.request': return '📋 Task Request';
      case 'goose.task.response': return '✅ Task Response';
      case 'goose.collaboration.invite': return '🤝 Collaboration Invite';
      case 'goose.collaboration.chat': return '💬 Chat Collaboration';
      case 'goose.collaboration.accept': return '✅ Collaboration Accepted';
      case 'goose.collaboration.decline': return '❌ Collaboration Declined';
      default: return '🦆 Goose Message';
    }
  };

  const getSenderDisplayName = (message: GooseChatMessage) => {
    const isFromSelf = message.metadata?.isFromSelf;
    if (isFromSelf) {
      return 'You';
    }
    
    // Try to find display name from goose instances
    const gooseInstance = gooseInstances.find(g => g.userId === message.sender);
    if (gooseInstance?.displayName) {
      return gooseInstance.displayName;
    }
    
    // Fallback to user ID without domain
    return message.sender.split(':')[0].substring(1);
  };

  if (!isConnected) {
    return (
      <div className="p-6 bg-background-muted rounded-lg">
        <h2 className="text-xl font-semibold mb-4">🦆 Goose-to-Goose Communication</h2>
        <p className="text-text-muted">Connect to Matrix to enable Goose-to-Goose communication.</p>
      </div>
    );
  }

  return (
    <div className="p-6 bg-background-muted rounded-lg">
      <h2 className="text-xl font-semibold mb-4">🦆 Goose-to-Goose Communication</h2>
      
      {/* Goose Instances */}
      <div className="mb-6">
        <h3 className="text-lg font-medium mb-2">Connected Goose Instances ({gooseInstances.length})</h3>
        {gooseInstances.length === 0 ? (
          <p className="text-text-muted text-sm">No other Goose instances found. Add friends with "goose" in their name to see them here.</p>
        ) : (
          <div className="space-y-2">
            {gooseInstances.map(goose => (
              <div key={goose.userId} className="flex items-center gap-3 p-2 bg-background-default rounded">
                <div className={`w-3 h-3 rounded-full ${
                  goose.presence === 'online' ? 'bg-green-500' : 
                  goose.presence === 'unavailable' ? 'bg-yellow-500' : 'bg-gray-400'
                }`} />
                <span className="font-medium">{goose.displayName || goose.userId}</span>
                <span className="text-sm text-text-muted">{goose.status || 'idle'}</span>
                <button
                  onClick={async () => {
                    try {
                      const roomId = await getOrCreateDirectMessageRoom(goose.userId);
                      setSelectedRoom(roomId);
                      console.log('🦆 Set room for DM with', goose.displayName || goose.userId, ':', roomId);
                    } catch (error) {
                      console.error('Failed to get/create DM room:', error);
                    }
                  }}
                  className="ml-auto px-2 py-1 bg-blue-500 text-white text-xs rounded hover:bg-blue-600"
                >
                  Chat
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Room Selection */}
      <div className="mb-4">
        <label className="block text-sm font-medium mb-2">Room ID (for testing)</label>
        <div className="flex gap-2">
          <input
            type="text"
            value={selectedRoom}
            onChange={(e) => setSelectedRoom(e.target.value)}
            placeholder="Enter room ID or create collaboration room"
            className="flex-1 px-3 py-2 border border-border-default rounded bg-background-default"
          />
          <button
            onClick={handleCreateCollaborationRoom}
            className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
          >
            Create Collab Room
          </button>
        </div>
      </div>

      {/* Message Input */}
      <div className="mb-4">
        <label className="block text-sm font-medium mb-2">Send Chat Message</label>
        <div className="flex gap-2">
          <input
            type="text"
            value={messageText}
            onChange={(e) => setMessageText(e.target.value)}
            placeholder="Type a message to other Goose instances..."
            className="flex-1 px-3 py-2 border border-border-default rounded bg-background-default"
            onKeyPress={(e) => e.key === 'Enter' && handleSendMessage()}
          />
          <button
            onClick={handleSendMessage}
            disabled={!messageText.trim() || !selectedRoom}
            className="px-4 py-2 bg-green-500 text-white rounded hover:bg-green-600 disabled:opacity-50"
          >
            Send
          </button>
        </div>
      </div>

      {/* Task Request */}
      <div className="mb-4">
        <label className="block text-sm font-medium mb-2">Send Task Request</label>
        <div className="flex gap-2">
          <input
            type="text"
            value={taskDescription}
            onChange={(e) => setTaskDescription(e.target.value)}
            placeholder="Describe a task for another Goose to help with..."
            className="flex-1 px-3 py-2 border border-border-default rounded bg-background-default"
          />
          <button
            onClick={handleSendTaskRequest}
            disabled={!taskDescription.trim() || !selectedRoom}
            className="px-4 py-2 bg-orange-500 text-white rounded hover:bg-orange-600 disabled:opacity-50"
          >
            Request Task
          </button>
        </div>
      </div>

      {/* Collaboration Invite */}
      <div className="mb-4">
        <label className="block text-sm font-medium mb-2">Send Collaboration Invite</label>
        <div className="flex gap-2">
          <input
            type="text"
            value={collaborationProject}
            onChange={(e) => setCollaborationProject(e.target.value)}
            placeholder="Describe a collaboration project..."
            className="flex-1 px-3 py-2 border border-border-default rounded bg-background-default"
          />
          <button
            onClick={handleSendCollaborationInvite}
            disabled={!collaborationProject.trim() || !selectedRoom}
            className="px-4 py-2 bg-purple-500 text-white rounded hover:bg-purple-600 disabled:opacity-50"
          >
            Invite
          </button>
        </div>
      </div>

      {/* Debug Section */}
      <div className="mb-6 p-4 bg-yellow-50 border border-yellow-200 rounded-lg">
        <h3 className="text-lg font-medium mb-2 text-yellow-800">🔍 Debug Tools</h3>
        <div className="flex gap-2 mb-2">
          <button
            onClick={async () => {
              if (!selectedRoom) {
                alert('Please select a room first');
                return;
              }
              try {
                await debugGooseMessage(selectedRoom);
                alert('Debug message sent! Check console logs and the other Goose instance.');
              } catch (error) {
                console.error('Debug message failed:', error);
                alert('Debug message failed: ' + error);
              }
            }}
            disabled={!selectedRoom}
            className="px-3 py-1 bg-yellow-500 text-white text-sm rounded hover:bg-yellow-600 disabled:opacity-50"
          >
            Send Debug Message
          </button>
          <button
            onClick={() => {
              const debugInfo = getDebugInfo();
              console.log('🔍 DEBUG INFO:', debugInfo);
              alert('Debug info logged to console. Check browser dev tools.');
            }}
            className="px-3 py-1 bg-yellow-500 text-white text-sm rounded hover:bg-yellow-600"
          >
            Log Debug Info
          </button>
        </div>
        <p className="text-xs text-yellow-700">
          Use these tools to test cross-user Goose communication. Send a debug message and check if it appears in other Goose instances.
        </p>
      </div>

      {/* Messages */}
      <div>
        <h3 className="text-lg font-medium mb-2">Recent Goose Messages ({messages.length})</h3>
        <div className="space-y-2 max-h-64 overflow-y-auto">
          {messages.length === 0 ? (
            <p className="text-text-muted text-sm">No Goose messages yet. Send a message to see it here!</p>
          ) : (
            messages.map((message, index) => {
              const isFromSelf = message.metadata?.isFromSelf;
              const senderName = getSenderDisplayName(message);
              
              return (
                <div 
                  key={index} 
                  className={`p-3 rounded border-l-4 ${
                    isFromSelf 
                      ? 'bg-blue-50 border-blue-400 ml-8' 
                      : 'bg-background-default border-green-400 mr-8'
                  }`}
                >
                  <div className="flex items-center justify-between mb-1">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium">{formatMessageType(message.type)}</span>
                      <span className={`text-xs px-2 py-1 rounded ${
                        isFromSelf 
                          ? 'bg-blue-100 text-blue-700' 
                          : 'bg-green-100 text-green-700'
                      }`}>
                        {senderName}
                      </span>
                    </div>
                    <span className="text-xs text-text-muted">
                      {message.timestamp.toLocaleTimeString()}
                    </span>
                  </div>
                  <p className="text-sm mb-1">{message.content}</p>
                  <div className="text-xs text-text-muted">
                    Room: {message.roomId}
                    {message.metadata?.taskId && ` | Task: ${message.metadata.taskId}`}
                    {message.metadata?.priority && ` | Priority: ${message.metadata.priority}`}
                    {message.metadata?.capabilities && ` | Capabilities: ${message.metadata.capabilities.join(', ')}`}
                  </div>
                  {message.type === 'goose.collaboration.invite' && !isFromSelf && (
                    <div className="mt-2 flex gap-2">
                      <button
                        onClick={() => handleAcceptCollaboration(message)}
                        className="px-3 py-1 bg-green-500 text-white text-xs rounded hover:bg-green-600"
                      >
                        Join AI Session
                      </button>
                      <button
                        onClick={() => {
                          console.log('Decline collaboration:', message.messageId);
                          // TODO: Send decline message back
                        }}
                        className="px-3 py-1 bg-red-500 text-white text-xs rounded hover:bg-red-600"
                      >
                        Decline
                      </button>
                    </div>
                  )}
                  {message.type === 'goose.task.request' && !isFromSelf && (
                    <div className="mt-2">
                      <button
                        onClick={() => {
                          // TODO: Implement task response functionality
                          console.log('Respond to task:', message.metadata?.taskId);
                        }}
                        className="px-3 py-1 bg-orange-500 text-white text-xs rounded hover:bg-orange-600"
                      >
                        Respond to Task
                      </button>
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
};

export default GooseChat;
