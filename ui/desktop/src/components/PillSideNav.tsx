import React, { useState, useRef, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { cn } from '../utils';
import { 
  Home as HomeIcon,
  MessageSquare as ChatIcon,
  Clock as ClockIcon,
  FileText as FileIcon,
  Puzzle as PuzzleIcon,
  Settings as SettingsIcon,
  History as HistoryIcon,
  ChevronRight as ChevronRightIcon,
  X as CloseIcon
} from 'lucide-react';

export const PillSideNav: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const currentPath = location.pathname;
  const [isExpanded, setIsExpanded] = useState(false);
  const navRef = useRef<HTMLDivElement>(null);

  const handleNavigation = (path: string) => {
    navigate(path);
    setIsExpanded(false);
  };

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (navRef.current && !navRef.current.contains(event.target as Node)) {
        setIsExpanded(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  // Navigation items configuration
  const navItems = [
    { icon: <HomeIcon size={18} />, label: 'Home', path: '/' },
    { icon: <ChatIcon size={18} />, label: 'Chat', path: '/pair' },
    { icon: <HistoryIcon size={18} />, label: 'History', path: '/sessions' },
    { icon: <FileIcon size={18} />, label: 'Recipes', path: '/recipes' },
    { icon: <SettingsIcon size={18} />, label: 'Settings', path: '/settings' },
  ];

  // Find the current active item
  const activeItem = navItems.find(item => item.path === currentPath) || navItems[0];

  // Create a reordered list with the active item first
  const orderedNavItems = [
    activeItem,
    ...navItems.filter(item => item.path !== activeItem.path)
  ];
  
  return (
    <div className="relative z-50" ref={navRef}>
      {/* All Pills (including the active one) */}
      <div className="relative">
        {orderedNavItems.map((item, index) => {
          const isActive = item.path === currentPath;
          const showPill = isExpanded || isActive;
          
          // Calculate position - first pill is at top (0), others follow
          const verticalOffset = index * 42; // Height of pill + small gap
          
          return (
            <div
              key={item.path}
              className={cn(
                "absolute left-1/2 transform -translate-x-1/2",
                "bg-black/70 backdrop-blur-xl rounded-full shadow-lg border border-white/20",
                "transition-all duration-300 ease-in-out cursor-pointer",
                "hover:bg-black/80",
                showPill 
                  ? "opacity-100" 
                  : "opacity-0 pointer-events-none"
              )}
              style={{
                top: showPill ? `${verticalOffset}px` : 0,
                zIndex: 50 - index,
                transitionDelay: isExpanded ? `${index * 50}ms` : '0ms'
              }}
              onClick={() => {
                if (isActive) {
                  setIsExpanded(!isExpanded);
                } else {
                  handleNavigation(item.path);
                }
              }}
            >
              <div className="flex items-center gap-2 px-4 py-2 whitespace-nowrap">
                <div className="text-white">{item.icon}</div>
                <span className="text-white text-sm font-medium">{item.label}</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};

export default PillSideNav;
