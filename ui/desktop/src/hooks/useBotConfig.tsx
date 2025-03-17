import { useState, useEffect } from 'react';
import { parseBotConfigFromUrl, setBotSystemPrompt, type BotConfig } from '../botConfig';
import { toast } from 'react-toastify';

export function useBotConfig() {
  const [botConfig, setBotConfig] = useState<BotConfig | null>(null);

  useEffect(() => {
    window.electron.logInfo('Setting up bot configuration listener');

    const handleConfigureBot = async (_: unknown, url: string) => {
      window.electron.logInfo('Bot configuration event received: ' + url);
      try {
        const config = parseBotConfigFromUrl(url);
        window.electron.logInfo('Parsed bot config: ' + JSON.stringify(config));

        if (!config) {
          window.electron.logInfo('Invalid bot configuration');
          toast.error('Invalid bot configuration');
          return;
        }

        // Set the system prompt
        window.electron.logInfo('Setting system prompt for bot: ' + config.name);
        const success = await setBotSystemPrompt(config.instructions);
        if (!success) {
          window.electron.logInfo('Failed to set system prompt');
          toast.error('Failed to configure bot');
          return;
        }

        // Store the bot configuration
        window.electron.logInfo(
          'Bot configuration successful, activities: ' + JSON.stringify(config.activities)
        );
        setBotConfig(config);
        toast.success(`${config.name} bot configured successfully`);
      } catch (error) {
        window.electron.logInfo('Error configuring bot: ' + error);
        toast.error('Failed to configure bot');
      }
    };

    // Make sure we're listening for the event
    window.electron.off('configure-bot', handleConfigureBot); // Remove any existing listeners
    window.electron.on('configure-bot', handleConfigureBot); // Add fresh listener

    // Signal to main process that we're ready to receive bot configuration
    window.electron.logInfo('Sending bot-ready event to main process');
    window.electron.botReady();

    return () => {
      window.electron.logInfo('Removing bot configuration listener');
      window.electron.off('configure-bot', handleConfigureBot);
    };
  }, []);

  return { botConfig };
}
