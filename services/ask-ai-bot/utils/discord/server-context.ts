import type { Guild, TextChannel } from "discord.js";
import { ChannelType } from "discord.js";

export async function buildServerContext(guild: Guild): Promise<string> {
  try {
    const channels = await guild.channels.fetch();

    const textChannels = Array.from(channels.values())
      .filter(
        (ch): ch is TextChannel =>
          ch?.type === ChannelType.GuildText && ch !== null,
      )
      .sort((a, b) => (a.position ?? 0) - (b.position ?? 0));

    if (textChannels.length === 0) {
      return "";
    }

    const channelList = textChannels
      .map((ch) => `- #${ch.name}${ch.topic ? `: ${ch.topic}` : ""}`)
      .join("\n");

    return `## Server Channels
If a user asks about the server's channels or where to find something, here's the current channel list:
${channelList}`;
  } catch (error) {
    console.error("Error building server context:", error);
    return "";
  }
}
