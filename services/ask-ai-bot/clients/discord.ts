import { Client, GatewayIntentBits } from "discord.js";

const client = new Client({
  /* Sensible defaults, you can add or remove intents as needed. */
  intents: [
    GatewayIntentBits.Guilds,
    GatewayIntentBits.GuildMessages,
    GatewayIntentBits.MessageContent,
  ],
});

export default client;
