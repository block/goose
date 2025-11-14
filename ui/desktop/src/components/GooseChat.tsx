import React, { useState, useEffect } from 'react';
import { useMatrix } from '../contexts/MatrixContext';
import { GooseChatMessage } from '../services/MatrixService';

const GooseChat: React.FC = () => {
  const { 
    isConnected, 
    gooseInstances, 
    sendGooseMessage, 
    sendTaskRequest,
    sendCollaborationInvite,
    acceptCollaborationInvite,
    createGooseCollaborationRoom,
    announceCapabilities,
    onGooseMessage 
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

  const handleAcceptCollaboration = async (messageId: string) => {
    if (!selectedRoom) return;

    try {
      await acceptCollaborationInvite(selectedRoom, messageId, ['coding', 'analysis']);
    } catch (error) {
      console.error('Failed to accept collaboration:', error);
    }
  };

  const formatMessageType = (type: string) => {
    switch (type) {
      case 'goose.chat': return '💬 Chat';
      case 'goose.task.request': return '📋 Task Request';
      case 'goose.task.response': return '✅ Task Response';
      case 'goose.collaboration.invite': return '🤝 Collaboration Invite';
      case 'goose.collaboration.accept': return '✅ Collaboration Accepted';
      case 'goose.collaboration.decline': return '❌ Collaboration Declined';
      default: return '🦆 Goose Message';
    }
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
      <div className="mb-6">
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

      {/* Messages */}
      <div>
        <h3 className="text-lg font-medium mb-2">Recent Goose Messages ({messages.length})</h3>
        <div className="space-y-2 max-h-64 overflow-y-auto">
          {messages.length === 0 ? (
            <p className="text-text-muted text-sm">No Goose messages yet. Send a message to see it here!</p>
          ) : (
            messages.map((message, index) => (
              <div key={index} className="p-3 bg-background-default rounded border-l-4 border-blue-400">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm font-medium">{formatMessageType(message.type)}</span>
                  <span className="text-xs text-text-muted">
                    {message.timestamp.toLocaleTimeString()}
                  </span>
                </div>
                <p className="text-sm mb-1">{message.content}</p>
                <div className="text-xs text-text-muted">
                  From: {message.sender} | Room: {message.roomId}
                  {message.metadata?.taskId && ` | Task: ${message.metadata.taskId}`}
                  {message.metadata?.priority && ` | Priority: ${message.metadata.priority}`}
                </div>
                {message.type === 'goose.collaboration.invite' && (
                  <button
                    onClick={() => handleAcceptCollaboration(message.messageId)}
                    className="mt-2 px-3 py-1 bg-green-500 text-white text-xs rounded hover:bg-green-600"
                  >
                    Accept Collaboration
                  </button>
                )}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
};

export default GooseChat;
