/**
 * Bot configuration interface
 */
export interface BotConfig {
  id: string;
  name: string;
  description: string;
  instructions: string;
  activities: string[] | null;
  outputExample?: string;
}
