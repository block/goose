import React, { useState, useEffect, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ArrowLeft, Send, Users, MoreVertical } from 'lucide-react';
import { useMatrix } from '../contexts/MatrixContext';
import { MatrixUser } from '../services/MatrixService';
import AvatarImage from './AvatarImage';

interface MatrixChatProps {
  roomId: string;
  recipientId?: string;
  onBack?: () => void;
  className?: string;
  disableMessageHandling?: boolean; // Add prop to disable message handling when used alongside useSessionSharing
}

interface ChatMessage {
  id: string;
  content: string;
  sender: string;
  senderName: string;
  senderAvatar?: string;
  timestamp: Date;
  isFromSelf: boolean;
  isGooseMessage?: boolean;
}

const MatrixChat: React.FC<MatrixChatProps> = ({
  roomId,
  recipientId,
  onBack,
  className = '',
  disableMessageHandling = false,
}) => {
  const {
    currentUser,
    friends,
    sendMessage,
    onMessage,
    onGooseMessage,
    isConnected,
  } = useMatrix();

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [newMessage, setNewMessage] = useState('');
  const [isTyping, setIsTyping] = useState(false);
  const [recipient, setRecipient] = useState<MatrixUser | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Find recipient info
  useEffect(() => {
    if (recipientId) {
      const foundRecipient = friends.find(f => f.userId === recipientId);
      setRecipient(foundRecipient || {
        userId: recipientId,
        displayName: recipientId.split(':')[0].substring(1),
      });
    }
  }, [recipientId, friends]);

  // Listen for messages in this room
  useEffect(() => {
    if (!roomId || !currentUser || disableMessageHandling) {
      if (disableMessageHandling) {
        console.log('🚫 MatrixChat message handling disabled - useSessionSharing is handling messages');
      }
      return;
    }

    const handleRegularMessage = (messageData: any) => {
      const { content, sender, roomId: msgRoomId, timestamp, senderInfo } = messageData;
      
      // Only process messages for this room
      if (msgRoomId !== roomId) return;

      const chatMessage: ChatMessage = {
        id: `msg_${timestamp.getTime()}_${sender}`,
        content,
        sender,
        senderName: senderInfo?.displayName || sender.split(':')[0].substring(1),
        senderAvatar: senderInfo?.avatarUrl,
        timestamp: new Date(timestamp),
        isFromSelf: sender === currentUser.userId,
        isGooseMessage: false,
      };

      console.log('📱 MatrixChat received regular message:', chatMessage);

      setMessages(prev => {
        // Avoid duplicates
        const exists = prev.some(m => m.id === chatMessage.id);
        if (exists) return prev;
        
        // Insert in chronological order
        const newMessages = [...prev, chatMessage].sort((a, b) => 
          a.timestamp.getTime() - b.timestamp.getTime()
        );
        return newMessages;
      });
    };

    const handleGooseMessage = (gooseMessage: any) => {
      const { sender, content, roomId: msgRoomId, timestamp, metadata } = gooseMessage;
      
      // Only process messages for this room
      if (msgRoomId !== roomId) return;

      const chatMessage: ChatMessage = {
        id: `goose_${timestamp.getTime()}_${sender}`,
        content,
        sender,
        senderName: sender.split(':')[0].substring(1),
        senderAvatar: undefined, // Goose messages might not have avatar info
        timestamp: new Date(timestamp),
        isFromSelf: metadata?.isFromSelf || false,
        isGooseMessage: true,
      };

      console.log('🦆 MatrixChat received Goose message:', chatMessage);

      setMessages(prev => {
        // Avoid duplicates
        const exists = prev.some(m => m.id === chatMessage.id);
        if (exists) return prev;
        
        // Insert in chronological order
        const newMessages = [...prev, chatMessage].sort((a, b) => 
          a.timestamp.getTime() - b.timestamp.getTime()
        );
        return newMessages;
      });
    };

    const unsubscribeRegular = onMessage(handleRegularMessage);
    const unsubscribeGoose = onGooseMessage(handleGooseMessage);

    return () => {
      unsubscribeRegular();
      unsubscribeGoose();
    };
  }, [roomId, currentUser, onMessage, onGooseMessage, disableMessageHandling]);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Focus input on mount
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleSendMessage = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!newMessage.trim() || !roomId || !currentUser) return;

    const messageContent = newMessage.trim();
    setNewMessage('');
    setIsTyping(false);

    try {
      await sendMessage(roomId, messageContent);
      console.log('✅ Message sent successfully');
    } catch (error) {
      console.error('❌ Failed to send message:', error);
      // Optionally show error to user
    }
  };

  const formatTime = (timestamp: Date) => {
    const now = new Date();
    const isToday = timestamp.toDateString() === now.toDateString();
    
    if (isToday) {
      return timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    } else {
      return timestamp.toLocaleDateString([], { month: 'short', day: 'numeric' });
    }
  };

  const groupMessagesByDate = (messages: ChatMessage[]) => {
    const groups: { [key: string]: ChatMessage[] } = {};
    
    messages.forEach(message => {
      const dateKey = message.timestamp.toDateString();
      if (!groups[dateKey]) {
        groups[dateKey] = [];
      }
      groups[dateKey].push(message);
    });
    
    return groups;
  };

  const messageGroups = groupMessagesByDate(messages);

  if (!isConnected) {
    return (
      <div className={`flex flex-col h-full bg-background-default ${className}`}>
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <h3 className="text-lg font-medium text-text-default mb-2">Not Connected</h3>
            <p className="text-text-muted">Connect to Matrix to start chatting</p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={`flex flex-col h-full bg-background-default ${className}`}>
      {/* Header */}
      <div className="flex items-center gap-3 p-4 border-b border-border-default bg-background-default">
        {onBack && (
          <button
            onClick={onBack}
            className="p-2 rounded-lg hover:bg-background-medium transition-colors"
          >
            <ArrowLeft className="w-5 h-5" />
          </button>
        )}
        
        <div className="flex items-center gap-3 flex-1">
          <div className="w-10 h-10 bg-background-accent rounded-full flex items-center justify-center overflow-hidden">
            {recipient ? (
              <AvatarImage
                avatarUrl={recipient.avatarUrl}
                displayName={recipient.displayName}
                size="md"
              />
            ) : (
              <Users className="w-5 h-5 text-text-on-accent" />
            )}
          </div>
          
          <div className="flex-1 min-w-0">
            <h2 className="font-semibold text-text-default truncate">
              {recipient?.displayName || recipient?.userId || 'Matrix Chat'}
            </h2>
            {recipient && (
              <p className="text-sm text-text-muted truncate">
                {recipient.userId}
              </p>
            )}
          </div>
        </div>
        
        <button className="p-2 rounded-lg hover:bg-background-medium transition-colors">
          <MoreVertical className="w-5 h-5" />
        </button>
      </div>

      {/* Messages */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {Object.entries(messageGroups).map(([dateKey, dayMessages]) => (
          <div key={dateKey}>
            {/* Date separator */}
            <div className="flex items-center justify-center mb-4">
              <div className="px-3 py-1 bg-background-muted rounded-full">
                <span className="text-xs text-text-muted">
                  {new Date(dateKey).toLocaleDateString([], { 
                    weekday: 'long', 
                    month: 'long', 
                    day: 'numeric' 
                  })}
                </span>
              </div>
            </div>
            
            {/* Messages for this date */}
            {dayMessages.map((message, index) => {
              const prevMessage = index > 0 ? dayMessages[index - 1] : null;
              const isConsecutive = prevMessage && 
                prevMessage.sender === message.sender &&
                (message.timestamp.getTime() - prevMessage.timestamp.getTime()) < 5 * 60 * 1000; // 5 minutes
              
              return (
                <motion.div
                  key={message.id}
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  className={`flex gap-3 ${message.isFromSelf ? 'flex-row-reverse' : 'flex-row'} ${
                    isConsecutive ? 'mt-1' : 'mt-4'
                  }`}
                >
                  {/* Avatar */}
                  <div className={`w-8 h-8 ${isConsecutive ? 'invisible' : ''}`}>
                    {!message.isFromSelf && (
                      <div className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center overflow-hidden">
                        {message.isGooseMessage ? (
                          <span className="text-green-600 font-bold text-xs">🦆</span>
                        ) : (
                          <AvatarImage
                            avatarUrl={message.senderAvatar}
                            displayName={message.senderName}
                            size="sm"
                          />
                        )}
                      </div>
                    )}
                  </div>
                  
                  {/* Message bubble */}
                  <div className={`flex flex-col max-w-[70%] ${message.isFromSelf ? 'items-end' : 'items-start'}`}>
                    {!isConsecutive && (
                      <div className={`flex items-center gap-2 mb-1 ${message.isFromSelf ? 'flex-row-reverse' : 'flex-row'}`}>
                        <span className="text-xs font-medium text-text-default">
                          {message.isFromSelf ? 'You' : message.senderName}
                        </span>
                        {message.isGooseMessage && (
                          <span className="text-xs bg-green-100 text-green-700 px-2 py-0.5 rounded-full">
                            🦆 Goose
                          </span>
                        )}
                        <span className="text-xs text-text-muted">
                          {formatTime(message.timestamp)}
                        </span>
                      </div>
                    )}
                    
                    <div
                      className={`px-4 py-2 rounded-2xl max-w-full break-words ${
                        message.isFromSelf
                          ? 'bg-blue-500 text-white'
                          : message.isGooseMessage
                          ? 'bg-green-100 text-green-800 border border-green-200'
                          : 'bg-background-muted text-text-default'
                      }`}
                    >
                      {message.content}
                    </div>
                  </div>
                </motion.div>
              );
            })}
          </div>
        ))}
        
        {messages.length === 0 && (
          <div className="text-center py-12">
            <Users className="w-12 h-12 text-text-muted mx-auto mb-4" />
            <h3 className="text-lg font-medium text-text-default mb-2">Start the conversation</h3>
            <p className="text-text-muted">Send a message to begin chatting</p>
          </div>
        )}
        
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      <div className="p-4 border-t border-border-default bg-background-default">
        <form onSubmit={handleSendMessage} className="flex gap-3">
          <input
            ref={inputRef}
            type="text"
            value={newMessage}
            onChange={(e) => {
              setNewMessage(e.target.value);
              setIsTyping(e.target.value.length > 0);
            }}
            placeholder="Type a message..."
            className="flex-1 px-4 py-3 rounded-2xl border border-border-default bg-background-muted focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            disabled={!isConnected}
          />
          <button
            type="submit"
            disabled={!newMessage.trim() || !isConnected}
            className="px-6 py-3 rounded-2xl bg-blue-500 text-white hover:bg-blue-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-2"
          >
            <Send className="w-4 h-4" />
          </button>
        </form>
      </div>
    </div>
  );
};

export default MatrixChat;
