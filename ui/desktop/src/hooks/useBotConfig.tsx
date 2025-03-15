import { useState, useEffect } from 'react';
import { parseBotConfigFromUrl, setBotSystemPrompt, type BotConfig } from '../botConfig';
import { toast } from 'react-toastify';

export function useBotConfig() {
  const [botConfig, setBotConfig] = useState<BotConfig | null>(null);

  useEffect(() => {
    const handleConfigureBot = async (_: unknown, url: string) => {
      try {
        const config = parseBotConfigFromUrl(url);
        if (!config) {
          toast.error('Invalid bot configuration');
          return;
        }

        // Set the system prompt
        const success = await setBotSystemPrompt(config.instructions);
        if (!success) {
          toast.error('Failed to configure bot');
          return;
        }

        // Store the bot configuration
        setBotConfig(config);
        toast.success(`${config.name} bot configured successfully`);
      } catch (error) {
        console.error('Error configuring bot:', error);
        toast.error('Failed to configure bot');
      }
    };

    window.electron.on('configure-bot', handleConfigureBot);
    return () => {
      window.electron.off('configure-bot', handleConfigureBot);
    };
  }, []);

  return { botConfig };
}
