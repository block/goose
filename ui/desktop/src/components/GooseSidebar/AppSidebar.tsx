import React, { useEffect } from 'react';
import { FileText, Clock, Home, Puzzle, History } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import {
  SidebarContent,
  SidebarFooter,
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarGroup,
  SidebarGroupContent,
  SidebarSeparator,
} from '../ui/sidebar';
import { ChatSmart, Gear } from '../icons';
import { ViewOptions, View } from '../../utils/navigationUtils';
import { useChatContext } from '../../contexts/ChatContext';
import { DEFAULT_CHAT_TITLE } from '../../contexts/ChatContext';
import EnvironmentBadge from './EnvironmentBadge';
import { useCounsel } from '../../contexts/CounselContext';

interface SidebarProps {
  onSelectSession: (sessionId: string) => void;
  refreshTrigger?: number;
  children?: React.ReactNode;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  setView?: (view: View, viewOptions?: ViewOptions) => void;
  currentPath?: string;
}

interface NavigationItem {
  type: 'item';
  path: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  tooltip: string;
}

interface NavigationSeparator {
  type: 'separator';
}

type NavigationEntry = NavigationItem | NavigationSeparator;

const menuItems: NavigationEntry[] = [
  {
    type: 'item',
    path: '/',
    label: 'Home',
    icon: Home,
    tooltip: 'Go back to the main chat screen',
  },
  { type: 'separator' },
  {
    type: 'item',
    path: '/pair',
    label: 'Chat',
    icon: ChatSmart,
    tooltip: 'Start pairing with Goose',
  },
  {
    type: 'item',
    path: '/sessions',
    label: 'History',
    icon: History,
    tooltip: 'View your session history',
  },
  { type: 'separator' },
  {
    type: 'item',
    path: '/recipes',
    label: 'Recipes',
    icon: FileText,
    tooltip: 'Browse your saved recipes',
  },
  {
    type: 'item',
    path: '/schedules',
    label: 'Scheduler',
    icon: Clock,
    tooltip: 'Manage scheduled runs',
  },
  {
    type: 'item',
    path: '/extensions',
    label: 'Extensions',
    icon: Puzzle,
    tooltip: 'Manage your extensions',
  },
  { type: 'separator' },
  {
    type: 'item',
    path: '/settings',
    label: 'Settings',
    icon: Gear,
    tooltip: 'Configure Goose settings',
  },
];

const AppSidebar: React.FC<SidebarProps> = ({ currentPath }) => {
  const navigate = useNavigate();
  const chatContext = useChatContext();
  const { openCounselModal } = useCounsel();

  useEffect(() => {
    const timer = setTimeout(() => {
      // setIsVisible(true);
    }, 100);

    return () => clearTimeout(timer);
  }, []);

  useEffect(() => {
    const currentItem = menuItems.find(
      (item) => item.type === 'item' && item.path === currentPath
    ) as NavigationItem | undefined;

    const titleBits = ['Goose'];

    if (
      currentPath === '/pair' &&
      chatContext?.chat?.name &&
      chatContext.chat.name !== DEFAULT_CHAT_TITLE
    ) {
      titleBits.push(chatContext.chat.name);
    } else if (currentPath !== '/' && currentItem) {
      titleBits.push(currentItem.label);
    }

    document.title = titleBits.join(' - ');
  }, [currentPath, chatContext?.chat?.name]);

  const isActivePath = (path: string) => {
    return currentPath === path;
  };

  const renderMenuItem = (entry: NavigationEntry, index: number) => {
    if (entry.type === 'separator') {
      return <SidebarSeparator key={index} />;
    }

    const IconComponent = entry.icon;
    const isChatItem = entry.path === '/pair';

    return (
      <React.Fragment key={entry.path}>
        <SidebarGroup>
          <SidebarGroupContent className="space-y-1">
            <div className="sidebar-item">
              <SidebarMenuItem>
                <SidebarMenuButton
                  data-testid={`sidebar-${entry.label.toLowerCase()}-button`}
                  onClick={() => navigate(entry.path)}
                  isActive={isActivePath(entry.path)}
                  tooltip={entry.tooltip}
                  className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                >
                  <IconComponent className="w-4 h-4" />
                  <span>{entry.label}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </div>
          </SidebarGroupContent>
        </SidebarGroup>

        {/* Add Counsel button after Chat item */}
        {isChatItem && (
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    data-testid="sidebar-counsel-button"
                    onClick={openCounselModal}
                    tooltip="Get opinions from the Counsel of 9"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200"
                  >
                    <span className="w-4 h-4 flex items-center justify-center text-sm">🎭</span>
                    <span>Counsel of 9</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>
        )}
      </React.Fragment>
    );
  };

  return (
    <>
      <SidebarContent className="pt-16">
        <SidebarMenu>{menuItems.map((entry, index) => renderMenuItem(entry, index))}</SidebarMenu>
      </SidebarContent>

      <SidebarFooter className="pb-2 flex items-start">
        <EnvironmentBadge />
      </SidebarFooter>
    </>
  );
};

export default AppSidebar;
