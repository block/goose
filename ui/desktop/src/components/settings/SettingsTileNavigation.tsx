import React from 'react';
import { motion } from 'framer-motion';
import { Bot, MessageSquare, Share2, Monitor, Settings, Palette, Users, Cog } from 'lucide-react';

interface SettingsTile {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  description?: string;
  getTag?: () => string;
  tagColor?: 'default' | 'success' | 'warning' | 'info';
}

interface SettingsTileNavigationProps {
  activeSection: string;
  onSectionChange: (section: string) => void;
}

export const SettingsTileNavigation: React.FC<SettingsTileNavigationProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const settingsTiles: SettingsTile[] = [
    {
      id: 'models',
      label: 'Models',
      icon: Bot,
      description: 'AI models, providers & lead worker',
      getTag: () => 'AI',
      tagColor: 'info',
    },
    {
      id: 'chat',
      label: 'Chat',
      icon: MessageSquare,
      description: 'Interface, modes & behavior',
      getTag: () => 'UI',
      tagColor: 'success',
    },
    {
      id: 'sharing',
      label: 'Session',
      icon: Share2,
      description: 'Sharing & collaboration settings',
      getTag: () => 'SYNC',
      tagColor: 'warning',
    },
    {
      id: 'app',
      label: 'App',
      icon: Monitor,
      description: 'System preferences & updates',
      getTag: () => 'SYS',
      tagColor: 'default',
    },
  ];

  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 md:grid-cols-4 lg:grid-cols-4 xl:grid-cols-4 gap-0.5">
        {settingsTiles.map((tile, index) => {
          const IconComponent = tile.icon;
          const isActive = activeSection === tile.id;

          return (
            <motion.button
              key={tile.id}
              onClick={() => onSectionChange(tile.id)}
              initial={{ opacity: 0, y: 20, scale: 0.9 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              transition={{
                type: "spring",
                stiffness: 350,
                damping: 25,
                delay: index * 0.03, // 30ms stagger like TopNavigation
              }}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              className={`
                w-full relative flex flex-col items-start justify-end
                rounded-2xl
                px-6 py-6
                h-[140px]
                transition-colors duration-200
                ${isActive 
                  ? 'bg-background-accent text-text-on-accent' 
                  : 'bg-background-default hover:bg-background-medium'
                }
              `}
            >
              {/* Spacer above icon */}
              <div className="h-[40px]"></div>
              
              {/* Icon */}
              <IconComponent className="w-6 h-6 mb-2 flex-shrink-0" />
              
              {/* Label and Description */}
              <div className="flex flex-col">
                <h2 className="text-lg font-light text-left">{tile.label}</h2>
                {tile.description && (
                  <p className={`text-xs text-left leading-tight mt-0.5 ${
                    isActive ? 'text-text-on-accent/80' : 'text-text-muted'
                  }`}>
                    {tile.description}
                  </p>
                )}
              </div>
            </motion.button>
          );
        })}
    </div>
  );
};

export default SettingsTileNavigation;
