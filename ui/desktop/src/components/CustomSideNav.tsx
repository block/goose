import React from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { cn } from '../utils';
import { 
  House, 
  ChatCenteredText, 
  Clock, 
  FileText, 
  Puzzle, 
  Gear,
  CaretRight,
  History
} from 'phosphor-react';

interface SideNavItemProps {
  icon: React.ReactNode;
  label: string;
  path: string;
  isActive?: boolean;
  onClick?: () => void;
}

const SideNavItem: React.FC<SideNavItemProps> = ({ 
  icon, 
  label, 
  path, 
  isActive = false,
  onClick
}) => {
  return (
    <li className="mb-1">
      <a 
        href={path}
        onClick={(e) => {
          e.preventDefault();
          if (onClick) onClick();
        }}
        className={cn(
          "flex items-center gap-3 px-3 py-2 rounded-lg transition-all duration-200",
          isActive 
            ? "bg-white/10 text-white" 
            : "text-white/70 hover:text-white hover:bg-white/5"
        )}
      >
        <div className="text-lg">
          {icon}
        </div>
        <span className="text-sm font-medium">{label}</span>
        {isActive && (
          <CaretRight weight="bold" className="ml-auto text-white/70" size={16} />
        )}
      </a>
    </li>
  );
};

interface SideNavSectionProps {
  title?: string;
  children: React.ReactNode;
}

const SideNavSection: React.FC<SideNavSectionProps> = ({ title, children }) => {
  return (
    <div className="mb-6">
      {title && (
        <h3 className="text-xs font-medium text-white/50 uppercase tracking-wider px-3 mb-2">
          {title}
        </h3>
      )}
      <ul>{children}</ul>
    </div>
  );
};

const SideNavDivider: React.FC = () => {
  return <div className="h-px bg-white/10 my-4 mx-3" />;
};

export const CustomSideNav: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const currentPath = location.pathname;

  const handleNavigation = (path: string) => {
    navigate(path);
  };

  return (
    <div className="h-full w-64 bg-gradient-to-b from-gray-800/90 to-gray-900/90 backdrop-blur-lg border-r border-white/10 flex flex-col">
      {/* Logo Area */}
      <div className="flex items-center px-4 py-6">
        <div className="w-8 h-8 bg-white rounded-full flex items-center justify-center">
          <span className="text-gray-900 font-bold text-lg">G</span>
        </div>
        <span className="ml-2 text-white font-medium text-lg">Goose</span>
      </div>

      {/* Navigation */}
      <div className="flex-1 overflow-y-auto px-3 py-2">
        <SideNavSection>
          <SideNavItem 
            icon={<House weight="fill" />} 
            label="Home" 
            path="/"
            isActive={currentPath === '/'}
            onClick={() => handleNavigation('/')}
          />
          <SideNavItem 
            icon={<ChatCenteredText weight="fill" />} 
            label="Chat" 
            path="/pair"
            isActive={currentPath === '/pair'}
            onClick={() => handleNavigation('/pair')}
          />
        </SideNavSection>

        <SideNavDivider />

        <SideNavSection title="History">
          <SideNavItem 
            icon={<History weight="fill" />} 
            label="Sessions" 
            path="/sessions"
            isActive={currentPath === '/sessions'}
            onClick={() => handleNavigation('/sessions')}
          />
          <SideNavItem 
            icon={<Clock weight="fill" />} 
            label="Scheduler" 
            path="/schedules"
            isActive={currentPath === '/schedules'}
            onClick={() => handleNavigation('/schedules')}
          />
        </SideNavSection>

        <SideNavDivider />

        <SideNavSection title="Tools">
          <SideNavItem 
            icon={<FileText weight="fill" />} 
            label="Recipes" 
            path="/recipes"
            isActive={currentPath === '/recipes'}
            onClick={() => handleNavigation('/recipes')}
          />
          <SideNavItem 
            icon={<Puzzle weight="fill" />} 
            label="Extensions" 
            path="/extensions"
            isActive={currentPath === '/extensions'}
            onClick={() => handleNavigation('/extensions')}
          />
        </SideNavSection>
      </div>

      {/* Footer */}
      <div className="p-3 border-t border-white/10">
        <SideNavItem 
          icon={<Gear weight="fill" />} 
          label="Settings" 
          path="/settings"
          isActive={currentPath === '/settings'}
          onClick={() => handleNavigation('/settings')}
        />
      </div>
    </div>
  );
};

export default CustomSideNav;
