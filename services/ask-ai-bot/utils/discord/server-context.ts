import type { Guild, TextChannel } from "discord.js";
import { ChannelType, PermissionFlagsBits } from "discord.js";

function isPublicChannel(ch: TextChannel, guild: Guild): boolean {
  // Check @everyone role permissions
  const everyoneOverwrite = ch.permissionOverwrites.cache.get(guild.id);

  // If @everyone is explicitly denied ViewChannel, the channel is private.
  if (everyoneOverwrite?.deny.has(PermissionFlagsBits.ViewChannel)) {
    return false;
  }

  // If the channel has any role/user overwrites that grant ViewChannel,
  // it's restricted to specific roles, treat as private.
  const hasRestrictiveOverwrites = ch.permissionOverwrites.cache.some(
    (overwrite) =>
      overwrite.id !== guild.id && // skip @everyone
      overwrite.allow.has(PermissionFlagsBits.ViewChannel),
  );
  if (hasRestrictiveOverwrites) {
    return false;
  }

  return true;
}

export async function buildServerContext(guild: Guild): Promise<string> {
  try {
    const channels = await guild.channels.fetch();

    const textChannels = Array.from(channels.values())
      .filter(
        (ch): ch is TextChannel =>
          ch?.type === ChannelType.GuildText &&
          ch !== null &&
          isPublicChannel(ch, guild),
      )
      .sort((a, b) => (a.position ?? 0) - (b.position ?? 0));

    if (textChannels.length === 0) {
      return "";
    }

    const channelList = textChannels
      .map(
        (ch) =>
          `- ID: ${ch.id}; Name: ${ch.name}; ${ch.topic ? `Topic: ${ch.topic}` : ""}`,
      )
      .join("\n");

    return `## Server Channels
If a user asks about the server's channels or where to find something, here's the current channel list:
${channelList}

When mentioning a channel, provide the link to the channel rather than using the plain text name. You can link to a channel by using the following format: \`<#channelId>\`.`;
  } catch (error) {
    console.error("Error building server context:", error);
    return "";
  }
}
