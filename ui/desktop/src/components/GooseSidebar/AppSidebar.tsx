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
import { ViewOptions, View } from '../../App';

interface SidebarProps {
  onSelectSession: (sessionId: string) => void;
  refreshTrigger?: number;
  children?: React.ReactNode;
  setIsGoosehintsModalOpen?: (isOpen: boolean) => void;
  setView?: (view: View, viewOptions?: ViewOptions) => void;
  currentPath?: string;
}

// Main Sidebar Component
const AppSidebar: React.FC<SidebarProps> = ({ currentPath }) => {
  const navigate = useNavigate();

  useEffect(() => {
    // Trigger animation after a small delay
    const timer = setTimeout(() => {
      // setIsVisible(true);
    }, 100);
    // eslint-disable-next-line no-undef
    return () => clearTimeout(timer);
  }, []);

  // Helper function to check if a path is active
  const isActivePath = (path: string) => {
    return currentPath === path;
  };

  return (
    <>
      <SidebarContent className="pt-16">
        {/* Menu */}
        <SidebarMenu>
          {/* Navigation Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => {
                      navigate('/');
                    }}
                    isActive={isActivePath('/')}
                    tooltip="Go back to the main chat screen"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Home className="w-4 h-4" />
                    <span>Home</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Chat & Configuration Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/pair')}
                    isActive={isActivePath('/pair')}
                    tooltip="Start pairing with Goose"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <ChatSmart className="w-4 h-4" />
                    <span>Chat</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>

              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/sessions')}
                    isActive={isActivePath('/sessions')}
                    tooltip="View your session history"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <History className="w-4 h-4" />
                    <span>History</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Content Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              {/*<div className="sidebar-item">*/}
              {/*  <SidebarMenuItem>*/}
              {/*    <SidebarMenuButton*/}
              {/*      onClick={() => navigate('/projects')}*/}
              {/*      isActive={isActivePath('/projects')}*/}
              {/*      tooltip="Manage your projects"*/}
              {/*      className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"*/}
              {/*    >*/}
              {/*      <FolderKanban className="w-4 h-4" />*/}
              {/*      <span>Projects</span>*/}
              {/*    </SidebarMenuButton>*/}
              {/*  </SidebarMenuItem>*/}
              {/*</div>*/}

              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/recipes')}
                    isActive={isActivePath('/recipes')}
                    tooltip="Browse your saved recipes"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <FileText className="w-4 h-4" />
                    <span>Recipes</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>

              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/schedules')}
                    isActive={isActivePath('/schedules')}
                    tooltip="Manage scheduled runs"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Clock className="w-4 h-4" />
                    <span>Scheduler</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>

              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/extensions')}
                    isActive={isActivePath('/extensions')}
                    tooltip="Manage your extensions"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Puzzle className="w-4 h-4" />
                    <span>Extensions</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>

          <SidebarSeparator />

          {/* Settings Group */}
          <SidebarGroup>
            <SidebarGroupContent className="space-y-1">
              <div className="sidebar-item">
                <SidebarMenuItem>
                  <SidebarMenuButton
                    onClick={() => navigate('/settings')}
                    isActive={isActivePath('/settings')}
                    tooltip="Configure Goose settings"
                    className="w-full justify-start px-3 rounded-lg h-fit hover:bg-background-medium/50 transition-all duration-200 data-[active=true]:bg-background-medium"
                  >
                    <Gear className="w-4 h-4" />
                    <span>Settings</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </div>
            </SidebarGroupContent>
          </SidebarGroup>
        </SidebarMenu>
      </SidebarContent>

      <SidebarFooter />
    </>
  );
};

export default AppSidebar;
