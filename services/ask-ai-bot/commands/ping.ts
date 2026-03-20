import { ChatInputCommandInteraction, SlashCommandBuilder } from "discord.js";
import packageJson from "../package.json";

export default {
  data: new SlashCommandBuilder().setName("ping").setDescription("Ping!"),

  async execute(data: { interaction: ChatInputCommandInteraction }) {
    const interaction = data.interaction;
    await interaction.reply(`Bot is online, version ${packageJson.version}.`);
  },
};
