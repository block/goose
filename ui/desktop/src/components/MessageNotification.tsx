import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { MessageCircle, X, User, Clock } from 'lucide-react';
import { useMatrix } from '../contexts/MatrixContext';
import { useLocation } from 'react-router-dom';
import AvatarImage from './AvatarImage';
import { useActiveSession } from '../hooks/useActiveSession';

interface MessageNotificationData {
  id: string;
  senderId: string;
  senderName: string;
  senderAvatar?: string;
  content: string;
  roomId: string;
  timestamp: Date;
  isGooseMessage?: boolean;
}

interface MessageNotificationProps {
  className?: string;
  onOpenChat?: (roomId: string, senderId: string) => void;
}

const MessageNotification: React.FC<MessageNotificationProps> = ({
  className = '',
  onOpenChat,
}) => {
  const { 
    isConnected, 
    onMessage,
    onGooseMessage,
    currentUser,
  } = useMatrix();
  
  const location = useLocation();
  const { shouldSuppressNotification } = useActiveSession();
  
  const [notifications, setNotifications] = useState<MessageNotificationData[]>([]);
  const [dismissedIds, setDismissedIds] = useState<Set<string>>(new Set());

  // Listen for incoming messages
  useEffect(() => {
    if (!isConnected || !currentUser) return;

    // Listen for regular Matrix messages
    const unsubscribeRegular = onMessage((messageData: any) => {
      const { content, sender, roomId, timestamp, senderInfo } = messageData;
      
      // Only show notifications for messages from others
      if (sender === currentUser.userId) return;
      
      // Use the enhanced notification suppression logic
      if (shouldSuppressNotification(roomId, sender)) {
        console.log('🔕 Suppressing message notification for active session:', roomId);
        return;
      }
      
      // Filter out very old messages (more than 2 minutes old) to prevent spam on startup
      const messageAge = Date.now() - new Date(timestamp).getTime();
      const twoMinutesInMs = 2 * 60 * 1000;
      
      if (messageAge > twoMinutesInMs) {
        console.log('📱 Ignoring old message for notification:', messageAge / 1000, 'seconds old');
        return;
      }

      const notificationId = `msg_${roomId}_${sender}_${timestamp.getTime()}`;
      
      // Don't show if already dismissed
      if (dismissedIds.has(notificationId)) return;

      const notification: MessageNotificationData = {
        id: notificationId,
        senderId: sender,
        senderName: senderInfo?.displayName || sender.split(':')[0].substring(1),
        senderAvatar: senderInfo?.avatarUrl,
        content: content,
        roomId: roomId,
        timestamp: new Date(timestamp),
        isGooseMessage: false,
      };

      console.log('📱 New message notification:', notification);
      
      setNotifications(prev => {
        // Remove any existing notification from the same sender in the same room
        const filtered = prev.filter(n => !(n.senderId === sender && n.roomId === roomId));
        return [...filtered, notification];
      });
    });

    // Listen for Goose messages
    const unsubscribeGoose = onGooseMessage((gooseMessage: any) => {
      const { sender, content, roomId, timestamp, metadata } = gooseMessage;
      
      // Only show notifications for messages from others
      if (metadata?.isFromSelf) return;
      
      // Use the enhanced notification suppression logic
      if (shouldSuppressNotification(roomId, sender)) {
        console.log('🔕 Suppressing Goose message notification for active session:', roomId);
        return;
      }
      
      // Filter out old messages
      const messageAge = Date.now() - new Date(timestamp).getTime();
      const twoMinutesInMs = 2 * 60 * 1000;
      
      if (messageAge > twoMinutesInMs) {
        console.log('🦆 Ignoring old Goose message for notification:', messageAge / 1000, 'seconds old');
        return;
      }

      const notificationId = `goose_${roomId}_${sender}_${timestamp.getTime()}`;
      
      // Don't show if already dismissed
      if (dismissedIds.has(notificationId)) return;

      const notification: MessageNotificationData = {
        id: notificationId,
        senderId: sender,
        senderName: sender.split(':')[0].substring(1), // Extract username from Matrix ID
        senderAvatar: undefined, // Goose messages might not have avatar info
        content: content,
        roomId: roomId,
        timestamp: new Date(timestamp),
        isGooseMessage: true,
      };

      console.log('🦆 New Goose message notification:', notification);
      
      setNotifications(prev => {
        // Remove any existing notification from the same sender in the same room
        const filtered = prev.filter(n => !(n.senderId === sender && n.roomId === roomId));
        return [...filtered, notification];
      });
    });

    return () => {
      unsubscribeRegular();
      unsubscribeGoose();
    };
  }, [isConnected, currentUser, onMessage, onGooseMessage, dismissedIds]);

  // Auto-dismiss notifications after 10 seconds
  useEffect(() => {
    notifications.forEach(notification => {
      const timer = setTimeout(() => {
        handleDismiss(notification.id);
      }, 10000); // 10 seconds

      return () => clearTimeout(timer);
    });
  }, [notifications]);

  const handleDismiss = (notificationId: string) => {
    setDismissedIds(prev => new Set([...prev, notificationId]));
    setNotifications(prev => prev.filter(n => n.id !== notificationId));
  };

  const handleOpenChat = (notification: MessageNotificationData) => {
    console.log('📱 Opening chat for notification:', notification);
    
    // Dismiss the notification
    handleDismiss(notification.id);
    
    // Call the onOpenChat callback if provided
    onOpenChat?.(notification.roomId, notification.senderId);
  };

  const formatMessagePreview = (content: string, maxLength: number = 100) => {
    if (content.length <= maxLength) return content;
    return content.substring(0, maxLength) + '...';
  };

  const getTimeAgo = (timestamp: Date) => {
    const now = new Date();
    const diffInSeconds = Math.floor((now.getTime() - timestamp.getTime()) / 1000);
    
    if (diffInSeconds < 60) return 'just now';
    if (diffInSeconds < 3600) return `${Math.floor(diffInSeconds / 60)}m ago`;
    if (diffInSeconds < 86400) return `${Math.floor(diffInSeconds / 3600)}h ago`;
    return `${Math.floor(diffInSeconds / 86400)}d ago`;
  };

  if (!isConnected || notifications.length === 0) {
    return null;
  }

  return (
    <div className={`fixed top-4 right-4 z-50 space-y-2 ${className}`}>
      <AnimatePresence>
        {notifications.map((notification) => (
          <motion.div
            key={notification.id}
            initial={{ opacity: 0, x: 300, scale: 0.8 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: 300, scale: 0.8 }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
            className="bg-background-default border border-border-default rounded-lg shadow-lg p-4 max-w-sm min-w-[320px]"
          >
            <div className="flex items-start gap-3">
              <div className="flex-shrink-0 w-10 h-10 bg-background-accent rounded-full flex items-center justify-center">
                {notification.senderAvatar ? (
                  <AvatarImage
                    avatarUrl={notification.senderAvatar}
                    displayName={notification.senderName}
                    size="md"
                    className="ring-1 ring-border-default"
                  />
                ) : notification.isGooseMessage ? (
                  <div className="w-8 h-8 bg-background-accent rounded-full flex items-center justify-center">
                    <span className="text-text-on-accent font-bold text-sm">🦆</span>
                  </div>
                ) : (
                  <User className="w-5 h-5 text-text-on-accent" />
                )}
              </div>
              
              <div className="flex-1 min-w-0">
                <div className="flex items-center justify-between mb-1">
                  <h4 className="text-sm font-semibold text-text-default truncate">
                    {notification.isGooseMessage ? '🦆 ' : '💬 '}
                    {notification.senderName}
                  </h4>
                  <button
                    onClick={() => handleDismiss(notification.id)}
                    className="text-text-muted hover:text-text-default transition-colors"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
                
                <p className="text-sm text-text-default mb-3 line-clamp-3">
                  {formatMessagePreview(notification.content)}
                </p>
                
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1 text-xs text-text-muted">
                    <Clock className="w-3 h-3" />
                    {getTimeAgo(notification.timestamp)}
                  </div>
                  
                  <button
                    onClick={() => handleOpenChat(notification)}
                    className="flex items-center gap-1 px-3 py-1.5 bg-background-accent text-text-on-accent text-xs rounded hover:bg-background-accent/80 transition-colors"
                  >
                    <MessageCircle className="w-3 h-3" />
                    Open Chat
                  </button>
                </div>
              </div>
            </div>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
};

export default MessageNotification;
