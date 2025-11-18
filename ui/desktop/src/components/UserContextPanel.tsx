import React, { useState, useEffect } from 'react';
import { useUserContext } from '../hooks/useUserContext';
import { UserProfile } from '../services/UserContextService';

interface UserContextPanelProps {
  sessionId: string;
  className?: string;
}

export const UserContextPanel: React.FC<UserContextPanelProps> = ({ 
  sessionId, 
  className = '' 
}) => {
  const userContext = useUserContext();
  const [users, setUsers] = useState<UserProfile[]>([]);
  const [isExpanded, setIsExpanded] = useState(false);
  const [loading, setLoading] = useState(false);

  // Load users for this session
  useEffect(() => {
    const loadUsers = async () => {
      if (!userContext.isInitialized) return;
      
      setLoading(true);
      try {
        const introductions = await userContext.processIntroduction('', '', sessionId);
        const allUsers = await userContext.getAllUsers();
        
        // Filter users who have been in this session
        const sessionUsers = allUsers.filter(user => 
          user.collaborationHistory?.some(history => history.sessionId === sessionId)
        );
        
        setUsers(sessionUsers);
      } catch (error) {
        console.error('Failed to load session users:', error);
      } finally {
        setLoading(false);
      }
    };

    loadUsers();
  }, [sessionId, userContext]);

  if (!userContext.isInitialized || loading) {
    return null;
  }

  if (users.length === 0) {
    return null;
  }

  return (
    <div className={`bg-gray-50 dark:bg-gray-800 border-l border-gray-200 dark:border-gray-700 ${className}`}>
      <div className="p-4">
        <button
          onClick={() => setIsExpanded(!isExpanded)}
          className="flex items-center justify-between w-full text-left"
        >
          <h3 className="text-sm font-medium text-gray-900 dark:text-gray-100">
            User Context ({users.length})
          </h3>
          <svg
            className={`w-4 h-4 transition-transform ${isExpanded ? 'rotate-180' : ''}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </button>
        
        {isExpanded && (
          <div className="mt-3 space-y-3">
            {users.map((user) => (
              <UserCard key={user.userId} user={user} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

interface UserCardProps {
  user: UserProfile;
}

const UserCard: React.FC<UserCardProps> = ({ user }) => {
  const displayName = user.preferredName || user.displayName || user.userId;
  const lastSeen = user.lastSeen ? new Date(user.lastSeen).toLocaleDateString() : 'Never';
  
  return (
    <div className="bg-white dark:bg-gray-700 rounded-lg p-3 border border-gray-200 dark:border-gray-600">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <h4 className="text-sm font-medium text-gray-900 dark:text-gray-100">
            {displayName}
          </h4>
          
          {user.role && (
            <p className="text-xs text-gray-600 dark:text-gray-400 mt-1">
              {user.role}
            </p>
          )}
          
          {user.expertise && user.expertise.length > 0 && (
            <div className="mt-2">
              <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">Expertise:</p>
              <div className="flex flex-wrap gap-1">
                {user.expertise.slice(0, 3).map((skill, index) => (
                  <span
                    key={index}
                    className="inline-block px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900 text-blue-800 dark:text-blue-200 rounded"
                  >
                    {skill}
                  </span>
                ))}
                {user.expertise.length > 3 && (
                  <span className="text-xs text-gray-500 dark:text-gray-400">
                    +{user.expertise.length - 3} more
                  </span>
                )}
              </div>
            </div>
          )}
          
          {user.notes && (
            <p className="text-xs text-gray-600 dark:text-gray-400 mt-2 italic">
              "{user.notes}"
            </p>
          )}
          
          <p className="text-xs text-gray-500 dark:text-gray-400 mt-2">
            Last seen: {lastSeen}
          </p>
        </div>
        
        {user.avatarUrl && (
          <img
            src={user.avatarUrl}
            alt={displayName}
            className="w-8 h-8 rounded-full ml-3"
          />
        )}
      </div>
    </div>
  );
};

export default UserContextPanel;
